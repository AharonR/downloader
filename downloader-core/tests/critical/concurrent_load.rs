//! Phase 4 (P1): 100+ concurrent downloads, semaphore stress.

use downloader_core::{Database, Queue};
use tempfile::TempDir;

use crate::support::critical_utils::concurrent_load_generator;

#[tokio::test]
async fn p1_concurrent_load_generator_no_deadlock() {
    let temp_dir = TempDir::new().expect("temp dir");
    let db_path = temp_dir.path().join("test.db");
    let db = Database::new(&db_path).await.expect("db");
    let queue = Queue::new(db);

    let result = concurrent_load_generator(queue, 20, 10).await;

    assert!(result.enqueued > 0, "enqueued={}", result.enqueued);
    assert!(result.completed > 0, "completed={}", result.completed);
}

#[tokio::test]
#[ignore] // needs file DB; run with --ignored in nightly
async fn p1_high_concurrency_100_ops() {
    let temp_dir = TempDir::new().expect("temp dir");
    let db_path = temp_dir.path().join("test.db");
    let db = Database::new(&db_path).await.expect("db");
    let queue = Queue::new(db);

    let result = concurrent_load_generator(queue, 10, 10).await;

    assert_eq!(result.enqueued, result.completed + result.failed);
}
