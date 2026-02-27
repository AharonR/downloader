//! Phase 2 (P0): 429 responses, Retry-After headers.
//! Assert backoff and eventual success or final failure.

use std::sync::Arc;

use downloader_core::{
    Database, DownloadEngine, HttpClient, Queue, QueueStatus, RateLimiter, RetryPolicy,
};
use tempfile::TempDir;
use wiremock::matchers::{method, path};
use wiremock::{Mock, ResponseTemplate};

use crate::support::socket_guard::{socket_skip_return, start_mock_server_or_skip};

#[tokio::test]
async fn p0_429_then_200_succeeds_after_retry() {
    let Some(mock_server) = start_mock_server_or_skip().await else {
        return socket_skip_return();
    };

    Mock::given(method("GET"))
        .and(path("/rate-limited"))
        .respond_with(ResponseTemplate::new(429))
        .up_to_n_times(1)
        .mount(&mock_server)
        .await;

    Mock::given(method("GET"))
        .and(path("/rate-limited"))
        .respond_with(ResponseTemplate::new(200).set_body_bytes(b"content"))
        .mount(&mock_server)
        .await;

    let temp_dir = TempDir::new().expect("temp dir");
    let db_path = temp_dir.path().join("test.db");
    let db = Database::new(&db_path).await.expect("db");
    let queue = Queue::new(db);

    let url = format!("{}/rate-limited", mock_server.uri());
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
    assert!(stats.retried() >= 1);
    let item = queue.get(id).await.expect("get").expect("item");
    assert_eq!(item.status(), QueueStatus::Completed);
}

#[tokio::test]
async fn p0_429_with_retry_after_header_respected() {
    let Some(mock_server) = start_mock_server_or_skip().await else {
        return socket_skip_return();
    };

    Mock::given(method("GET"))
        .and(path("/retry-after"))
        .respond_with(ResponseTemplate::new(429).insert_header("Retry-After", "0"))
        .up_to_n_times(1)
        .mount(&mock_server)
        .await;

    Mock::given(method("GET"))
        .and(path("/retry-after"))
        .respond_with(ResponseTemplate::new(200).set_body_bytes(b"ok"))
        .mount(&mock_server)
        .await;

    let temp_dir = TempDir::new().expect("temp dir");
    let db_path = temp_dir.path().join("test.db");
    let db = Database::new(&db_path).await.expect("db");
    let queue = Queue::new(db);

    let url = format!("{}/retry-after", mock_server.uri());
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
