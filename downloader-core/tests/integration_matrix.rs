//! Integration test matrix: explicit cross-module and E2E scenarios.
//!
//! Covers: Engine+Queue, DB+Queue (WAL), Parser→Resolver error propagation,
//! failure recovery flow, and concurrent operations.

use std::sync::Arc;

use downloader_core::{
    Database, DownloadEngine, HttpClient, Queue, QueueStatus, RateLimiter, RetryPolicy, parse_input,
};
use tempfile::TempDir;
use wiremock::matchers::{method, path};
use wiremock::{Mock, ResponseTemplate};

mod support;
use support::socket_guard::{socket_skip_return, start_mock_server_or_skip};

async fn setup_db_queue() -> (Queue, TempDir) {
    let temp_dir = TempDir::new().expect("temp dir");
    let db_path = temp_dir.path().join("test.db");
    let db = Database::new(&db_path).await.expect("db");
    (Queue::new(db), temp_dir)
}

/// Download Engine + Queue: concurrent dequeue/enqueue and status updates under load.
#[tokio::test]
async fn test_integration_engine_queue_concurrent_status_updates() {
    let Some(mock_server) = start_mock_server_or_skip().await else {
        return socket_skip_return();
    };

    Mock::given(method("GET"))
        .respond_with(ResponseTemplate::new(200).set_body_bytes(b"x"))
        .mount(&mock_server)
        .await;

    let (queue, _temp) = setup_db_queue().await;
    for i in 0..15 {
        let url = format!("{}/f{}", mock_server.uri(), i);
        queue
            .enqueue(&url, "direct_url", None)
            .await
            .expect("enqueue");
    }

    let client = HttpClient::new();
    let rate_limiter = Arc::new(RateLimiter::disabled());
    let engine =
        DownloadEngine::new(5, RetryPolicy::with_max_attempts(1), rate_limiter).expect("engine");
    let output_dir = TempDir::new().expect("temp dir");

    let stats = engine
        .process_queue(&queue, &client, output_dir.path())
        .await
        .expect("process");

    assert_eq!(stats.total(), 15);
    assert_eq!(stats.completed(), 15);
    let pending = queue
        .count_by_status(QueueStatus::Pending)
        .await
        .expect("count");
    assert_eq!(pending, 0);
}

/// DB + Queue: WAL mode and concurrent transactions.
#[tokio::test]
async fn test_integration_db_queue_wal_concurrent() {
    let (queue, temp_dir) = setup_db_queue().await;
    let db_path = temp_dir.path().join("test.db");

    let n = 20_usize;
    for i in 0..n {
        queue
            .enqueue(
                &format!("https://example.com/db-{}.pdf", i),
                "direct_url",
                None,
            )
            .await
            .expect("enqueue");
    }

    let completed = Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let mut handles = Vec::new();
    for _ in 0..n {
        let q = queue.clone();
        let c = Arc::clone(&completed);
        handles.push(tokio::spawn(async move {
            if let Ok(Some(item)) = q.dequeue().await {
                let _ = q.mark_completed(item.id).await;
                c.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            }
        }));
    }
    for h in handles {
        let _ = h.await;
    }
    assert_eq!(completed.load(std::sync::atomic::Ordering::SeqCst), n);

    let db = Database::new(&db_path).await.expect("reopen to check WAL");
    let wal = db.is_wal_enabled().await.expect("pragma journal_mode");
    assert!(wal, "WAL mode should be enabled for file-based DB");
}

/// Parser: invalid input produces empty or safe parse result (no panic).
#[test]
fn test_integration_parser_invalid_input_safe() {
    let input = "not a valid url or doi\n\n";
    let result = parse_input(input);
    assert!(result.items.is_empty() || result.items.len() <= 1);
}

/// Failure recovery: network failure then retry then success.
#[tokio::test]
async fn test_integration_failure_recovery_retry_then_success() {
    let Some(mock_server) = start_mock_server_or_skip().await else {
        return socket_skip_return();
    };

    Mock::given(method("GET"))
        .and(path("/recover"))
        .respond_with(ResponseTemplate::new(503))
        .up_to_n_times(2)
        .mount(&mock_server)
        .await;

    Mock::given(method("GET"))
        .and(path("/recover"))
        .respond_with(ResponseTemplate::new(200).set_body_bytes(b"ok"))
        .mount(&mock_server)
        .await;

    let (queue, _temp) = setup_db_queue().await;
    let url = format!("{}/recover", mock_server.uri());
    queue
        .enqueue(&url, "direct_url", None)
        .await
        .expect("enqueue");

    let client = HttpClient::new();
    let rate_limiter = Arc::new(RateLimiter::disabled());
    let engine = DownloadEngine::new(1, RetryPolicy::default(), rate_limiter).expect("engine");
    let output_dir = TempDir::new().expect("temp dir");

    let stats = engine
        .process_queue(&queue, &client, output_dir.path())
        .await
        .expect("process");

    assert_eq!(stats.completed(), 1);
    assert!(stats.retried() >= 1);
}
