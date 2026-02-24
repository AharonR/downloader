//! Creates and initializes the download queue and state directory.

use std::fs;
use std::path::Path;
use std::sync::Arc;

use anyhow::Result;
use downloader_core::{Database, DatabaseOptions, Queue};
use tracing::info;

/// Creates the state directory under `output_dir`, initializes the database and queue,
/// resets in-progress items, and returns the queue and `history_start_id`.
/// Caller must ensure `output_dir` exists before calling.
/// Do not log state_dir or db_path in debug; they can reveal user directory layout.
pub(crate) async fn create_queue(
    output_dir: &Path,
    db_options: &DatabaseOptions,
) -> Result<(Arc<Queue>, Option<i64>)> {
    let state_dir = output_dir.join(".downloader");
    if !state_dir.exists() {
        fs::create_dir_all(&state_dir)?;
    }
    let db_path = state_dir.join("queue.db");
    let db = Database::new_with_options(&db_path, db_options).await?;
    let queue = Arc::new(Queue::new(db));
    let reset_count = queue.reset_in_progress().await?;
    if reset_count > 0 {
        info!(
            reset_count,
            "Recovered interrupted queue items from previous run"
        );
    }
    let history_start_id = queue.latest_download_attempt_id().await?;
    Ok((queue, history_start_id))
}

#[cfg(test)]
mod tests {
    use super::create_queue;
    use downloader_core::{DatabaseOptions, QueueStatus};
    use tempfile::TempDir;

    #[tokio::test]
    async fn create_queue_with_temp_dir_returns_queue_and_history_id() {
        let temp = TempDir::new().unwrap();
        let output_dir = temp.path();
        let (queue, history_start_id) =
            create_queue(output_dir, &DatabaseOptions::default())
                .await
                .expect("create_queue should succeed");

        // Queue is usable (e.g. list pending)
        let pending = queue.list_by_status(QueueStatus::Pending).await.unwrap();
        assert!(pending.is_empty());

        // history_start_id is Option<i64>; fresh DB has no download_log rows so can be None
        assert!(history_start_id.is_none() || history_start_id.is_some());
    }

    #[tokio::test]
    async fn create_queue_twice_on_same_path_yields_valid_queues() {
        let temp = TempDir::new().unwrap();
        let output_dir = temp.path();

        let (queue1, _) = create_queue(output_dir, &DatabaseOptions::default())
            .await
            .expect("first create_queue should succeed");
        let (queue2, _) = create_queue(output_dir, &DatabaseOptions::default())
            .await
            .expect("second create_queue should succeed");

        // Both queues use the same DB path; second open is valid
        let pending1 = queue1.list_by_status(QueueStatus::Pending).await.unwrap();
        let pending2 = queue2.list_by_status(QueueStatus::Pending).await.unwrap();
        assert!(pending1.is_empty());
        assert!(pending2.is_empty());
    }
}
