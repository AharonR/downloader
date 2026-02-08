//! Queue module for managing download queue persistence.
//!
//! This module provides `SQLite`-backed queue management for tracking
//! download items through their lifecycle (pending → `in_progress` → completed/failed).
//!
//! # Overview
//!
//! The queue system consists of:
//! - [`Queue`] - Main interface for queue operations
//! - [`QueueItem`] - Individual queue entry with metadata
//! - [`QueueStatus`] - Item lifecycle states
//! - [`QueueError`] - Operation error types
//!
//! # Example
//!
//! ```ignore
//! use downloader_core::queue::{Queue, QueueStatus};
//! use downloader_core::Database;
//! use std::path::Path;
//!
//! let db = Database::new(Path::new("queue.db")).await?;
//! let queue = Queue::new(db);
//!
//! // Add items to queue
//! let id = queue.enqueue("https://example.com/paper.pdf", "direct_url", None).await?;
//!
//! // Process items
//! if let Some(item) = queue.dequeue().await? {
//!     // ... download the item ...
//!     queue.mark_completed(item.id).await?;
//! }
//! ```

mod error;
mod item;

pub use error::QueueError;
pub use item::{QueueItem, QueueStatus};

use crate::db::Database;
use sqlx::Row;
use tracing::instrument;

/// Result type for queue operations.
pub type Result<T> = std::result::Result<T, QueueError>;

/// Queue manager for download items.
///
/// Provides atomic operations for managing download queue items
/// backed by `SQLite` with WAL mode for concurrent access.
#[derive(Debug, Clone)]
pub struct Queue {
    db: Database,
}

impl Queue {
    /// Creates a new queue manager with the given database connection.
    #[must_use]
    pub fn new(db: Database) -> Self {
        Self { db }
    }

    /// Adds a new item to the queue with pending status.
    ///
    /// # Arguments
    ///
    /// * `url` - The resolved URL to download
    /// * `source_type` - How the item entered the queue (`direct_url`, doi, reference)
    /// * `original_input` - The original user input before resolution (e.g., DOI string)
    ///
    /// # Returns
    ///
    /// The ID of the newly created queue item.
    ///
    /// # Errors
    ///
    /// Returns [`QueueError::Database`] if the insert fails.
    #[instrument(skip(self), fields(url = %url, source_type = %source_type))]
    pub async fn enqueue(
        &self,
        url: &str,
        source_type: &str,
        original_input: Option<&str>,
    ) -> Result<i64> {
        let result = sqlx::query(
            r"INSERT INTO queue (url, source_type, original_input, status, priority, retry_count)
              VALUES (?, ?, ?, 'pending', 0, 0)
              RETURNING id",
        )
        .bind(url)
        .bind(source_type)
        .bind(original_input)
        .fetch_one(self.db.pool())
        .await?;

        Ok(result.get("id"))
    }

    /// Retrieves and claims the next pending item for processing.
    ///
    /// Atomically transitions the highest-priority pending item to `in_progress`
    /// and returns it. Returns None if no pending items exist.
    ///
    /// # Errors
    ///
    /// Returns [`QueueError::Database`] if the query fails.
    #[instrument(skip(self))]
    pub async fn dequeue(&self) -> Result<Option<QueueItem>> {
        // Atomic UPDATE...RETURNING ensures no race condition between select and update
        let item = sqlx::query_as::<_, QueueItem>(
            r"UPDATE queue
              SET status = 'in_progress', updated_at = datetime('now')
              WHERE id = (
                  SELECT id FROM queue
                  WHERE status = 'pending'
                  ORDER BY priority DESC, created_at ASC
                  LIMIT 1
              )
              RETURNING *",
        )
        .fetch_optional(self.db.pool())
        .await?;

        Ok(item)
    }

    /// Marks an item as successfully completed.
    ///
    /// # Errors
    ///
    /// Returns [`QueueError::ItemNotFound`] if no item exists with the given ID.
    /// Returns [`QueueError::Database`] if the update fails.
    #[instrument(skip(self))]
    pub async fn mark_completed(&self, id: i64) -> Result<()> {
        let result = sqlx::query(
            r"UPDATE queue
              SET status = 'completed', updated_at = datetime('now')
              WHERE id = ?",
        )
        .bind(id)
        .execute(self.db.pool())
        .await?;

        if result.rows_affected() == 0 {
            return Err(QueueError::ItemNotFound(id));
        }

        Ok(())
    }

    /// Marks an item as failed with an error message and retry count.
    ///
    /// # Errors
    ///
    /// Returns [`QueueError::ItemNotFound`] if no item exists with the given ID.
    /// Returns [`QueueError::Database`] if the update fails.
    #[instrument(skip(self), fields(error = %error, retry_count))]
    pub async fn mark_failed(&self, id: i64, error: &str, retry_count: i64) -> Result<()> {
        let result = sqlx::query(
            r"UPDATE queue
              SET status = 'failed',
                  retry_count = ?,
                  last_error = ?,
                  updated_at = datetime('now')
              WHERE id = ?",
        )
        .bind(retry_count)
        .bind(error)
        .bind(id)
        .execute(self.db.pool())
        .await?;

        if result.rows_affected() == 0 {
            return Err(QueueError::ItemNotFound(id));
        }

        Ok(())
    }

    /// Returns an item to pending status for retry.
    ///
    /// Used when an item needs to be reprocessed (e.g., after transient failure).
    ///
    /// # Errors
    ///
    /// Returns [`QueueError::ItemNotFound`] if no item exists with the given ID.
    /// Returns [`QueueError::Database`] if the update fails.
    #[instrument(skip(self))]
    pub async fn requeue(&self, id: i64) -> Result<()> {
        let result = sqlx::query(
            r"UPDATE queue
              SET status = 'pending', updated_at = datetime('now')
              WHERE id = ?",
        )
        .bind(id)
        .execute(self.db.pool())
        .await?;

        if result.rows_affected() == 0 {
            return Err(QueueError::ItemNotFound(id));
        }

        Ok(())
    }

    /// Gets a queue item by ID.
    ///
    /// # Errors
    ///
    /// Returns [`QueueError::Database`] if the query fails.
    #[instrument(skip(self))]
    pub async fn get(&self, id: i64) -> Result<Option<QueueItem>> {
        let item = sqlx::query_as::<_, QueueItem>(r"SELECT * FROM queue WHERE id = ?")
            .bind(id)
            .fetch_optional(self.db.pool())
            .await?;

        Ok(item)
    }

    /// Counts items by status.
    ///
    /// # Errors
    ///
    /// Returns [`QueueError::Database`] if the query fails.
    #[instrument(skip(self))]
    pub async fn count_by_status(&self, status: QueueStatus) -> Result<i64> {
        let result = sqlx::query(r"SELECT COUNT(*) as count FROM queue WHERE status = ?")
            .bind(status.as_str())
            .fetch_one(self.db.pool())
            .await?;

        Ok(result.get("count"))
    }

    /// Returns all items currently in progress.
    ///
    /// Used for crash recovery to identify items that were being processed
    /// when the application terminated unexpectedly.
    ///
    /// # Errors
    ///
    /// Returns [`QueueError::Database`] if the query fails.
    #[instrument(skip(self))]
    pub async fn get_in_progress(&self) -> Result<Vec<QueueItem>> {
        let items = sqlx::query_as::<_, QueueItem>(
            r"SELECT * FROM queue WHERE status = 'in_progress' ORDER BY updated_at ASC",
        )
        .fetch_all(self.db.pool())
        .await?;

        Ok(items)
    }

    /// Resets all in-progress items back to pending status.
    ///
    /// Called at startup for crash recovery - any items left `in_progress`
    /// from a previous session are returned to the queue for reprocessing.
    ///
    /// # Returns
    ///
    /// The number of items that were reset.
    ///
    /// # Errors
    ///
    /// Returns [`QueueError::Database`] if the update fails.
    #[instrument(skip(self))]
    pub async fn reset_in_progress(&self) -> Result<u64> {
        let result = sqlx::query(
            r"UPDATE queue
              SET status = 'pending', updated_at = datetime('now')
              WHERE status = 'in_progress'",
        )
        .execute(self.db.pool())
        .await?;

        Ok(result.rows_affected())
    }

    /// Lists items filtered by status.
    ///
    /// Returns items ordered by priority (descending) and creation time (ascending).
    ///
    /// # Errors
    ///
    /// Returns [`QueueError::Database`] if the query fails.
    #[instrument(skip(self))]
    pub async fn list_by_status(&self, status: QueueStatus) -> Result<Vec<QueueItem>> {
        let items = sqlx::query_as::<_, QueueItem>(
            r"SELECT * FROM queue
              WHERE status = ?
              ORDER BY priority DESC, created_at ASC",
        )
        .bind(status.as_str())
        .fetch_all(self.db.pool())
        .await?;

        Ok(items)
    }

    /// Lists all items in the queue.
    ///
    /// Returns items ordered by priority (descending) and creation time (ascending).
    ///
    /// # Errors
    ///
    /// Returns [`QueueError::Database`] if the query fails.
    #[instrument(skip(self))]
    pub async fn list_all(&self) -> Result<Vec<QueueItem>> {
        let items = sqlx::query_as::<_, QueueItem>(
            r"SELECT * FROM queue ORDER BY priority DESC, created_at ASC",
        )
        .fetch_all(self.db.pool())
        .await?;

        Ok(items)
    }

    /// Removes a queue item by ID.
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` if the item was deleted.
    ///
    /// # Errors
    ///
    /// Returns [`QueueError::ItemNotFound`] if no item exists with the given ID.
    /// Returns [`QueueError::Database`] if the delete fails.
    #[instrument(skip(self))]
    pub async fn remove(&self, id: i64) -> Result<()> {
        let result = sqlx::query(r"DELETE FROM queue WHERE id = ?")
            .bind(id)
            .execute(self.db.pool())
            .await?;

        if result.rows_affected() == 0 {
            return Err(QueueError::ItemNotFound(id));
        }

        Ok(())
    }

    /// Clears all items with a specific status.
    ///
    /// # Returns
    ///
    /// The number of items removed.
    ///
    /// # Errors
    ///
    /// Returns [`QueueError::Database`] if the delete fails.
    #[instrument(skip(self))]
    pub async fn clear_by_status(&self, status: QueueStatus) -> Result<u64> {
        let result = sqlx::query(r"DELETE FROM queue WHERE status = ?")
            .bind(status.as_str())
            .execute(self.db.pool())
            .await?;

        Ok(result.rows_affected())
    }
}

#[cfg(test)]
mod tests {
    // Integration tests require actual database setup - see tests/queue_integration.rs
    // Unit tests for Queue struct methods are minimal since they're thin wrappers around SQL

    use super::*;

    #[test]
    fn test_queue_result_type_alias() {
        // Verify the Result type alias works correctly
        let ok_result: Result<i64> = Ok(42);
        assert!(ok_result.is_ok());

        let err_result: Result<i64> = Err(QueueError::ItemNotFound(1));
        assert!(err_result.is_err());
    }
}
