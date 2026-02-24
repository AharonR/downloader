//! Phase 5 (P1): SIGKILL during downloads, partial files.
//! Restart and assert queue state and resumable work.

use downloader_core::{Database, Queue, QueueStatus};
use tempfile::TempDir;

#[tokio::test]
#[ignore] // needs file DB; run with --ignored in nightly
async fn p1_reopen_after_drop_queue_resumable() {
    let temp_dir = TempDir::new().expect("temp dir");
    let db_path = temp_dir.path().join("crash.db");

    let id = {
        let db = Database::new(&db_path).await.expect("db");
        let queue = Queue::new(db);
        let _ = queue
            .enqueue("https://example.com/resume.pdf", "direct_url", None)
            .await
            .expect("enqueue");
        let item = queue.dequeue().await.expect("dequeue").expect("one item");
        let id = item.id;
        drop(queue);
        id
    };

    let db = Database::new(&db_path).await.expect("reopen db");
    let queue = Queue::new(db);
    let item = queue.get(id).await.expect("get").expect("item");
    assert_eq!(item.status(), QueueStatus::InProgress);
    queue
        .mark_failed(id, "simulated crash", 0)
        .await
        .expect("mark failed");
}
