//! Database connection and schema management.
//!
//! This module provides `SQLite` database connectivity with:
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
/// Kept low for `SQLite` since it uses file-level locking.
pub const DEFAULT_MAX_CONNECTIONS: u32 = 5;

/// Default `SQLite` busy timeout in milliseconds.
/// Connections will wait this long before returning `SQLITE_BUSY`.
pub const DEFAULT_BUSY_TIMEOUT_MS: u32 = 5000;

/// Optional database connection settings (pool size, busy timeout).
/// Used by [`Database::new_with_options`].
#[derive(Debug, Clone)]
pub struct DatabaseOptions {
    /// Maximum connections in the pool (1..=20 for `SQLite`).
    pub max_connections: u32,
    /// Busy timeout in milliseconds.
    pub busy_timeout_ms: u32,
}

impl Default for DatabaseOptions {
    fn default() -> Self {
        Self {
            max_connections: DEFAULT_MAX_CONNECTIONS,
            busy_timeout_ms: DEFAULT_BUSY_TIMEOUT_MS,
        }
    }
}

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
/// Handles `SQLite` connection pooling, WAL mode configuration,
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
    /// * `db_path` - Path to the `SQLite` database file
    ///
    /// # Errors
    ///
    /// Returns `DbError::Connection` if the connection fails,
    /// or `DbError::Migration` if migrations fail.
    #[instrument(skip(db_path), fields(path = %db_path.display()))]
    pub async fn new(db_path: &Path) -> Result<Self, DbError> {
        Self::new_with_options(db_path, &DatabaseOptions::default()).await
    }

    /// Creates a new database connection with explicit pool and timeout options.
    ///
    /// # Errors
    ///
    /// Returns `DbError::Connection` if the connection fails,
    /// or `DbError::Migration` if migrations fail.
    #[instrument(skip(db_path, options), fields(path = %db_path.display()))]
    pub async fn new_with_options(
        db_path: &Path,
        options: &DatabaseOptions,
    ) -> Result<Self, DbError> {
        let db_url = format!("sqlite:{}?mode=rwc", db_path.display());

        let pool = SqlitePoolOptions::new()
            .max_connections(options.max_connections)
            .connect(&db_url)
            .await?;

        // Enable WAL mode for concurrent reads
        sqlx::query("PRAGMA journal_mode=WAL")
            .execute(&pool)
            .await?;

        // Set busy timeout to avoid immediate lock errors
        sqlx::query(&format!("PRAGMA busy_timeout={}", options.busy_timeout_ms))
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
    use sqlx::Row;

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
    async fn test_database_queue_dequeue_composite_index_exists() {
        let db = Database::new_in_memory().await.unwrap();
        let rows = sqlx::query("PRAGMA index_list('queue')")
            .fetch_all(db.pool())
            .await
            .unwrap();

        let has_index = rows.iter().any(|row| {
            row.try_get::<String, _>("name")
                .map(|name| name == "idx_queue_status_priority_created")
                .unwrap_or(false)
        });
        assert!(
            has_index,
            "expected idx_queue_status_priority_created migration index to exist"
        );
    }

    #[tokio::test]
    async fn test_database_dequeue_query_plan_uses_composite_index() {
        let db = Database::new_in_memory().await.unwrap();
        // Seed a few rows so planner has realistic table stats.
        for i in 0..50 {
            let status = if i % 3 == 0 { "completed" } else { "pending" };
            sqlx::query("INSERT INTO queue (url, source_type, status, priority, created_at) VALUES (?, 'direct_url', ?, ?, datetime('now'))")
                .bind(format!("https://example.com/{i}"))
                .bind(status)
                .bind(i % 10)
                .execute(db.pool())
                .await
                .unwrap();
        }
        sqlx::query("ANALYZE").execute(db.pool()).await.unwrap();

        let plan_rows = sqlx::query(
            "EXPLAIN QUERY PLAN SELECT id FROM queue WHERE status='pending' ORDER BY priority DESC, created_at ASC LIMIT 1",
        )
        .fetch_all(db.pool())
        .await
        .unwrap();

        let uses_composite = plan_rows.iter().any(|row| {
            row.try_get::<String, _>(3)
                .map(|detail| detail.contains("idx_queue_status_priority_created"))
                .unwrap_or(false)
        });
        assert!(
            uses_composite,
            "expected dequeue query plan to use idx_queue_status_priority_created"
        );
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
    async fn test_database_download_log_metadata_columns_exist() {
        let db = Database::new_in_memory().await.unwrap();

        let result = sqlx::query(
            r"INSERT INTO download_log (
                url,
                status,
                started_at,
                title,
                authors,
                doi
              )
              VALUES (
                'https://example.com/metadata.pdf',
                'success',
                datetime('now'),
                'Metadata Title',
                'Doe, Jane',
                '10.1234/example'
              )",
        )
        .execute(db.pool())
        .await;

        assert!(
            result.is_ok(),
            "Download log metadata columns should exist after migration"
        );
    }

    #[tokio::test]
    async fn test_database_download_log_failure_detail_columns_exist() {
        let db = Database::new_in_memory().await.unwrap();

        let result = sqlx::query(
            r"INSERT INTO download_log (
                url,
                status,
                started_at,
                error_message,
                error_type,
                retry_count,
                last_retry_at,
                original_input
              )
              VALUES (
                'https://example.com/failure.pdf',
                'failed',
                datetime('now'),
                'HTTP 404
  Suggestion: Verify URL',
                'not_found',
                2,
                datetime('now'),
                '10.1000/original'
              )",
        )
        .execute(db.pool())
        .await;

        assert!(
            result.is_ok(),
            "Download log failure detail columns should exist after migration"
        );
    }

    #[tokio::test]
    async fn test_database_queue_parse_confidence_columns_exist() {
        let db = Database::new_in_memory().await.unwrap();

        let result = sqlx::query(
            r#"INSERT INTO queue (
                url,
                source_type,
                parse_confidence,
                parse_confidence_factors
              )
              VALUES (
                'https://example.com/reference',
                'reference',
                'low',
                '{"has_authors":false,"has_year":true,"has_title":false,"author_count":0}'
              )"#,
        )
        .execute(db.pool())
        .await;

        assert!(
            result.is_ok(),
            "Queue parse confidence columns should exist after migration"
        );
    }

    #[tokio::test]
    async fn test_database_download_log_parse_confidence_columns_exist() {
        let db = Database::new_in_memory().await.unwrap();

        let result = sqlx::query(
            r#"INSERT INTO download_log (
                url,
                status,
                started_at,
                parse_confidence,
                parse_confidence_factors
              )
              VALUES (
                'https://example.com/reference',
                'success',
                datetime('now'),
                'medium',
                '{"has_authors":true,"has_year":true,"has_title":false,"author_count":1}'
              )"#,
        )
        .execute(db.pool())
        .await;

        assert!(
            result.is_ok(),
            "Download log parse confidence columns should exist after migration"
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
    async fn test_database_new_with_options() {
        let temp_dir = tempfile::tempdir().unwrap();
        let db_path = temp_dir.path().join("options.db");
        let options = DatabaseOptions {
            max_connections: 2,
            busy_timeout_ms: 1000,
        };

        let db = Database::new_with_options(&db_path, &options).await;
        assert!(db.is_ok(), "new_with_options should succeed");
        let db = db.unwrap();
        let is_wal = db.is_wal_enabled().await.unwrap();
        assert!(is_wal, "WAL mode should be enabled");
        // Verify pool works
        let _: (i64,) = sqlx::query_as("SELECT 1")
            .fetch_one(db.pool())
            .await
            .unwrap();
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
