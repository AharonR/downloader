//! Phase 1 (P0): DB migration failures, backup/restore.
//! Re-open DB after normal close; verify queue state persists.

use downloader_core::{Database, Queue, QueueStatus};
use tempfile::TempDir;

#[tokio::test]
async fn p0_queue_persists_after_reopen() {
    let temp_dir = TempDir::new().expect("temp dir");
    let db_path = temp_dir.path().join("persist.db");

    let id = {
        let db = Database::new(&db_path).await.expect("create db");
        let queue = Queue::new(db);
        let id = queue
            .enqueue("https://example.com/persist.pdf", "direct_url", None)
            .await
            .expect("enqueue");
        drop(queue);
        id
    };

    let db = Database::new(&db_path).await.expect("reopen db");
    let queue = Queue::new(db);
    let item = queue.get(id).await.expect("get").expect("item still there");
    assert_eq!(item.url, "https://example.com/persist.pdf");
    assert_eq!(item.status(), QueueStatus::Pending);
}

#[tokio::test]
async fn p0_wal_mode_after_reopen() {
    let temp_dir = TempDir::new().expect("temp dir");
    let db_path = temp_dir.path().join("wal.db");

    let db = Database::new(&db_path).await.expect("create db");
    let wal1 = db.is_wal_enabled().await.expect("pragma");
    drop(db);

    let db = Database::new(&db_path).await.expect("reopen db");
    let wal2 = db.is_wal_enabled().await.expect("pragma");
    assert!(wal1 && wal2, "WAL should remain enabled after reopen");
}
