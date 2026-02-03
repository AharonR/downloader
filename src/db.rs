//! Database connection and schema management.
//!
//! This module provides SQLite database connectivity with:
//! - Connection pool management
//! - WAL mode for concurrent reads
//! - Automatic migration execution
//!
//! # Example
//!
//! ```no_run
//! use downloader_core::Database;
//! use std::path::Path;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let db = Database::new(Path::new("downloads.db")).await?;
//! // Use db for queries...
//! # Ok(())
//! # }
//! ```

use std::path::Path;

use sqlx::sqlite::{SqlitePool, SqlitePoolOptions};
use thiserror::Error;
use tracing::instrument;

/// Default maximum number of connections in the pool.
/// Kept low for SQLite since it uses file-level locking.
const DEFAULT_MAX_CONNECTIONS: u32 = 5;

/// SQLite busy timeout in milliseconds.
/// Connections will wait this long before returning SQLITE_BUSY.
const BUSY_TIMEOUT_MS: u32 = 5000;

/// Database-related errors.
#[derive(Error, Debug)]
pub enum DbError {
    /// Failed to connect to the database.
    #[error("failed to connect to database: {0}")]
    Connection(#[from] sqlx::Error),

    /// Failed to run migrations.
    #[error("failed to run migrations: {0}")]
    Migration(#[from] sqlx::migrate::MigrateError),
}

/// Database connection wrapper with connection pool.
///
/// Handles SQLite connection pooling, WAL mode configuration,
/// and automatic migration execution.
#[derive(Debug, Clone)]
pub struct Database {
    pool: SqlitePool,
}

impl Database {
    /// Creates a new database connection to the specified path.
    ///
    /// This will:
    /// 1. Create the database file if it doesn't exist
    /// 2. Enable WAL mode for concurrent reads
    /// 3. Run any pending migrations
    ///
    /// # Arguments
    ///
    /// * `db_path` - Path to the SQLite database file
    ///
    /// # Errors
    ///
    /// Returns `DbError::Connection` if the connection fails,
    /// or `DbError::Migration` if migrations fail.
    #[instrument(skip(db_path), fields(path = %db_path.display()))]
    pub async fn new(db_path: &Path) -> Result<Self, DbError> {
        let db_url = format!("sqlite:{}?mode=rwc", db_path.display());

        let pool = SqlitePoolOptions::new()
            .max_connections(DEFAULT_MAX_CONNECTIONS)
            .connect(&db_url)
            .await?;

        // Enable WAL mode for concurrent reads
        sqlx::query("PRAGMA journal_mode=WAL")
            .execute(&pool)
            .await?;

        // Set busy timeout to avoid immediate lock errors
        sqlx::query(&format!("PRAGMA busy_timeout={BUSY_TIMEOUT_MS}"))
            .execute(&pool)
            .await?;

        // Run migrations
        sqlx::migrate!("./migrations").run(&pool).await?;

        Ok(Self { pool })
    }

    /// Creates an in-memory database for testing.
    ///
    /// The database exists only for the lifetime of the connection
    /// and is useful for unit tests. Note: WAL mode is not enabled
    /// for in-memory databases as it provides no benefit.
    ///
    /// # Errors
    ///
    /// Returns `DbError::Connection` if the connection fails,
    /// or `DbError::Migration` if migrations fail.
    #[instrument]
    pub async fn new_in_memory() -> Result<Self, DbError> {
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await?;

        // Run migrations
        sqlx::migrate!("./migrations").run(&pool).await?;

        Ok(Self { pool })
    }

    /// Returns a reference to the underlying connection pool.
    ///
    /// Use this for executing queries with sqlx.
    #[must_use]
    pub fn pool(&self) -> &SqlitePool {
        &self.pool
    }

    /// Checks if WAL mode is enabled.
    ///
    /// Returns `true` if WAL mode is active, `false` otherwise.
    ///
    /// # Errors
    ///
    /// Returns `DbError::Connection` if the query fails.
    #[instrument(skip(self))]
    pub async fn is_wal_enabled(&self) -> Result<bool, DbError> {
        let result: (String,) = sqlx::query_as("PRAGMA journal_mode")
            .fetch_one(&self.pool)
            .await?;

        Ok(result.0.to_lowercase() == "wal")
    }

    /// Gracefully closes all connections in the pool.
    ///
    /// This should be called before the application exits to ensure
    /// all connections are properly closed. After calling this method,
    /// the Database instance should not be used.
    #[instrument(skip(self))]
    pub async fn close(self) {
        self.pool.close().await;
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_database_new_in_memory_succeeds() {
        let db = Database::new_in_memory().await;
        assert!(db.is_ok(), "Failed to create in-memory database");
    }

    #[tokio::test]
    async fn test_database_migrations_run_successfully() {
        let db = Database::new_in_memory().await.unwrap();

        // Verify queue table exists by inserting a row
        let result = sqlx::query(
            "INSERT INTO queue (url, source_type) VALUES ('https://example.com', 'direct_url')",
        )
        .execute(db.pool())
        .await;

        assert!(result.is_ok(), "Queue table should exist after migration");
    }

    #[tokio::test]
    async fn test_database_download_log_table_exists() {
        let db = Database::new_in_memory().await.unwrap();

        // Verify download_log table exists by inserting a row
        let result = sqlx::query(
            "INSERT INTO download_log (url, status, started_at) VALUES ('https://example.com', 'success', datetime('now'))",
        )
        .execute(db.pool())
        .await;

        assert!(
            result.is_ok(),
            "Download log table should exist after migration"
        );
    }

    #[tokio::test]
    async fn test_database_with_tempfile() {
        let temp_dir = tempfile::tempdir().unwrap();
        let db_path = temp_dir.path().join("test.db");

        let db = Database::new(&db_path).await;
        assert!(db.is_ok(), "Failed to create database at temp path");

        // Verify WAL mode is enabled for file-based databases
        let db = db.unwrap();
        let is_wal = db.is_wal_enabled().await.unwrap();
        assert!(is_wal, "WAL mode should be enabled for file-based database");
    }

    #[tokio::test]
    async fn test_database_queue_table_constraints() {
        let db = Database::new_in_memory().await.unwrap();

        // Test that invalid source_type is rejected
        let result = sqlx::query(
            "INSERT INTO queue (url, source_type) VALUES ('https://example.com', 'invalid_type')",
        )
        .execute(db.pool())
        .await;

        assert!(
            result.is_err(),
            "Invalid source_type should be rejected by CHECK constraint"
        );
    }

    #[tokio::test]
    async fn test_database_pool_returns_valid_pool() {
        let db = Database::new_in_memory().await.unwrap();
        let pool = db.pool();

        // Verify pool works by running a simple query
        let result: (i64,) = sqlx::query_as("SELECT 1").fetch_one(pool).await.unwrap();

        assert_eq!(result.0, 1);
    }

    #[tokio::test]
    async fn test_database_close_works() {
        let db = Database::new_in_memory().await.unwrap();
        db.close().await;
        // If we get here without panic, close worked
    }
}
