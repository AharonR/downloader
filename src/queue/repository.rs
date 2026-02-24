//! Repository seam for queue/history persistence operations.
//!
//! This trait keeps current `Queue` APIs intact while allowing higher-level
//! orchestration (download engine, command flows) to depend on an abstract data
//! access boundary.

use std::path::Path;

use async_trait::async_trait;

use super::{
    DownloadAttemptQuery, DownloadSearchCandidate, DownloadSearchQuery, NewDownloadAttempt, Queue,
    QueueItem, QueueStatus, Result,
};

/// Data-access contract for queue and history operations.
#[async_trait]
pub trait QueueRepository {
    /// Claims the next pending queue item.
    async fn dequeue(&self) -> Result<Option<QueueItem>>;

    /// Requeues a claimed item back to pending.
    async fn requeue(&self, id: i64) -> Result<()>;

    /// Marks an item completed with optional saved file path.
    async fn mark_completed_with_path(&self, id: i64, saved_path: Option<&Path>) -> Result<()>;

    /// Marks an item failed with message and retry count.
    async fn mark_failed(&self, id: i64, error: &str, retry_count: i64) -> Result<()>;

    /// Updates bytes/content-length progress metadata.
    async fn update_progress(
        &self,
        id: i64,
        bytes_downloaded: i64,
        content_length: Option<i64>,
    ) -> Result<()>;

    /// Returns the count of queue items in a status.
    async fn count_by_status(&self, status: QueueStatus) -> Result<i64>;

    /// Returns all queue items currently in progress.
    async fn get_in_progress(&self) -> Result<Vec<QueueItem>>;

    /// Returns all queue items for a status.
    async fn list_by_status(&self, status: QueueStatus) -> Result<Vec<QueueItem>>;

    /// Persists a terminal download attempt history row.
    async fn log_download_attempt(&self, attempt: &NewDownloadAttempt<'_>) -> Result<i64>;

    /// Reads paginated download attempts.
    async fn query_download_attempts(
        &self,
        query: &DownloadAttemptQuery,
    ) -> Result<Vec<super::DownloadAttempt>>;

    /// Reads searchable history candidates.
    async fn query_download_search_candidates(
        &self,
        query: &DownloadSearchQuery,
    ) -> Result<Vec<DownloadSearchCandidate>>;
}

#[async_trait]
impl QueueRepository for Queue {
    async fn dequeue(&self) -> Result<Option<QueueItem>> {
        Queue::dequeue(self).await
    }

    async fn requeue(&self, id: i64) -> Result<()> {
        Queue::requeue(self, id).await
    }

    async fn mark_completed_with_path(&self, id: i64, saved_path: Option<&Path>) -> Result<()> {
        Queue::mark_completed_with_path(self, id, saved_path).await
    }

    async fn mark_failed(&self, id: i64, error: &str, retry_count: i64) -> Result<()> {
        Queue::mark_failed(self, id, error, retry_count).await
    }

    async fn update_progress(
        &self,
        id: i64,
        bytes_downloaded: i64,
        content_length: Option<i64>,
    ) -> Result<()> {
        Queue::update_progress(self, id, bytes_downloaded, content_length).await
    }

    async fn count_by_status(&self, status: QueueStatus) -> Result<i64> {
        Queue::count_by_status(self, status).await
    }

    async fn get_in_progress(&self) -> Result<Vec<QueueItem>> {
        Queue::get_in_progress(self).await
    }

    async fn list_by_status(&self, status: QueueStatus) -> Result<Vec<QueueItem>> {
        Queue::list_by_status(self, status).await
    }

    async fn log_download_attempt(&self, attempt: &NewDownloadAttempt<'_>) -> Result<i64> {
        Queue::log_download_attempt(self, attempt).await
    }

    async fn query_download_attempts(
        &self,
        query: &DownloadAttemptQuery,
    ) -> Result<Vec<super::DownloadAttempt>> {
        Queue::query_download_attempts(self, query).await
    }

    async fn query_download_search_candidates(
        &self,
        query: &DownloadSearchQuery,
    ) -> Result<Vec<DownloadSearchCandidate>> {
        Queue::query_download_search_candidates(self, query).await
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use crate::Database;
    use crate::queue::{DownloadAttemptStatus, NewDownloadAttempt, QueueStatus};

    async fn pending_count(repo: &impl QueueRepository) -> Result<i64> {
        repo.count_by_status(QueueStatus::Pending).await
    }

    #[tokio::test]
    async fn test_queue_repository_trait_delegates_core_queue_lifecycle() {
        let db = Database::new_in_memory().await.unwrap();
        let queue = Queue::new(db);

        queue
            .enqueue("https://example.com/repo-seam.pdf", "direct_url", None)
            .await
            .unwrap();

        assert_eq!(pending_count(&queue).await.unwrap(), 1);

        let item = QueueRepository::dequeue(&queue).await.unwrap().unwrap();
        assert_eq!(item.url, "https://example.com/repo-seam.pdf");

        QueueRepository::requeue(&queue, item.id).await.unwrap();
        assert_eq!(pending_count(&queue).await.unwrap(), 1);
    }

    #[tokio::test]
    async fn test_queue_repository_trait_supports_history_logging_and_query() {
        let db = Database::new_in_memory().await.unwrap();
        let queue = Queue::new(db);

        let attempt = NewDownloadAttempt {
            url: "https://example.com/history.pdf",
            final_url: Some("https://example.com/history.pdf"),
            status: DownloadAttemptStatus::Success,
            file_path: Some("history.pdf"),
            file_size: Some(42),
            content_type: Some("application/pdf"),
            error_message: None,
            error_type: None,
            retry_count: 0,
            project: Some("/tmp/project"),
            original_input: Some("https://example.com/history.pdf"),
            http_status: Some(200),
            duration_ms: Some(10),
            title: Some("History"),
            authors: Some("Author"),
            doi: Some("10.1234/repo"),
            topics: None,
            parse_confidence: None,
            parse_confidence_factors: None,
        };
        QueueRepository::log_download_attempt(&queue, &attempt)
            .await
            .unwrap();

        let rows =
            QueueRepository::query_download_attempts(&queue, &DownloadAttemptQuery::default())
                .await
                .unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].url, "https://example.com/history.pdf");
    }
}
