//! Phase 4 (P1): Resource management around download targets.
//! This module currently verifies download-to-valid-dir succeeds; full-disk behavior
//! (out of disk space, clean failure) is not tested here (would require quota, chroot, or mock).

use std::sync::Arc;

use downloader_core::{
    Database, DownloadEngine, HttpClient, Queue, QueueStatus, RateLimiter, RetryPolicy,
};
use tempfile::TempDir;
use wiremock::matchers::{method, path};
use wiremock::{Mock, ResponseTemplate};

use crate::support::socket_guard::{socket_skip_return, start_mock_server_or_skip};

#[tokio::test]
#[ignore] // needs file DB + socket; run with --ignored in nightly
async fn p1_download_to_valid_dir_succeeds() {
    let Some(mock_server) = start_mock_server_or_skip().await else {
        return socket_skip_return();
    };

    Mock::given(method("GET"))
        .and(path("/file"))
        .respond_with(ResponseTemplate::new(200).set_body_bytes(b"content"))
        .mount(&mock_server)
        .await;

    let temp_dir = TempDir::new().expect("temp dir");
    let db_path = temp_dir.path().join("test.db");
    let db = Database::new(&db_path).await.expect("db");
    let queue = Queue::new(db);

    let url = format!("{}/file", mock_server.uri());
    let id = queue
        .enqueue(&url, "direct_url", None)
        .await
        .expect("enqueue");

    let client = HttpClient::new();
    let rate_limiter = Arc::new(RateLimiter::disabled());
    let engine =
        DownloadEngine::new(1, RetryPolicy::with_max_attempts(1), rate_limiter).expect("engine");

    let output_dir = TempDir::new().expect("temp dir");
    let stats = engine
        .process_queue(&queue, &client, output_dir.path())
        .await
        .expect("process");

    assert_eq!(stats.completed(), 1);
    let item = queue.get(id).await.expect("get").expect("item");
    assert_eq!(item.status(), QueueStatus::Completed);
}
