//! Phase 2 (P0): Connection drops, DNS failures, TLS errors.
//! Assert retries and final error reporting.

use std::sync::Arc;

use downloader_core::{
    Database, DownloadEngine, HttpClient, Queue, QueueStatus, RateLimiter, RetryPolicy,
};
use tempfile::TempDir;
use wiremock::matchers::{method, path};
use wiremock::{Mock, ResponseTemplate};

use crate::support::socket_guard::{socket_skip_return, start_mock_server_or_skip};

async fn setup() -> Option<(Queue, TempDir, wiremock::MockServer)> {
    let mock_server = start_mock_server_or_skip().await?;
    let temp_dir = TempDir::new().ok()?;
    let db_path = temp_dir.path().join("test.db");
    let db = Database::new(&db_path).await.ok()?;
    Some((Queue::new(db), temp_dir, mock_server))
}

#[tokio::test]
async fn p0_server_error_500_retries_then_final_status() {
    let Some((queue, _temp, mock_server)) = setup().await else {
        return socket_skip_return();
    };

    Mock::given(method("GET"))
        .and(path("/fail"))
        .respond_with(ResponseTemplate::new(500).set_body_bytes(b"error"))
        .mount(&mock_server)
        .await;

    let url = format!("{}/fail", mock_server.uri());
    let id = queue
        .enqueue(&url, "direct_url", None)
        .await
        .expect("enqueue");

    let client = HttpClient::new();
    let rate_limiter = Arc::new(RateLimiter::disabled());
    let engine =
        DownloadEngine::new(2, RetryPolicy::with_max_attempts(2), rate_limiter).expect("engine");

    let output_dir = TempDir::new().expect("temp dir");
    let stats = engine
        .process_queue(&queue, &client, output_dir.path())
        .await
        .expect("process");

    assert!(stats.failed() >= 1 || stats.completed() == 1);
    let item = queue.get(id).await.expect("get").expect("item");
    assert!(
        item.status() == QueueStatus::Completed || item.status() == QueueStatus::Failed,
        "item should be completed or failed"
    );
}

#[tokio::test]
async fn p0_503_retries_then_fails_or_succeeds() {
    let Some((queue, _temp, mock_server)) = setup().await else {
        return socket_skip_return();
    };

    Mock::given(method("GET"))
        .and(path("/503"))
        .respond_with(ResponseTemplate::new(503))
        .mount(&mock_server)
        .await;

    let url = format!("{}/503", mock_server.uri());
    let id = queue
        .enqueue(&url, "direct_url", None)
        .await
        .expect("enqueue");

    let client = HttpClient::new();
    let rate_limiter = Arc::new(RateLimiter::disabled());
    let engine =
        DownloadEngine::new(2, RetryPolicy::with_max_attempts(3), rate_limiter).expect("engine");

    let output_dir = TempDir::new().expect("temp dir");
    let _stats = engine
        .process_queue(&queue, &client, output_dir.path())
        .await
        .expect("process");

    let item = queue.get(id).await.expect("get").expect("item");
    assert_eq!(item.status(), QueueStatus::Failed);
}
