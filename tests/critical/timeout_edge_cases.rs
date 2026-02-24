//! Phase 2 (P0): Read timeouts, connection timeouts.
//! Mock slow or hanging response; assert timeout errors and no hang.

use std::sync::Arc;
use std::time::Duration;

use downloader_core::{
    Database, DownloadEngine, HttpClient, Queue, QueueStatus, RateLimiter, RetryPolicy,
};
use tempfile::TempDir;
use wiremock::matchers::{method, path};
use wiremock::{Mock, ResponseTemplate};

use crate::support::socket_guard::{socket_skip_return, start_mock_server_or_skip};

#[tokio::test]
async fn p0_short_read_timeout_fails_gracefully() {
    let Some(mock_server) = start_mock_server_or_skip().await else {
        return socket_skip_return();
    };

    // Delay response longer than client read timeout so client times out
    Mock::given(method("GET"))
        .and(path("/slow"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_bytes(b"x")
                .set_delay(Duration::from_secs(60)),
        )
        .mount(&mock_server)
        .await;

    let temp_dir = TempDir::new().expect("temp dir");
    let db_path = temp_dir.path().join("test.db");
    let db = Database::new(&db_path).await.expect("db");
    let queue = Queue::new(db);

    let url = format!("{}/slow", mock_server.uri());
    let id = queue
        .enqueue(&url, "direct_url", None)
        .await
        .expect("enqueue");

    let client = HttpClient::new_with_timeouts(1, 2);
    let rate_limiter = Arc::new(RateLimiter::disabled());
    let engine =
        DownloadEngine::new(1, RetryPolicy::with_max_attempts(1), rate_limiter).expect("engine");

    let output_dir = TempDir::new().expect("temp dir");
    let stats = engine
        .process_queue(&queue, &client, output_dir.path())
        .await
        .expect("process");

    assert_eq!(stats.failed(), 1);
    let item = queue.get(id).await.expect("get").expect("item");
    assert_eq!(item.status(), QueueStatus::Failed);
}
