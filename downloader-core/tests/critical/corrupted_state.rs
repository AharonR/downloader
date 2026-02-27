//! Phase 5 (P1): Queue state corruption, inconsistent DB.
//! reset_in_progress recovers in_progress items after crash.

use downloader_core::{Database, Queue, QueueStatus};
use tempfile::TempDir;

#[tokio::test]
async fn p1_reset_in_progress_recovery() {
    let temp_dir = TempDir::new().expect("temp dir");
    let db_path = temp_dir.path().join("corrupt.db");
    let db = Database::new(&db_path).await.expect("db");
    let queue = Queue::new(db);

    let _ = queue
        .enqueue("https://example.com/x.pdf", "direct_url", None)
        .await
        .expect("enqueue");
    let item = queue.dequeue().await.expect("dequeue").expect("one");
    drop(queue);

    let db = Database::new(&db_path).await.expect("reopen");
    let queue = Queue::new(db);
    let n = queue.reset_in_progress().await.expect("reset");
    assert_eq!(n, 1);

    let item = queue.get(item.id).await.expect("get").expect("item");
    assert_eq!(item.status(), QueueStatus::Pending);
}
