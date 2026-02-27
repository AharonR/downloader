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
mod history;
mod item;
mod repository;

pub use error::QueueError;
pub use history::{
    DownloadAttempt, DownloadAttemptQuery, DownloadAttemptStatus, DownloadErrorType,
    DownloadSearchCandidate, DownloadSearchQuery, NewDownloadAttempt,
};
pub use item::{QueueItem, QueueMetadata, QueueStatus};
pub use repository::QueueRepository;

use crate::db::Database;
use sqlx::Row;
use tracing::instrument;

/// Returns `Ok(())` if at least one row was affected; otherwise [`QueueError::ItemNotFound`].
fn check_affected(id: i64, rows_affected: u64) -> Result<()> {
    if rows_affected == 0 {
        Err(QueueError::ItemNotFound(id))
    } else {
        Ok(())
    }
}

/// Default priority for new queue items.
const DEFAULT_PRIORITY: i64 = 0;

/// Default retry count for new queue items.
const DEFAULT_RETRY_COUNT: i64 = 0;

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
    /// * `source_type` - How the item entered the queue (`direct_url`, doi, reference, bibtex)
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
        self.enqueue_with_metadata(url, source_type, original_input, None)
            .await
    }

    /// Adds a new item to the queue with optional metadata for naming/indexing.
    ///
    /// # Errors
    ///
    /// Returns [`QueueError::Database`] if the insert fails.
    #[instrument(skip(self, metadata), fields(url = %url, source_type = %source_type))]
    pub async fn enqueue_with_metadata(
        &self,
        url: &str,
        source_type: &str,
        original_input: Option<&str>,
        metadata: Option<&QueueMetadata>,
    ) -> Result<i64> {
        let suggested_filename = metadata.and_then(|m| m.suggested_filename.as_deref());
        let title = metadata.and_then(|m| m.title.as_deref());
        let authors = metadata.and_then(|m| m.authors.as_deref());
        let year = metadata.and_then(|m| m.year.as_deref());
        let doi = metadata.and_then(|m| m.doi.as_deref());
        let topics_json = metadata
            .and_then(|m| m.topics.as_ref())
            .and_then(|t| QueueItem::serialize_topics(t));
        let parse_confidence = metadata.and_then(|m| m.parse_confidence.as_deref());
        let parse_confidence_factors = metadata.and_then(|m| m.parse_confidence_factors.as_deref());

        let result = sqlx::query(
            r"INSERT INTO queue (
                url,
                source_type,
                original_input,
                status,
                priority,
                retry_count,
                suggested_filename,
                meta_title,
                meta_authors,
                meta_year,
                meta_doi,
                topics,
                parse_confidence,
                parse_confidence_factors
              )
              VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
              RETURNING id",
        )
        .bind(url)
        .bind(source_type)
        .bind(original_input)
        .bind(QueueStatus::Pending.as_str())
        .bind(DEFAULT_PRIORITY)
        .bind(DEFAULT_RETRY_COUNT)
        .bind(suggested_filename)
        .bind(title)
        .bind(authors)
        .bind(year)
        .bind(doi)
        .bind(topics_json)
        .bind(parse_confidence)
        .bind(parse_confidence_factors)
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
              SET status = ?, updated_at = datetime('now')
              WHERE id = (
                  SELECT id FROM queue
                  WHERE status = ?
                  ORDER BY priority DESC, created_at ASC
                  LIMIT 1
              )
              RETURNING *",
        )
        .bind(QueueStatus::InProgress.as_str())
        .bind(QueueStatus::Pending.as_str())
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
        self.mark_completed_with_path(id, None).await
    }

    /// Marks an item as successfully completed and stores saved file path metadata.
    ///
    /// # Errors
    ///
    /// Returns [`QueueError::ItemNotFound`] if no item exists with the given ID.
    /// Returns [`QueueError::Database`] if the update fails.
    #[instrument(skip(self, saved_path))]
    pub async fn mark_completed_with_path(
        &self,
        id: i64,
        saved_path: Option<&std::path::Path>,
    ) -> Result<()> {
        let saved_path = saved_path.and_then(|p| p.to_str());
        let result = sqlx::query(
            r"UPDATE queue
              SET status = ?, saved_path = ?, updated_at = datetime('now')
              WHERE id = ?",
        )
        .bind(QueueStatus::Completed.as_str())
        .bind(saved_path)
        .bind(id)
        .execute(self.db.pool())
        .await?;

        check_affected(id, result.rows_affected())
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
              SET status = ?,
                  retry_count = ?,
                  last_error = ?,
                  updated_at = datetime('now')
              WHERE id = ?",
        )
        .bind(QueueStatus::Failed.as_str())
        .bind(retry_count)
        .bind(error)
        .bind(id)
        .execute(self.db.pool())
        .await?;

        check_affected(id, result.rows_affected())
    }

    /// Updates partial download progress metadata for resumable downloads.
    ///
    /// # Errors
    ///
    /// Returns [`QueueError::ItemNotFound`] if no item exists with the given ID.
    /// Returns [`QueueError::Database`] if the update fails.
    #[instrument(skip(self))]
    pub async fn update_progress(
        &self,
        id: i64,
        bytes_downloaded: i64,
        content_length: Option<i64>,
    ) -> Result<()> {
        let result = sqlx::query(
            r"UPDATE queue
              SET bytes_downloaded = ?, content_length = ?, updated_at = datetime('now')
              WHERE id = ?",
        )
        .bind(bytes_downloaded)
        .bind(content_length)
        .bind(id)
        .execute(self.db.pool())
        .await?;

        check_affected(id, result.rows_affected())
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
              SET status = ?, updated_at = datetime('now')
              WHERE id = ?",
        )
        .bind(QueueStatus::Pending.as_str())
        .bind(id)
        .execute(self.db.pool())
        .await?;

        check_affected(id, result.rows_affected())
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

    /// Checks whether a URL already has a pending or in-progress queue entry.
    ///
    /// Used to avoid duplicate enqueue when resuming with the same input.
    ///
    /// # Errors
    ///
    /// Returns [`QueueError::Database`] if the query fails.
    #[instrument(skip(self), fields(url = %url))]
    pub async fn has_active_url(&self, url: &str) -> Result<bool> {
        let result = sqlx::query(
            r"SELECT COUNT(*) as count FROM queue
              WHERE url = ? AND status IN (?, ?)",
        )
        .bind(url)
        .bind(QueueStatus::Pending.as_str())
        .bind(QueueStatus::InProgress.as_str())
        .fetch_one(self.db.pool())
        .await?;

        Ok(result.get::<i64, _>("count") > 0)
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
            r"SELECT * FROM queue WHERE status = ? ORDER BY updated_at ASC",
        )
        .bind(QueueStatus::InProgress.as_str())
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
              SET status = ?, updated_at = datetime('now')
              WHERE status = ?",
        )
        .bind(QueueStatus::Pending.as_str())
        .bind(QueueStatus::InProgress.as_str())
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

        check_affected(id, result.rows_affected())
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
    use crate::Database;

    #[test]
    fn test_queue_result_type_alias() {
        // Verify the Result type alias works correctly
        let ok_result: Result<i64> = Ok(42);
        assert!(ok_result.is_ok());

        let err_result: Result<i64> = Err(QueueError::ItemNotFound(1));
        assert!(err_result.is_err());
    }

    /// Regression: after refactor, update/delete methods use check_affected and return
    /// ItemNotFound when no row is affected. Ensures mark_completed_with_path returns
    /// ItemNotFound for a non-existent id.
    #[tokio::test]
    async fn test_regression_mark_completed_with_path_returns_item_not_found_for_missing_id() {
        let db = Database::new_in_memory().await.unwrap();
        let queue = Queue::new(db);

        let id = queue
            .enqueue("https://example.com/doc.pdf", "direct_url", None)
            .await
            .unwrap();
        assert_eq!(id, 1);

        let missing_id = 999;
        let result = queue.mark_completed_with_path(missing_id, None).await;
        assert!(
            matches!(result, Err(QueueError::ItemNotFound(999))),
            "expected ItemNotFound(999), got {:?}",
            result
        );
    }
}
