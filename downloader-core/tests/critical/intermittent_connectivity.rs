//! Phase 2 (P0): Flaky networks; retry succeeds after N failures.

use std::sync::Arc;

use downloader_core::{
    Database, DownloadEngine, HttpClient, Queue, QueueStatus, RateLimiter, RetryPolicy,
};
use tempfile::TempDir;

use crate::support::critical_utils::flaky_network_mock;
use crate::support::socket_guard::socket_skip_return;

#[tokio::test]
async fn p0_flaky_mock_fail_twice_then_succeed() {
    let Some((_mock_server, base_uri)) = flaky_network_mock(2, b"ok".to_vec()).await else {
        return socket_skip_return();
    };

    let temp_dir = TempDir::new().expect("temp dir");
    let db_path = temp_dir.path().join("test.db");
    let db = Database::new(&db_path).await.expect("db");
    let queue = Queue::new(db);

    let url = format!("{}/file", base_uri);
    let id = queue
        .enqueue(&url, "direct_url", None)
        .await
        .expect("enqueue");

    let client = HttpClient::new();
    let rate_limiter = Arc::new(RateLimiter::disabled());
    let engine = DownloadEngine::new(2, RetryPolicy::default(), rate_limiter).expect("engine");

    let output_dir = TempDir::new().expect("temp dir");
    let stats = engine
        .process_queue(&queue, &client, output_dir.path())
        .await
        .expect("process");

    assert_eq!(stats.completed(), 1);
    let item = queue.get(id).await.expect("get").expect("item");
    assert_eq!(item.status(), QueueStatus::Completed);
}
