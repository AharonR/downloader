//! Phase 1 (P0): DB transaction rollback edge cases.
//! Assert queue/DB state remains consistent when operations fail or are rolled back.

use downloader_core::{Database, Queue, QueueError, QueueStatus};
use tempfile::TempDir;

async fn setup_queue() -> (Queue, TempDir) {
    let temp_dir = TempDir::new().expect("temp dir");
    let db_path = temp_dir.path().join("test.db");
    let db = Database::new(&db_path).await.expect("create db");
    (Queue::new(db), temp_dir)
}

#[tokio::test]
async fn p0_mark_completed_nonexistent_returns_error() {
    let (queue, _temp) = setup_queue().await;

    let result = queue.mark_completed(999_999).await;

    assert!(result.is_err());
    assert!(matches!(
        result.unwrap_err(),
        QueueError::ItemNotFound(999_999)
    ));
}

#[tokio::test]
async fn p0_mark_failed_nonexistent_returns_error() {
    let (queue, _temp) = setup_queue().await;

    let result = queue.mark_failed(999_999, "test error", 0).await;

    assert!(result.is_err());
    assert!(matches!(
        result.unwrap_err(),
        QueueError::ItemNotFound(999_999)
    ));
}

#[tokio::test]
async fn p0_dequeue_after_mark_completed_other_items_unchanged() {
    let (queue, _temp) = setup_queue().await;

    let id1 = queue
        .enqueue("https://example.com/a.pdf", "direct_url", None)
        .await
        .expect("enqueue");
    let id2 = queue
        .enqueue("https://example.com/b.pdf", "direct_url", None)
        .await
        .expect("enqueue");

    let _ = queue.mark_completed(id1).await;

    let item = queue
        .dequeue()
        .await
        .expect("dequeue")
        .expect("one pending");
    assert_eq!(item.id, id2);

    let one = queue.get(id1).await.expect("get").expect("exists");
    let two = queue.get(id2).await.expect("get").expect("exists");
    assert_eq!(one.status(), QueueStatus::Completed);
    assert_eq!(two.status(), QueueStatus::InProgress);
}
