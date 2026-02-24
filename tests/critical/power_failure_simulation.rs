//! Phase 5 (P1): Unclean shutdowns, WAL recovery.
//! Re-open after normal close; WAL should recover.

use downloader_core::{Database, Queue, QueueStatus};
use tempfile::TempDir;

#[tokio::test]
async fn p1_wal_recovery_after_reopen() {
    let temp_dir = TempDir::new().expect("temp dir");
    let db_path = temp_dir.path().join("power.db");

    let id = {
        let db = Database::new(&db_path).await.expect("db");
        let queue = Queue::new(db);
        let id = queue
            .enqueue("https://example.com/a.pdf", "direct_url", None)
            .await
            .expect("enqueue");
        drop(queue);
        id
    };

    let db = Database::new(&db_path)
        .await
        .expect("reopen after simulated power loss");
    let wal = db.is_wal_enabled().await.expect("pragma");
    assert!(wal, "WAL should still be enabled after reopen");

    let queue = Queue::new(db);
    let item = queue.get(id).await.expect("get").expect("item");
    assert_eq!(item.status(), QueueStatus::Pending);
}
