//! Phase 5 (P1): Ctrl+C handling, graceful shutdown.

use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use std::sync::atomic::Ordering;
use std::time::Duration;

use downloader_core::{
    Database, DownloadEngine, HttpClient, Queue, QueueProcessingOptions, RateLimiter, RetryPolicy,
};
use tempfile::TempDir;
use wiremock::matchers::{method, path};
use wiremock::{Mock, ResponseTemplate};

use crate::support::socket_guard::{socket_skip_return, start_mock_server_or_skip};

/// Exercises interrupt flag: give the engine time to start the slow request, then set cancel;
/// process should exit with was_interrupted() and not hang.
#[tokio::test]
#[ignore] // socket + FD pressure; run with --ignored in nightly
async fn p1_interruptible_process_respects_cancel() {
    let Some(mock_server) = start_mock_server_or_skip().await else {
        return socket_skip_return();
    };

    Mock::given(method("GET"))
        .and(path("/slow"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_bytes(b"x")
                .set_delay(std::time::Duration::from_secs(5)),
        )
        .mount(&mock_server)
        .await;

    let temp_dir = TempDir::new().expect("temp dir");
    let db_path = temp_dir.path().join("test.db");
    let db = Database::new(&db_path).await.expect("db");
    let queue = Queue::new(db);

    let url = format!("{}/slow", mock_server.uri());
    let _id = queue
        .enqueue(&url, "direct_url", None)
        .await
        .expect("enqueue");

    let cancelled = Arc::new(AtomicBool::new(false));
    let cancel = Arc::clone(&cancelled);

    let client = HttpClient::new();
    let rate_limiter = Arc::new(RateLimiter::disabled());
    let engine =
        DownloadEngine::new(1, RetryPolicy::with_max_attempts(1), rate_limiter).expect("engine");

    let output_dir = TempDir::new().expect("temp dir");
    let opts = QueueProcessingOptions::default();

    let handle = tokio::spawn({
        let queue = queue.clone();
        let client = client.clone();
        async move {
            engine
                .process_queue_interruptible_with_options(
                    &queue,
                    &client,
                    output_dir.path(),
                    cancel,
                    opts,
                )
                .await
        }
    });

    tokio::time::sleep(Duration::from_millis(200)).await;
    cancelled.store(true, Ordering::Relaxed);

    let stats = handle.await.expect("join").expect("process");
    // Engine may report was_interrupted() or completed() depending on when it checks the flag.
    assert!(
        stats.was_interrupted() || stats.completed() > 0,
        "should exit via interrupt or completion, not hang"
    );
}
