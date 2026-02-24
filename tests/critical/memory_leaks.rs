//! Phase 4 (P1): Stream cleanup, connection pool leaks.
//! Run many cycles; detect growth (pattern only; full leak detection can be nightly).

use std::sync::Arc;

use downloader_core::{
    Database, DownloadEngine, HttpClient, Queue, QueueStatus, RateLimiter, RetryPolicy,
};
use tempfile::TempDir;
use wiremock::matchers::{method, path};
use wiremock::{Mock, ResponseTemplate};

use crate::support::socket_guard::{socket_skip_return, start_mock_server_or_skip};

#[tokio::test]
async fn p1_many_download_cycles_complete_without_panic() {
    let Some(mock_server) = start_mock_server_or_skip().await else {
        return socket_skip_return();
    };

    Mock::given(method("GET"))
        .and(path("/small"))
        .respond_with(ResponseTemplate::new(200).set_body_bytes(b"x"))
        .mount(&mock_server)
        .await;

    let temp_dir = TempDir::new().expect("temp dir");
    let db_path = temp_dir.path().join("test.db");
    let db = Database::new(&db_path).await.expect("db");
    let queue = Queue::new(db);

    let url = format!("{}/small", mock_server.uri());
    let client = HttpClient::new();
    let rate_limiter = Arc::new(RateLimiter::disabled());
    let engine =
        DownloadEngine::new(2, RetryPolicy::with_max_attempts(1), rate_limiter).expect("engine");

    let cycles = 30_usize;
    for i in 0..cycles {
        let id = queue
            .enqueue(&url, "direct_url", None)
            .await
            .expect("enqueue");
        let output_dir = TempDir::new().expect("temp dir");
        let _ = engine
            .process_queue(&queue, &client, output_dir.path())
            .await
            .expect("process");
        let item = queue.get(id).await.expect("get").expect("item");
        assert_eq!(item.status(), QueueStatus::Completed, "cycle {}", i);
    }
}
