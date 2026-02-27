//! Integration tests for the download engine module.
//!
//! These tests verify DownloadEngine with real Queue/Database and mock HTTP server,
//! including retry functionality with exponential backoff.

use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;

use downloader_core::{
    Database, DownloadAttemptQuery, DownloadAttemptStatus, DownloadEngine, DownloadErrorType,
    HttpClient, Queue, QueueMetadata, QueueProcessingOptions, QueueStatus, RateLimiter,
    RetryPolicy,
};
use tempfile::TempDir;
use wiremock::matchers::{method, path};
use wiremock::{Mock, Respond, ResponseTemplate};

mod support;
use support::socket_guard::{socket_skip_return, start_mock_server_or_skip};

macro_rules! require_mock_server {
    () => {{
        let Some(mock_server) = start_mock_server_or_skip().await else {
            return socket_skip_return();
        };
        mock_server
    }};
}

/// Helper to create a test database with migrations applied.
///
/// # Errors
///
/// Returns error if temp directory or database creation fails.
async fn setup_test_db() -> Result<(Database, TempDir), Box<dyn std::error::Error>> {
    let temp_dir = TempDir::new()?;
    let db_path = temp_dir.path().join("test.db");

    let db = Database::new(&db_path).await?;

    Ok((db, temp_dir))
}

// ==================== Helper Functions ====================

/// Helper to create a disabled rate limiter for tests (no delay between requests)
fn test_rate_limiter() -> Arc<RateLimiter> {
    Arc::new(RateLimiter::disabled())
}

/// Helper to create an enabled rate limiter with specified delay
fn test_rate_limiter_with_delay(delay_ms: u64) -> Arc<RateLimiter> {
    Arc::new(RateLimiter::new(Duration::from_millis(delay_ms)))
}

/// Helper to create an engine with default retry policy (for backward compatibility)
fn create_engine(concurrency: usize) -> Result<DownloadEngine, downloader_core::EngineError> {
    DownloadEngine::new(concurrency, RetryPolicy::default(), test_rate_limiter())
}

/// Helper to create an engine with no retries (for tests that don't want retry behavior)
fn create_engine_no_retry(
    concurrency: usize,
) -> Result<DownloadEngine, downloader_core::EngineError> {
    DownloadEngine::new(
        concurrency,
        RetryPolicy::with_max_attempts(1),
        test_rate_limiter(),
    )
}

/// Helper to create an engine with explicit retry policy tuning.
fn create_engine_with_policy(
    concurrency: usize,
    policy: RetryPolicy,
) -> Result<DownloadEngine, downloader_core::EngineError> {
    DownloadEngine::new(concurrency, policy, test_rate_limiter())
}

// ==================== Empty Queue Tests ====================

#[tokio::test]
async fn test_process_queue_empty_returns_zero_stats() -> Result<(), Box<dyn std::error::Error>> {
    let (db, _temp_dir) = setup_test_db().await?;
    let queue = Queue::new(db);

    let client = HttpClient::new();
    let engine = create_engine(10)?;

    let output_dir = TempDir::new()?;
    let stats = engine
        .process_queue(&queue, &client, output_dir.path())
        .await?;

    assert_eq!(stats.completed(), 0);
    assert_eq!(stats.failed(), 0);
    assert_eq!(stats.retried(), 0);
    assert_eq!(stats.total(), 0);
    Ok(())
}

// ==================== Basic Download Tests ====================

#[tokio::test]
async fn test_process_queue_single_item_success() -> Result<(), Box<dyn std::error::Error>> {
    let (db, _temp_dir) = setup_test_db().await?;
    let queue = Queue::new(db);

    // Setup mock server
    let mock_server = require_mock_server!();
    Mock::given(method("GET"))
        .and(path("/file.txt"))
        .respond_with(ResponseTemplate::new(200).set_body_bytes(b"content"))
        .mount(&mock_server)
        .await;

    // Enqueue item
    let url = format!("{}/file.txt", mock_server.uri());
    let id = queue.enqueue(&url, "direct_url", None).await?;

    // Process
    let client = HttpClient::new();
    let engine = create_engine(10)?;
    let output_dir = TempDir::new()?;

    let stats = engine
        .process_queue(&queue, &client, output_dir.path())
        .await?;

    // Verify stats
    assert_eq!(stats.completed(), 1);
    assert_eq!(stats.failed(), 0);
    assert_eq!(stats.total(), 1);

    // Verify queue item marked completed
    let item = queue.get(id).await?.unwrap();
    assert_eq!(item.status(), QueueStatus::Completed);
    Ok(())
}

#[tokio::test]
async fn test_process_queue_interruptible_with_options_generates_sidecar_when_enabled()
-> Result<(), Box<dyn std::error::Error>> {
    use std::sync::atomic::AtomicBool;

    let (db, _temp_dir) = setup_test_db().await?;
    let queue = Queue::new(db);

    let mock_server = require_mock_server!();
    Mock::given(method("GET"))
        .and(path("/paper.pdf"))
        .respond_with(ResponseTemplate::new(200).set_body_bytes(b"pdf-content"))
        .mount(&mock_server)
        .await;

    let url = format!("{}/paper.pdf", mock_server.uri());
    let id = queue.enqueue(&url, "direct_url", None).await?;

    let client = HttpClient::new();
    let engine = create_engine_no_retry(1)?;
    let output_dir = TempDir::new()?;
    let interrupted = Arc::new(AtomicBool::new(false));

    let stats = engine
        .process_queue_interruptible_with_options(
            &queue,
            &client,
            output_dir.path(),
            interrupted,
            QueueProcessingOptions {
                generate_sidecars: true,
                ..QueueProcessingOptions::default()
            },
        )
        .await?;

    assert_eq!(stats.completed(), 1);
    let item = queue.get(id).await?.expect("queued item should exist");
    let saved_path = item
        .saved_path
        .expect("completed item should have saved path");
    let mut sidecar_path = std::path::PathBuf::from(saved_path);
    sidecar_path.set_extension("json");
    assert!(
        sidecar_path.exists(),
        "sidecar should be created when enabled"
    );

    let content = std::fs::read_to_string(&sidecar_path)?;
    assert!(content.contains("\"@context\": \"https://schema.org\""));
    assert!(content.contains("\"@type\": \"ScholarlyArticle\""));
    Ok(())
}

#[tokio::test]
async fn test_process_queue_interruptible_with_options_skips_sidecar_when_disabled()
-> Result<(), Box<dyn std::error::Error>> {
    use std::sync::atomic::AtomicBool;

    let (db, _temp_dir) = setup_test_db().await?;
    let queue = Queue::new(db);

    let mock_server = require_mock_server!();
    Mock::given(method("GET"))
        .and(path("/paper-disabled.pdf"))
        .respond_with(ResponseTemplate::new(200).set_body_bytes(b"pdf-content"))
        .mount(&mock_server)
        .await;

    let url = format!("{}/paper-disabled.pdf", mock_server.uri());
    let id = queue.enqueue(&url, "direct_url", None).await?;

    let client = HttpClient::new();
    let engine = create_engine_no_retry(1)?;
    let output_dir = TempDir::new()?;
    let interrupted = Arc::new(AtomicBool::new(false));

    let stats = engine
        .process_queue_interruptible_with_options(
            &queue,
            &client,
            output_dir.path(),
            interrupted,
            QueueProcessingOptions {
                generate_sidecars: false,
                ..QueueProcessingOptions::default()
            },
        )
        .await?;

    assert_eq!(stats.completed(), 1);
    let item = queue.get(id).await?.expect("queued item should exist");
    let saved_path = item
        .saved_path
        .expect("completed item should have saved path");
    let mut sidecar_path = std::path::PathBuf::from(saved_path);
    sidecar_path.set_extension("json");
    assert!(
        !sidecar_path.exists(),
        "sidecar should not be created when disabled"
    );
    Ok(())
}

#[tokio::test]
async fn test_process_queue_success_writes_download_log_row()
-> Result<(), Box<dyn std::error::Error>> {
    let (db, _temp_dir) = setup_test_db().await?;
    let queue = Queue::new(db);

    let mock_server = require_mock_server!();
    Mock::given(method("GET"))
        .and(path("/logged-success.pdf"))
        .respond_with(ResponseTemplate::new(200).set_body_bytes(b"pdf-bytes"))
        .mount(&mock_server)
        .await;

    let url = format!("{}/logged-success.pdf", mock_server.uri());
    let metadata = QueueMetadata {
        suggested_filename: Some("Logged_2026_Test.pdf".to_string()),
        title: Some("Logged Success".to_string()),
        authors: Some("Author, A".to_string()),
        year: Some("2026".to_string()),
        doi: Some("10.1234/logged".to_string()),
        topics: None,
        parse_confidence: Some("low".to_string()),
        parse_confidence_factors: Some(
            r#"{"has_authors":false,"has_year":true,"has_title":false,"author_count":0}"#
                .to_string(),
        ),
    };
    queue
        .enqueue_with_metadata(&url, "doi", Some("10.1234/logged"), Some(&metadata))
        .await?;

    let client = HttpClient::new();
    let engine = create_engine_no_retry(1)?;
    let output_dir = TempDir::new()?;
    let stats = engine
        .process_queue(&queue, &client, output_dir.path())
        .await?;

    assert_eq!(stats.completed(), 1);

    let rows = queue
        .query_download_attempts(&DownloadAttemptQuery::default())
        .await?;
    assert_eq!(rows.len(), 1, "exactly one history row expected");
    let row = &rows[0];
    assert_eq!(row.status(), DownloadAttemptStatus::Success);
    assert_eq!(row.url, url);
    assert_eq!(row.title.as_deref(), Some("Logged Success"));
    assert_eq!(row.authors.as_deref(), Some("Author, A"));
    assert_eq!(row.doi.as_deref(), Some("10.1234/logged"));
    assert_eq!(row.parse_confidence.as_deref(), Some("low"));
    assert_eq!(
        row.parse_confidence_factors.as_deref(),
        Some(r#"{"has_authors":false,"has_year":true,"has_title":false,"author_count":0}"#)
    );
    assert!(
        row.file_path
            .as_deref()
            .is_some_and(|value| value.ends_with(".pdf")),
        "success row should persist file_path"
    );

    Ok(())
}

#[tokio::test]
async fn test_process_queue_single_item_failure() -> Result<(), Box<dyn std::error::Error>> {
    let (db, _temp_dir) = setup_test_db().await?;
    let queue = Queue::new(db);

    // Setup mock server that returns 404 (permanent error - no retry)
    let mock_server = require_mock_server!();
    Mock::given(method("GET"))
        .and(path("/not-found.txt"))
        .respond_with(ResponseTemplate::new(404))
        .expect(1) // Should only be called once (no retry for 404)
        .mount(&mock_server)
        .await;

    // Enqueue item
    let url = format!("{}/not-found.txt", mock_server.uri());
    let id = queue.enqueue(&url, "direct_url", None).await?;

    // Process
    let client = HttpClient::new();
    let engine = create_engine(10)?;
    let output_dir = TempDir::new()?;

    let stats = engine
        .process_queue(&queue, &client, output_dir.path())
        .await?;

    // Verify stats
    assert_eq!(stats.completed(), 0);
    assert_eq!(stats.failed(), 1);
    assert_eq!(stats.retried(), 0); // 404 is permanent, no retries
    assert_eq!(stats.total(), 1);

    // Verify queue item marked failed
    let item = queue.get(id).await?.unwrap();
    assert_eq!(item.status(), QueueStatus::Failed);
    assert!(item.last_error.is_some());
    Ok(())
}

#[tokio::test]
async fn test_process_queue_failure_writes_download_log_row()
-> Result<(), Box<dyn std::error::Error>> {
    let (db, _temp_dir) = setup_test_db().await?;
    let queue = Queue::new(db);

    let mock_server = require_mock_server!();
    Mock::given(method("GET"))
        .and(path("/logged-failure.pdf"))
        .respond_with(ResponseTemplate::new(404))
        .mount(&mock_server)
        .await;

    let url = format!("{}/logged-failure.pdf", mock_server.uri());
    queue.enqueue(&url, "direct_url", None).await?;

    let client = HttpClient::new();
    let engine = create_engine_no_retry(1)?;
    let output_dir = TempDir::new()?;
    let stats = engine
        .process_queue(&queue, &client, output_dir.path())
        .await?;

    assert_eq!(stats.failed(), 1);

    let mut query = DownloadAttemptQuery::default();
    query.status = Some(DownloadAttemptStatus::Failed);
    let rows = queue.query_download_attempts(&query).await?;
    assert_eq!(rows.len(), 1, "one failed history row expected");
    let row = &rows[0];
    assert_eq!(row.status(), DownloadAttemptStatus::Failed);
    assert_eq!(row.url, url);
    assert!(
        row.file_path.is_none(),
        "failed row should not have file path"
    );
    assert!(
        row.error_message
            .as_deref()
            .is_some_and(|value| value.contains("HTTP 404")),
        "failed row should capture HTTP error"
    );
    assert_eq!(
        row.http_status,
        Some(404),
        "failed row should capture HTTP status code"
    );
    assert_eq!(
        row.error_type(),
        Some(DownloadErrorType::NotFound),
        "failed row should categorize 404 as not_found"
    );
    assert_eq!(row.retry_count, 0, "404 should not be retried");
    assert!(
        row.last_retry_at.is_none(),
        "no retries means no last_retry_at timestamp"
    );
    assert_eq!(
        row.original_input.as_deref(),
        Some(url.as_str()),
        "original input should fallback to source URL when queue input is direct"
    );

    Ok(())
}

#[tokio::test]
async fn test_process_queue_failure_propagates_parse_confidence_to_download_log()
-> Result<(), Box<dyn std::error::Error>> {
    let (db, _temp_dir) = setup_test_db().await?;
    let queue = Queue::new(db);

    let mock_server = require_mock_server!();
    Mock::given(method("GET"))
        .and(path("/reference-failure.pdf"))
        .respond_with(ResponseTemplate::new(404))
        .mount(&mock_server)
        .await;

    let url = format!("{}/reference-failure.pdf", mock_server.uri());
    let metadata = QueueMetadata {
        suggested_filename: Some("Reference_Failure.pdf".to_string()),
        title: Some("Reference Failure".to_string()),
        authors: Some("Author, B".to_string()),
        year: Some("2026".to_string()),
        doi: None,
        topics: None,
        parse_confidence: Some("low".to_string()),
        parse_confidence_factors: Some(
            r#"{"has_authors":false,"has_year":true,"has_title":false,"author_count":0}"#
                .to_string(),
        ),
    };
    queue
        .enqueue_with_metadata(&url, "reference", Some("Weak reference"), Some(&metadata))
        .await?;

    let client = HttpClient::new();
    let engine = create_engine_no_retry(1)?;
    let output_dir = TempDir::new()?;
    let stats = engine
        .process_queue(&queue, &client, output_dir.path())
        .await?;
    assert_eq!(stats.failed(), 1);

    let mut query = DownloadAttemptQuery::default();
    query.status = Some(DownloadAttemptStatus::Failed);
    let rows = queue.query_download_attempts(&query).await?;
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].parse_confidence.as_deref(), Some("low"));
    assert_eq!(
        rows[0].parse_confidence_factors.as_deref(),
        Some(r#"{"has_authors":false,"has_year":true,"has_title":false,"author_count":0}"#)
    );

    Ok(())
}

#[tokio::test]
async fn test_process_queue_failure_auth_classification() -> Result<(), Box<dyn std::error::Error>>
{
    let (db, _temp_dir) = setup_test_db().await?;
    let queue = Queue::new(db);

    let mock_server = require_mock_server!();
    Mock::given(method("GET"))
        .and(path("/auth-required.pdf"))
        .respond_with(ResponseTemplate::new(401))
        .mount(&mock_server)
        .await;

    let url = format!("{}/auth-required.pdf", mock_server.uri());
    queue.enqueue(&url, "direct_url", None).await?;

    let client = HttpClient::new();
    let engine = create_engine_no_retry(1)?;
    let output_dir = TempDir::new()?;
    let stats = engine
        .process_queue(&queue, &client, output_dir.path())
        .await?;
    assert_eq!(stats.failed(), 1);

    let mut query = DownloadAttemptQuery::default();
    query.status = Some(DownloadAttemptStatus::Failed);
    let rows = queue.query_download_attempts(&query).await?;
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].error_type(), Some(DownloadErrorType::Auth));
    assert!(
        rows[0]
            .error_message
            .as_deref()
            .is_some_and(|value| value.contains("Suggestion:")),
        "auth failure row should include actionable suggestion"
    );

    Ok(())
}

#[tokio::test]
async fn test_process_queue_failure_network_classification()
-> Result<(), Box<dyn std::error::Error>> {
    let (db, _temp_dir) = setup_test_db().await?;
    let queue = Queue::new(db);

    let url = "http://127.0.0.1:1/network-failure.pdf";
    queue.enqueue(url, "direct_url", None).await?;

    let client = HttpClient::new();
    let engine = create_engine_no_retry(1)?;
    let output_dir = TempDir::new()?;
    let stats = engine
        .process_queue(&queue, &client, output_dir.path())
        .await?;
    assert_eq!(stats.failed(), 1);

    let mut query = DownloadAttemptQuery::default();
    query.status = Some(DownloadAttemptStatus::Failed);
    let rows = queue.query_download_attempts(&query).await?;
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].error_type(), Some(DownloadErrorType::Network));

    Ok(())
}

#[tokio::test]
async fn test_process_queue_failure_persists_retry_count_and_last_retry_at()
-> Result<(), Box<dyn std::error::Error>> {
    let (db, _temp_dir) = setup_test_db().await?;
    let queue = Queue::new(db);

    let mock_server = require_mock_server!();
    Mock::given(method("GET"))
        .and(path("/retry-fail.pdf"))
        .respond_with(ResponseTemplate::new(503))
        .expect(2)
        .mount(&mock_server)
        .await;

    let url = format!("{}/retry-fail.pdf", mock_server.uri());
    queue.enqueue(&url, "direct_url", None).await?;

    let client = HttpClient::new();
    let retry_policy = RetryPolicy::new(2, Duration::from_millis(1), Duration::from_millis(1), 1.0);
    let engine = create_engine_with_policy(1, retry_policy)?;
    let output_dir = TempDir::new()?;
    let stats = engine
        .process_queue(&queue, &client, output_dir.path())
        .await?;
    assert_eq!(stats.failed(), 1);
    assert_eq!(stats.retried(), 1);

    let mut query = DownloadAttemptQuery::default();
    query.status = Some(DownloadAttemptStatus::Failed);
    let rows = queue.query_download_attempts(&query).await?;
    assert_eq!(rows.len(), 1);
    let row = &rows[0];
    assert_eq!(row.retry_count, 1);
    assert!(
        row.last_retry_at.is_some(),
        "failed row with retries should persist last_retry_at"
    );

    Ok(())
}

#[tokio::test]
async fn test_process_queue_failure_preserves_original_input_for_doi()
-> Result<(), Box<dyn std::error::Error>> {
    let (db, _temp_dir) = setup_test_db().await?;
    let queue = Queue::new(db);

    let mock_server = require_mock_server!();
    Mock::given(method("GET"))
        .and(path("/doi-failure.pdf"))
        .respond_with(ResponseTemplate::new(404))
        .mount(&mock_server)
        .await;

    let url = format!("{}/doi-failure.pdf", mock_server.uri());
    queue
        .enqueue(&url, "doi", Some("10.4242/original-doi"))
        .await?;

    let client = HttpClient::new();
    let engine = create_engine_no_retry(1)?;
    let output_dir = TempDir::new()?;
    let stats = engine
        .process_queue(&queue, &client, output_dir.path())
        .await?;
    assert_eq!(stats.failed(), 1);

    let mut query = DownloadAttemptQuery::default();
    query.status = Some(DownloadAttemptStatus::Failed);
    let rows = queue.query_download_attempts(&query).await?;
    assert_eq!(rows.len(), 1);
    assert_eq!(
        rows[0].original_input.as_deref(),
        Some("10.4242/original-doi")
    );

    Ok(())
}

// ==================== Mixed Success/Failure Tests ====================

#[tokio::test]
async fn test_process_queue_mixed_success_and_failure() -> Result<(), Box<dyn std::error::Error>> {
    let (db, _temp_dir) = setup_test_db().await?;
    let queue = Queue::new(db);

    // Setup mock server
    let mock_server = require_mock_server!();

    // 3 successful endpoints
    for i in 1..=3 {
        Mock::given(method("GET"))
            .and(path(format!("/success{}.txt", i)))
            .respond_with(
                ResponseTemplate::new(200).set_body_bytes(format!("content{}", i).as_bytes()),
            )
            .mount(&mock_server)
            .await;
    }

    // 2 failing endpoints - use 404 (permanent) to avoid retries slowing test
    for i in 1..=2 {
        Mock::given(method("GET"))
            .and(path(format!("/fail{}.txt", i)))
            .respond_with(ResponseTemplate::new(404))
            .mount(&mock_server)
            .await;
    }

    // Enqueue 3 success + 2 failure items
    let mut success_ids = Vec::new();
    let mut fail_ids = Vec::new();

    for i in 1..=3 {
        let url = format!("{}/success{}.txt", mock_server.uri(), i);
        let id = queue.enqueue(&url, "direct_url", None).await?;
        success_ids.push(id);
    }

    for i in 1..=2 {
        let url = format!("{}/fail{}.txt", mock_server.uri(), i);
        let id = queue.enqueue(&url, "direct_url", None).await?;
        fail_ids.push(id);
    }

    // Process with no-retry engine to keep test fast
    let client = HttpClient::new();
    let engine = create_engine_no_retry(10)?;
    let output_dir = TempDir::new()?;

    let stats = engine
        .process_queue(&queue, &client, output_dir.path())
        .await?;

    // Verify stats invariant: completed + failed = total
    assert_eq!(stats.completed(), 3);
    assert_eq!(stats.failed(), 2);
    assert_eq!(stats.total(), 5);
    assert_eq!(stats.completed() + stats.failed(), stats.total());

    // Verify all success items marked completed
    for id in success_ids {
        let item = queue.get(id).await?.unwrap();
        assert_eq!(item.status(), QueueStatus::Completed);
    }

    // Verify all failed items marked failed
    for id in fail_ids {
        let item = queue.get(id).await?.unwrap();
        assert_eq!(item.status(), QueueStatus::Failed);
    }
    Ok(())
}

// ==================== Concurrency Limit Tests ====================

/// Responder that tracks peak concurrent requests using atomic counters.
/// Uses a blocking sleep to ensure requests overlap for accurate measurement.
///
/// # Note on blocking sleep
///
/// We use `std::thread::sleep` here instead of `tokio::time::sleep` because:
/// 1. wiremock's `Respond` trait is synchronous (not async)
/// 2. We need the delay to happen DURING request processing to accurately measure
///    concurrent in-flight requests
/// 3. This is test-only code in an external crate's callback context
///
/// This is an exception to the project rule "never block async runtime with std::thread::sleep"
/// because the wiremock server runs in its own thread pool, not the main tokio runtime.
struct ConcurrencyTrackingResponder {
    current: Arc<AtomicUsize>,
    peak: Arc<AtomicUsize>,
    delay_ms: u64,
}

impl ConcurrencyTrackingResponder {
    fn new(current: Arc<AtomicUsize>, peak: Arc<AtomicUsize>, delay_ms: u64) -> Self {
        Self {
            current,
            peak,
            delay_ms,
        }
    }
}

impl Respond for ConcurrencyTrackingResponder {
    fn respond(&self, _request: &wiremock::Request) -> ResponseTemplate {
        // Increment current concurrent count at request start
        let prev = self.current.fetch_add(1, Ordering::SeqCst);
        let current_count = prev + 1;

        // Update peak if we have a new maximum
        self.peak.fetch_max(current_count, Ordering::SeqCst);

        // Use blocking sleep to ensure requests overlap.
        // This keeps the "concurrent" counter elevated while other requests arrive.
        // NOTE: std::thread::sleep is intentional here - see struct-level doc comment.
        std::thread::sleep(Duration::from_millis(self.delay_ms));

        // Decrement at end of request processing
        self.current.fetch_sub(1, Ordering::SeqCst);

        ResponseTemplate::new(200).set_body_bytes(b"content")
    }
}

#[tokio::test]
async fn test_semaphore_limits_concurrent_downloads() -> Result<(), Box<dyn std::error::Error>> {
    let (db, _temp_dir) = setup_test_db().await?;
    let queue = Queue::new(db);

    // Atomic counters for tracking concurrency
    let current = Arc::new(AtomicUsize::new(0));
    let peak = Arc::new(AtomicUsize::new(0));

    // Setup mock server with concurrency-tracking responder
    let mock_server = require_mock_server!();

    // Use a longer delay (100ms) to ensure overlap between requests
    let responder = ConcurrencyTrackingResponder::new(
        Arc::clone(&current),
        Arc::clone(&peak),
        100, // 100ms delay
    );

    Mock::given(method("GET"))
        .respond_with(responder)
        .mount(&mock_server)
        .await;

    // Enqueue 10 items to ensure we'd hit the limit with enough headroom
    for i in 0..10 {
        let url = format!("{}/file{}.txt", mock_server.uri(), i);
        queue.enqueue(&url, "direct_url", None).await?;
    }

    // Create engine with concurrency limit of 3 (no retry to keep test fast)
    let client = HttpClient::new();
    let engine = create_engine_no_retry(3)?;
    let output_dir = TempDir::new()?;

    let stats = engine
        .process_queue(&queue, &client, output_dir.path())
        .await?;

    // Verify all items processed
    assert_eq!(stats.total(), 10);

    // Verify semaphore limited concurrency - this is the critical assertion
    let observed_peak = peak.load(Ordering::SeqCst);
    assert!(
        observed_peak <= 3,
        "Peak concurrency {} should not exceed semaphore limit of 3",
        observed_peak
    );

    // Note: We intentionally don't assert a minimum peak concurrency because:
    // - The exact timing depends on thread scheduling
    // - The important invariant is that we NEVER exceed the limit
    // - Reaching the limit is nice-to-have but timing-dependent
    Ok(())
}

// ==================== Error Isolation Tests ====================

#[tokio::test]
async fn test_one_download_failure_does_not_affect_others() -> Result<(), Box<dyn std::error::Error>>
{
    let (db, _temp_dir) = setup_test_db().await?;
    let queue = Queue::new(db);

    // Setup mock server
    let mock_server = require_mock_server!();

    // First item will fail
    Mock::given(method("GET"))
        .and(path("/fail.txt"))
        .respond_with(ResponseTemplate::new(500))
        .mount(&mock_server)
        .await;

    // Second and third items will succeed
    Mock::given(method("GET"))
        .and(path("/success1.txt"))
        .respond_with(ResponseTemplate::new(200).set_body_bytes(b"content1"))
        .mount(&mock_server)
        .await;

    Mock::given(method("GET"))
        .and(path("/success2.txt"))
        .respond_with(ResponseTemplate::new(200).set_body_bytes(b"content2"))
        .mount(&mock_server)
        .await;

    // Enqueue: fail first, then two successes
    let fail_url = format!("{}/fail.txt", mock_server.uri());
    let success1_url = format!("{}/success1.txt", mock_server.uri());
    let success2_url = format!("{}/success2.txt", mock_server.uri());

    let fail_id = queue.enqueue(&fail_url, "direct_url", None).await?;
    let success1_id = queue.enqueue(&success1_url, "direct_url", None).await?;
    let success2_id = queue.enqueue(&success2_url, "direct_url", None).await?;

    // Process - use 404 for fail so it doesn't retry
    let client = HttpClient::new();
    let engine = create_engine_no_retry(10)?;
    let output_dir = TempDir::new()?;

    let stats = engine
        .process_queue(&queue, &client, output_dir.path())
        .await?;

    // Verify stats
    assert_eq!(stats.completed(), 2);
    assert_eq!(stats.failed(), 1);

    // Verify individual statuses
    let fail_item = queue.get(fail_id).await?.unwrap();
    assert_eq!(fail_item.status(), QueueStatus::Failed);

    let success1_item = queue.get(success1_id).await?.unwrap();
    assert_eq!(success1_item.status(), QueueStatus::Completed);

    let success2_item = queue.get(success2_id).await?.unwrap();
    assert_eq!(success2_item.status(), QueueStatus::Completed);
    Ok(())
}

#[tokio::test]
async fn test_all_items_reach_terminal_state() -> Result<(), Box<dyn std::error::Error>> {
    let (db, _temp_dir) = setup_test_db().await?;
    let queue = Queue::new(db);

    // Setup mock with varied responses - use 404 for failures to avoid retries
    let mock_server = require_mock_server!();

    Mock::given(method("GET"))
        .and(path("/ok.txt"))
        .respond_with(ResponseTemplate::new(200).set_body_bytes(b"ok"))
        .mount(&mock_server)
        .await;

    Mock::given(method("GET"))
        .and(path("/not-found.txt"))
        .respond_with(ResponseTemplate::new(404))
        .mount(&mock_server)
        .await;

    Mock::given(method("GET"))
        .and(path("/forbidden.txt"))
        .respond_with(ResponseTemplate::new(403))
        .mount(&mock_server)
        .await;

    // Enqueue items
    let id1 = queue
        .enqueue(&format!("{}/ok.txt", mock_server.uri()), "direct_url", None)
        .await?;
    let id2 = queue
        .enqueue(
            &format!("{}/not-found.txt", mock_server.uri()),
            "direct_url",
            None,
        )
        .await?;
    let id3 = queue
        .enqueue(
            &format!("{}/forbidden.txt", mock_server.uri()),
            "direct_url",
            None,
        )
        .await?;
    let ids = vec![id1, id2, id3];

    // Process with no-retry engine to keep test fast
    let client = HttpClient::new();
    let engine = create_engine_no_retry(10)?;
    let output_dir = TempDir::new()?;

    engine
        .process_queue(&queue, &client, output_dir.path())
        .await?;

    // Verify all items reached terminal state (Completed or Failed)
    for id in ids {
        let item = queue.get(id).await?.unwrap();
        let status = item.status();
        assert!(
            status == QueueStatus::Completed || status == QueueStatus::Failed,
            "Item {} should be in terminal state, but was {:?}",
            id,
            status
        );
    }
    Ok(())
}

// ==================== Status Independence Tests ====================

#[tokio::test]
async fn test_status_updates_dont_interfere_with_each_other()
-> Result<(), Box<dyn std::error::Error>> {
    let (db, _temp_dir) = setup_test_db().await?;
    let queue = Queue::new(db);

    // Setup mock server with delays to ensure concurrent updates
    let mock_server = require_mock_server!();

    // Multiple items with varying response times
    for i in 0..5 {
        let delay = Duration::from_millis(10 * (i as u64 + 1));
        Mock::given(method("GET"))
            .and(path(format!("/item{}.txt", i)))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_bytes(format!("content{}", i).as_bytes())
                    .set_delay(delay),
            )
            .mount(&mock_server)
            .await;
    }

    // Enqueue all items
    let mut ids = Vec::new();
    for i in 0..5 {
        let url = format!("{}/item{}.txt", mock_server.uri(), i);
        let id = queue.enqueue(&url, "direct_url", None).await?;
        ids.push(id);
    }

    // Process with high concurrency to maximize overlap
    let client = HttpClient::new();
    let engine = create_engine_no_retry(5)?;
    let output_dir = TempDir::new()?;

    let stats = engine
        .process_queue(&queue, &client, output_dir.path())
        .await?;

    // All should complete successfully
    assert_eq!(stats.completed(), 5);
    assert_eq!(stats.failed(), 0);

    // Verify each item has correct status
    for id in ids {
        let item = queue.get(id).await?.unwrap();
        assert_eq!(
            item.status(),
            QueueStatus::Completed,
            "Item {} should be completed",
            id
        );
    }
    Ok(())
}

// ==================== Retry Behavior Tests ====================

#[tokio::test]
async fn test_retry_succeeds_after_transient_failure() -> Result<(), Box<dyn std::error::Error>> {
    let (db, _temp_dir) = setup_test_db().await?;
    let queue = Queue::new(db);

    let mock_server = require_mock_server!();

    // First request returns 503 (transient), second returns 200 (success)
    Mock::given(method("GET"))
        .and(path("/paper.pdf"))
        .respond_with(ResponseTemplate::new(503))
        .up_to_n_times(1) // Matches exactly once, then falls through
        .mount(&mock_server)
        .await;

    Mock::given(method("GET"))
        .and(path("/paper.pdf"))
        .respond_with(ResponseTemplate::new(200).set_body_bytes(b"PDF content"))
        .mount(&mock_server)
        .await;

    // Enqueue item
    let url = format!("{}/paper.pdf", mock_server.uri());
    let id = queue.enqueue(&url, "direct_url", None).await?;

    // Process with retry enabled
    let client = HttpClient::new();
    let engine = create_engine(10)?;
    let output_dir = TempDir::new()?;

    let stats = engine
        .process_queue(&queue, &client, output_dir.path())
        .await?;

    // Should succeed after retry
    assert_eq!(stats.completed(), 1);
    assert_eq!(stats.failed(), 0);
    assert_eq!(stats.retried(), 1); // One retry attempt

    // Verify item marked completed
    let item = queue.get(id).await?.unwrap();
    assert_eq!(item.status(), QueueStatus::Completed);
    Ok(())
}

#[tokio::test]
async fn test_permanent_error_does_not_retry() -> Result<(), Box<dyn std::error::Error>> {
    let (db, _temp_dir) = setup_test_db().await?;
    let queue = Queue::new(db);

    let mock_server = require_mock_server!();

    // 404 is permanent - should NOT retry
    Mock::given(method("GET"))
        .and(path("/not-found.pdf"))
        .respond_with(ResponseTemplate::new(404))
        .expect(1) // Should only be called once
        .mount(&mock_server)
        .await;

    let url = format!("{}/not-found.pdf", mock_server.uri());
    let id = queue.enqueue(&url, "direct_url", None).await?;

    let client = HttpClient::new();
    let engine = create_engine(10)?;
    let output_dir = TempDir::new()?;

    let stats = engine
        .process_queue(&queue, &client, output_dir.path())
        .await?;

    assert_eq!(stats.completed(), 0);
    assert_eq!(stats.failed(), 1);
    assert_eq!(stats.retried(), 0); // No retries for permanent errors

    let item = queue.get(id).await?.unwrap();
    assert_eq!(item.status(), QueueStatus::Failed);
    Ok(())
}

#[tokio::test]
async fn test_401_does_not_retry() -> Result<(), Box<dyn std::error::Error>> {
    let (db, _temp_dir) = setup_test_db().await?;
    let queue = Queue::new(db);

    let mock_server = require_mock_server!();

    // 401 is NeedsAuth - should NOT retry (until Epic 4)
    Mock::given(method("GET"))
        .and(path("/protected.pdf"))
        .respond_with(ResponseTemplate::new(401))
        .expect(1) // Should only be called once
        .mount(&mock_server)
        .await;

    let url = format!("{}/protected.pdf", mock_server.uri());
    let id = queue.enqueue(&url, "direct_url", None).await?;

    let client = HttpClient::new();
    let engine = create_engine(10)?;
    let output_dir = TempDir::new()?;

    let stats = engine
        .process_queue(&queue, &client, output_dir.path())
        .await?;

    assert_eq!(stats.completed(), 0);
    assert_eq!(stats.failed(), 1);
    assert_eq!(stats.retried(), 0);

    let item = queue.get(id).await?.unwrap();
    assert_eq!(item.status(), QueueStatus::Failed);
    // Verify the error message has [AUTH] prefix
    let err_msg = item.last_error.as_deref().unwrap_or("");
    assert!(
        err_msg.starts_with("[AUTH]"),
        "Expected [AUTH] prefix in error, got: {err_msg}"
    );
    Ok(())
}

#[tokio::test]
async fn test_403_does_not_retry() -> Result<(), Box<dyn std::error::Error>> {
    let (db, _temp_dir) = setup_test_db().await?;
    let queue = Queue::new(db);

    let mock_server = require_mock_server!();

    // 403 triggers a single browser User-Agent fallback retry before giving up.
    Mock::given(method("GET"))
        .and(path("/forbidden.pdf"))
        .respond_with(ResponseTemplate::new(403))
        .expect(2) // Initial request + one browser UA retry
        .mount(&mock_server)
        .await;

    let url = format!("{}/forbidden.pdf", mock_server.uri());
    let id = queue.enqueue(&url, "direct_url", None).await?;

    let client = HttpClient::new();
    let engine = create_engine(10)?;
    let output_dir = TempDir::new()?;

    let stats = engine
        .process_queue(&queue, &client, output_dir.path())
        .await?;

    assert_eq!(stats.completed(), 0);
    assert_eq!(stats.failed(), 1);
    assert_eq!(stats.retried(), 1); // one browser UA retry

    let item = queue.get(id).await?.unwrap();
    assert_eq!(item.status(), QueueStatus::Failed);
    // 403 now produces AuthRequired with [AUTH] prefix
    let err_msg = item.last_error.as_deref().unwrap_or("");
    assert!(
        err_msg.starts_with("[AUTH]"),
        "Expected [AUTH] prefix in 403 error, got: {err_msg}"
    );
    Ok(())
}

#[tokio::test]
async fn test_429_triggers_retry_with_backoff() -> Result<(), Box<dyn std::error::Error>> {
    let (db, _temp_dir) = setup_test_db().await?;
    let queue = Queue::new(db);

    let mock_server = require_mock_server!();

    // First request returns 429 (rate limited), second returns 200 (success)
    Mock::given(method("GET"))
        .and(path("/rate-limited.pdf"))
        .respond_with(ResponseTemplate::new(429))
        .up_to_n_times(1)
        .mount(&mock_server)
        .await;

    Mock::given(method("GET"))
        .and(path("/rate-limited.pdf"))
        .respond_with(ResponseTemplate::new(200).set_body_bytes(b"PDF content"))
        .mount(&mock_server)
        .await;

    let url = format!("{}/rate-limited.pdf", mock_server.uri());
    let id = queue.enqueue(&url, "direct_url", None).await?;

    let client = HttpClient::new();
    let engine = create_engine(10)?;
    let output_dir = TempDir::new()?;

    let stats = engine
        .process_queue(&queue, &client, output_dir.path())
        .await?;

    // Should succeed after retry
    assert_eq!(stats.completed(), 1);
    assert_eq!(stats.failed(), 0);
    assert_eq!(stats.retried(), 1);

    let item = queue.get(id).await?.unwrap();
    assert_eq!(item.status(), QueueStatus::Completed);
    Ok(())
}

#[tokio::test]
async fn test_429_respects_retry_after_header() -> Result<(), Box<dyn std::error::Error>> {
    let (db, _temp_dir) = setup_test_db().await?;
    let queue = Queue::new(db);

    let mock_server = require_mock_server!();

    // First request returns 429 with Retry-After header
    Mock::given(method("GET"))
        .and(path("/retry-after.pdf"))
        .respond_with(
            ResponseTemplate::new(429).insert_header("Retry-After", "1"), // 1 second delay
        )
        .up_to_n_times(1)
        .mount(&mock_server)
        .await;

    // Second request succeeds
    Mock::given(method("GET"))
        .and(path("/retry-after.pdf"))
        .respond_with(ResponseTemplate::new(200).set_body_bytes(b"PDF content"))
        .mount(&mock_server)
        .await;

    let url = format!("{}/retry-after.pdf", mock_server.uri());
    let id = queue.enqueue(&url, "direct_url", None).await?;

    let client = HttpClient::new();
    let engine = create_engine(10)?;
    let output_dir = TempDir::new()?;

    let start = std::time::Instant::now();
    let stats = engine
        .process_queue(&queue, &client, output_dir.path())
        .await?;
    let elapsed = start.elapsed();

    // Should succeed after retry
    assert_eq!(stats.completed(), 1);
    assert_eq!(stats.failed(), 0);
    assert_eq!(stats.retried(), 1);

    // Should have waited at least ~1 second (Retry-After value)
    // Allow some tolerance for test execution overhead
    assert!(
        elapsed >= Duration::from_millis(900),
        "Should have waited for Retry-After delay, elapsed: {:?}",
        elapsed
    );

    let item = queue.get(id).await?.unwrap();
    assert_eq!(item.status(), QueueStatus::Completed);
    Ok(())
}

#[tokio::test]
async fn test_max_retries_exhausted_marks_item_failed() -> Result<(), Box<dyn std::error::Error>> {
    let (db, _temp_dir) = setup_test_db().await?;
    let queue = Queue::new(db);

    let mock_server = require_mock_server!();

    // Always return 503 (transient) - should retry until max_attempts
    Mock::given(method("GET"))
        .and(path("/always-503.pdf"))
        .respond_with(ResponseTemplate::new(503))
        .mount(&mock_server)
        .await;

    let url = format!("{}/always-503.pdf", mock_server.uri());
    let id = queue.enqueue(&url, "direct_url", None).await?;

    let client = HttpClient::new();
    // Use policy with 2 max attempts (1 initial + 1 retry)
    let policy = RetryPolicy::with_max_attempts(2);
    let engine = DownloadEngine::new(10, policy, test_rate_limiter())?;
    let output_dir = TempDir::new()?;

    let stats = engine
        .process_queue(&queue, &client, output_dir.path())
        .await?;

    // Should fail after exhausting retries
    assert_eq!(stats.completed(), 0);
    assert_eq!(stats.failed(), 1);
    assert_eq!(stats.retried(), 1); // 1 retry (2 attempts total)

    let item = queue.get(id).await?.unwrap();
    assert_eq!(item.status(), QueueStatus::Failed);
    assert!(item.last_error.is_some());
    Ok(())
}

#[tokio::test]
async fn test_retry_count_persisted_in_database() -> Result<(), Box<dyn std::error::Error>> {
    let (db, _temp_dir) = setup_test_db().await?;
    let queue = Queue::new(db);

    let mock_server = require_mock_server!();

    // Always return 503 - retry until exhausted
    Mock::given(method("GET"))
        .and(path("/fail.pdf"))
        .respond_with(ResponseTemplate::new(503))
        .mount(&mock_server)
        .await;

    let url = format!("{}/fail.pdf", mock_server.uri());
    let id = queue.enqueue(&url, "direct_url", None).await?;

    let client = HttpClient::new();
    // 3 max attempts means 2 retries
    let policy = RetryPolicy::with_max_attempts(3);
    let engine = DownloadEngine::new(10, policy, test_rate_limiter())?;
    let output_dir = TempDir::new()?;

    let stats = engine
        .process_queue(&queue, &client, output_dir.path())
        .await?;

    assert_eq!(stats.failed(), 1);
    assert_eq!(stats.retried(), 2); // 2 retries (3 attempts total)

    // Verify actual retry_count persisted in database
    let item = queue.get(id).await?.unwrap();
    assert_eq!(item.status(), QueueStatus::Failed);
    assert_eq!(item.retry_count, 2); // 3 attempts total => 2 retries
    Ok(())
}

// ==================== Rate Limiting Tests ====================

#[tokio::test]
async fn test_rate_limiter_delays_same_domain_requests() -> Result<(), Box<dyn std::error::Error>> {
    let (db, _temp_dir) = setup_test_db().await?;
    let queue = Queue::new(db);

    let mock_server = require_mock_server!();

    // Setup 3 endpoints on the same domain
    for i in 1..=3 {
        Mock::given(method("GET"))
            .and(path(format!("/file{}.txt", i)))
            .respond_with(ResponseTemplate::new(200).set_body_bytes(b"content"))
            .mount(&mock_server)
            .await;
    }

    // Enqueue 3 items to same domain
    for i in 1..=3 {
        let url = format!("{}/file{}.txt", mock_server.uri(), i);
        queue.enqueue(&url, "direct_url", None).await?;
    }

    let client = HttpClient::new();
    // Use rate limiter with 50ms delay (short but measurable)
    let rate_limiter = test_rate_limiter_with_delay(50);
    let engine = DownloadEngine::new(10, RetryPolicy::with_max_attempts(1), rate_limiter)?;
    let output_dir = TempDir::new()?;

    let start = std::time::Instant::now();
    let stats = engine
        .process_queue(&queue, &client, output_dir.path())
        .await?;
    let elapsed = start.elapsed();

    // All should complete
    assert_eq!(stats.completed(), 3);
    assert_eq!(stats.failed(), 0);

    // With 3 requests and 50ms delay, we should see at least 100ms total
    // (first request is immediate, 2nd waits 50ms, 3rd waits 50ms)
    assert!(
        elapsed >= Duration::from_millis(80), // Allow some tolerance
        "Rate limiting should delay same-domain requests, elapsed: {:?}",
        elapsed
    );

    Ok(())
}

#[tokio::test]
async fn test_rate_limiter_disabled_allows_fast_parallel() -> Result<(), Box<dyn std::error::Error>>
{
    let (db, _temp_dir) = setup_test_db().await?;
    let queue = Queue::new(db);

    let mock_server = require_mock_server!();

    // Setup 3 endpoints with 50ms delay each
    for i in 1..=3 {
        Mock::given(method("GET"))
            .and(path(format!("/fast{}.txt", i)))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_bytes(b"content")
                    .set_delay(Duration::from_millis(50)),
            )
            .mount(&mock_server)
            .await;
    }

    // Enqueue 3 items (same domain but rate limiting disabled)
    for i in 1..=3 {
        let url = format!("{}/fast{}.txt", mock_server.uri(), i);
        queue.enqueue(&url, "direct_url", None).await?;
    }

    let client = HttpClient::new();
    // Disabled rate limiter - should allow parallel requests
    let rate_limiter = test_rate_limiter();
    let engine = DownloadEngine::new(10, RetryPolicy::with_max_attempts(1), rate_limiter)?;
    let output_dir = TempDir::new()?;

    let start = std::time::Instant::now();
    let stats = engine
        .process_queue(&queue, &client, output_dir.path())
        .await?;
    let elapsed = start.elapsed();

    // All should complete
    assert_eq!(stats.completed(), 3);

    // With disabled rate limiting and concurrency 10, all 3 should run in parallel
    // Each takes 50ms, so total should be ~50-150ms (not 150ms sequential)
    assert!(
        elapsed < Duration::from_millis(200),
        "Disabled rate limiter should allow parallel requests, elapsed: {:?}",
        elapsed
    );

    Ok(())
}

// ==================== Interrupt Handling Tests ====================

#[tokio::test]
async fn test_interrupt_stops_claiming_new_work() -> Result<(), Box<dyn std::error::Error>> {
    use std::sync::atomic::AtomicBool;

    let (db, _temp_dir) = setup_test_db().await?;
    let queue = Queue::new(db);
    let mock_server = require_mock_server!();

    // Setup slow responses so items stay in-flight
    Mock::given(method("GET"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_bytes(b"content")
                .set_delay(Duration::from_millis(200)),
        )
        .mount(&mock_server)
        .await;

    // Enqueue 10 items
    for i in 0..10 {
        let url = format!("{}/file{}.txt", mock_server.uri(), i);
        queue.enqueue(&url, "direct_url", None).await?;
    }

    let client = HttpClient::new();
    // Low concurrency so items queue up
    let engine = DownloadEngine::new(2, RetryPolicy::with_max_attempts(1), test_rate_limiter())?;
    let output_dir = TempDir::new()?;

    // Set interrupt flag after a short delay
    let interrupted = Arc::new(AtomicBool::new(false));
    let interrupted_clone = Arc::clone(&interrupted);
    tokio::spawn(async move {
        tokio::time::sleep(Duration::from_millis(100)).await;
        interrupted_clone.store(true, std::sync::atomic::Ordering::SeqCst);
    });

    let stats = engine
        .process_queue_interruptible(&queue, &client, output_dir.path(), interrupted)
        .await?;

    // Should have been interrupted
    assert!(stats.was_interrupted(), "Stats should reflect interruption");

    // Should NOT have processed all 10 items (interrupted mid-batch)
    assert!(
        stats.total() < 10,
        "Should not have completed all items, got {}",
        stats.total()
    );

    // Remaining items should still be pending in the queue (available for resume)
    let pending = queue.count_by_status(QueueStatus::Pending).await?;
    assert!(
        pending > 0,
        "Some items should remain pending for resume, got 0"
    );

    Ok(())
}

#[tokio::test]
async fn test_interrupt_flag_before_processing_returns_immediately()
-> Result<(), Box<dyn std::error::Error>> {
    use std::sync::atomic::AtomicBool;

    let (db, _temp_dir) = setup_test_db().await?;
    let queue = Queue::new(db);
    let mock_server = require_mock_server!();

    Mock::given(method("GET"))
        .respond_with(ResponseTemplate::new(200).set_body_bytes(b"content"))
        .mount(&mock_server)
        .await;

    for i in 0..5 {
        let url = format!("{}/file{}.txt", mock_server.uri(), i);
        queue.enqueue(&url, "direct_url", None).await?;
    }

    let client = HttpClient::new();
    let engine = create_engine_no_retry(5)?;
    let output_dir = TempDir::new()?;

    // Set interrupt flag BEFORE processing starts
    let interrupted = Arc::new(AtomicBool::new(true));

    let stats = engine
        .process_queue_interruptible(&queue, &client, output_dir.path(), interrupted)
        .await?;

    assert!(stats.was_interrupted());
    assert_eq!(
        stats.total(),
        0,
        "No items should be processed when interrupted before start"
    );

    // All items should remain pending
    let pending = queue.count_by_status(QueueStatus::Pending).await?;
    assert_eq!(pending, 5, "All items should remain pending");

    Ok(())
}

#[tokio::test]
async fn test_interrupt_requeues_item_waiting_for_permit() -> Result<(), Box<dyn std::error::Error>>
{
    use std::sync::atomic::AtomicBool;

    let (db, _temp_dir) = setup_test_db().await?;
    let queue = Queue::new(db);
    let mock_server = require_mock_server!();

    // Very slow response to keep the single permit occupied
    Mock::given(method("GET"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_bytes(b"content")
                .set_delay(Duration::from_secs(10)),
        )
        .mount(&mock_server)
        .await;

    // Enqueue 3 items. With concurrency=1, the first will occupy the permit
    // and the second will block on semaphore acquire.
    for i in 0..3 {
        let url = format!("{}/file{}.txt", mock_server.uri(), i);
        queue.enqueue(&url, "direct_url", None).await?;
    }

    let client = HttpClient::new();
    let engine = DownloadEngine::new(1, RetryPolicy::with_max_attempts(1), test_rate_limiter())?;
    let output_dir = TempDir::new()?;

    // Set interrupt after 200ms — first item will be in-flight, second will
    // be waiting on the semaphore (the requeue path we're testing).
    let interrupted = Arc::new(AtomicBool::new(false));
    let interrupted_clone = Arc::clone(&interrupted);
    tokio::spawn(async move {
        tokio::time::sleep(Duration::from_millis(200)).await;
        interrupted_clone.store(true, std::sync::atomic::Ordering::SeqCst);
    });

    let stats = engine
        .process_queue_interruptible(&queue, &client, output_dir.path(), interrupted)
        .await?;

    assert!(stats.was_interrupted());

    // With 3 items and concurrency=1:
    // - Item 1: dequeued, acquired permit, in-flight (aborted after 5s timeout → in_progress)
    // - Item 2: dequeued, waiting for permit → requeued to pending by select! interrupt path
    // - Item 3: never dequeued → stays pending
    //
    // If the requeue path didn't work, item 2 would be stuck as in_progress.
    // The key assertion: at least 2 items are pending (requeued + never-dequeued).
    let pending = queue.count_by_status(QueueStatus::Pending).await?;
    assert!(
        pending >= 2,
        "Expected at least 2 pending items (1 requeued + 1 never dequeued), got {}",
        pending
    );

    // The in-flight aborted task stays in_progress (recovered by reset_in_progress on next run)
    let in_progress = queue.count_by_status(QueueStatus::InProgress).await?;
    assert!(
        in_progress <= 1,
        "At most 1 item should be in_progress (the aborted in-flight task), got {}",
        in_progress
    );

    Ok(())
}

// ==================== Queue Dedup Tests ====================

#[tokio::test]
async fn test_has_active_url_detects_pending_duplicate() -> Result<(), Box<dyn std::error::Error>> {
    let (db, _temp_dir) = setup_test_db().await?;
    let queue = Queue::new(db);

    queue
        .enqueue("https://example.com/paper.pdf", "direct_url", None)
        .await?;

    assert!(
        queue
            .has_active_url("https://example.com/paper.pdf")
            .await?,
        "Should detect pending URL as active"
    );
    assert!(
        !queue
            .has_active_url("https://example.com/other.pdf")
            .await?,
        "Should not detect absent URL as active"
    );

    Ok(())
}

#[tokio::test]
async fn test_has_active_url_detects_in_progress_item() -> Result<(), Box<dyn std::error::Error>> {
    let (db, _temp_dir) = setup_test_db().await?;
    let queue = Queue::new(db);

    queue
        .enqueue("https://example.com/downloading.pdf", "direct_url", None)
        .await?;
    // Dequeue transitions the item to in_progress
    queue.dequeue().await?;

    assert!(
        queue
            .has_active_url("https://example.com/downloading.pdf")
            .await?,
        "In-progress URL should be considered active"
    );

    Ok(())
}

#[tokio::test]
async fn test_has_active_url_ignores_completed_items() -> Result<(), Box<dyn std::error::Error>> {
    let (db, _temp_dir) = setup_test_db().await?;
    let queue = Queue::new(db);

    let id = queue
        .enqueue("https://example.com/done.pdf", "direct_url", None)
        .await?;
    queue.dequeue().await?;
    queue.mark_completed(id).await?;

    assert!(
        !queue.has_active_url("https://example.com/done.pdf").await?,
        "Completed URL should not be considered active"
    );

    Ok(())
}

// ==================== Resume / Range Request Tests ====================

#[tokio::test]
async fn test_resume_partial_file_sends_range_request() -> Result<(), Box<dyn std::error::Error>> {
    use wiremock::matchers::header;

    let (db, _temp_dir) = setup_test_db().await?;
    let queue = Queue::new(db);
    let mock_server = require_mock_server!();

    // Full content: "AAABBB" (6 bytes). Simulate partial file with first 3 bytes on disk.
    let output_dir = TempDir::new()?;
    std::fs::write(output_dir.path().join("resume.bin"), b"AAA")?;

    // HEAD response: supports ranges
    Mock::given(method("HEAD"))
        .and(path("/resume.bin"))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("Accept-Ranges", "bytes")
                .insert_header("Content-Length", "6"),
        )
        .mount(&mock_server)
        .await;

    // GET with Range header → 206 with remaining bytes
    Mock::given(method("GET"))
        .and(path("/resume.bin"))
        .and(header("Range", "bytes=3-"))
        .respond_with(
            ResponseTemplate::new(206)
                .set_body_bytes(b"BBB")
                .insert_header("Content-Length", "3"),
        )
        .with_priority(1)
        .mount(&mock_server)
        .await;

    // GET without Range → 200 full content (fallback)
    Mock::given(method("GET"))
        .and(path("/resume.bin"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_bytes(b"AAABBB")
                .insert_header("Content-Length", "6"),
        )
        .with_priority(u8::MAX)
        .mount(&mock_server)
        .await;

    let url = format!("{}/resume.bin", mock_server.uri());
    queue.enqueue(&url, "direct_url", None).await?;

    let client = HttpClient::new();
    let engine = create_engine_no_retry(1)?;
    let stats = engine
        .process_queue(&queue, &client, output_dir.path())
        .await?;

    assert_eq!(stats.completed(), 1, "Download should complete");
    assert_eq!(stats.failed(), 0);

    // Verify final file content is the full 6 bytes
    let final_content = std::fs::read(output_dir.path().join("resume.bin"))?;
    assert_eq!(
        final_content, b"AAABBB",
        "Resumed file should have full content"
    );

    Ok(())
}

#[tokio::test]
async fn test_resume_server_no_range_support_restarts() -> Result<(), Box<dyn std::error::Error>> {
    let (db, _temp_dir) = setup_test_db().await?;
    let queue = Queue::new(db);
    let mock_server = require_mock_server!();

    // Create partial file on disk
    let output_dir = TempDir::new()?;
    std::fs::write(output_dir.path().join("norange.bin"), b"partial")?;

    // HEAD response: no Accept-Ranges header
    Mock::given(method("HEAD"))
        .and(path("/norange.bin"))
        .respond_with(ResponseTemplate::new(200).insert_header("Content-Length", "12"))
        .mount(&mock_server)
        .await;

    // GET → 200 full content (server doesn't support ranges)
    Mock::given(method("GET"))
        .and(path("/norange.bin"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_bytes(b"full content")
                .insert_header("Content-Length", "12"),
        )
        .mount(&mock_server)
        .await;

    let url = format!("{}/norange.bin", mock_server.uri());
    queue.enqueue(&url, "direct_url", None).await?;

    let client = HttpClient::new();
    let engine = create_engine_no_retry(1)?;
    let stats = engine
        .process_queue(&queue, &client, output_dir.path())
        .await?;

    assert_eq!(stats.completed(), 1);

    // Without range support, the engine writes to a new unique-named file
    // (since norange.bin already exists and we're not resuming)
    let files: Vec<_> = std::fs::read_dir(output_dir.path())?
        .filter_map(|e| e.ok())
        .collect();
    assert_eq!(
        files.len(),
        2,
        "Should have original partial + new full download"
    );

    Ok(())
}

#[tokio::test]
async fn test_progress_metadata_persisted_after_download() -> Result<(), Box<dyn std::error::Error>>
{
    let (db, _temp_dir) = setup_test_db().await?;
    let queue = Queue::new(db);
    let mock_server = require_mock_server!();

    Mock::given(method("GET"))
        .and(path("/tracked.pdf"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_bytes(b"tracked content here")
                .insert_header("Content-Length", "20"),
        )
        .mount(&mock_server)
        .await;

    let url = format!("{}/tracked.pdf", mock_server.uri());
    let id = queue.enqueue(&url, "direct_url", None).await?;

    let client = HttpClient::new();
    let engine = create_engine_no_retry(1)?;
    let output_dir = TempDir::new()?;
    let stats = engine
        .process_queue(&queue, &client, output_dir.path())
        .await?;

    assert_eq!(stats.completed(), 1);

    // Verify progress metadata was persisted in queue
    let item = queue.get(id).await?.unwrap();
    assert_eq!(item.status(), QueueStatus::Completed);
    assert!(
        item.bytes_downloaded > 0,
        "bytes_downloaded should be recorded, got {}",
        item.bytes_downloaded
    );

    Ok(())
}

// ==================== Resume Integrity Tests ====================

#[tokio::test]
async fn test_resume_with_mismatched_content_length_fails() -> Result<(), Box<dyn std::error::Error>>
{
    use wiremock::matchers::header;

    let (db, _temp_dir) = setup_test_db().await?;
    let queue = Queue::new(db);
    let mock_server = require_mock_server!();

    // Partial file: 3 bytes on disk
    let output_dir = TempDir::new()?;
    std::fs::write(output_dir.path().join("integrity.bin"), b"AAA")?;

    // HEAD: supports ranges
    Mock::given(method("HEAD"))
        .and(path("/integrity.bin"))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("Accept-Ranges", "bytes")
                .insert_header("Content-Length", "10"),
        )
        .mount(&mock_server)
        .await;

    // 206 response lies about Content-Length: claims 20 bytes remaining but only sends 3.
    // This should fail — either as our integrity check or as a transport-level error.
    Mock::given(method("GET"))
        .and(path("/integrity.bin"))
        .and(header("Range", "bytes=3-"))
        .respond_with(
            ResponseTemplate::new(206)
                .set_body_bytes(b"BBB")
                .insert_header("Content-Length", "20"),
        )
        .with_priority(1)
        .mount(&mock_server)
        .await;

    let url = format!("{}/integrity.bin", mock_server.uri());
    queue.enqueue(&url, "direct_url", None).await?;

    let client = HttpClient::new();
    let engine = create_engine_no_retry(1)?;
    let stats = engine
        .process_queue(&queue, &client, output_dir.path())
        .await?;

    // The download should fail (integrity mismatch or transport error)
    assert_eq!(stats.completed(), 0, "Mismatched resume should not succeed");
    assert_eq!(stats.failed(), 1);

    Ok(())
}

// ==================== Auth Detection Tests ====================

#[tokio::test]
async fn test_login_redirect_detected_as_auth_failure() -> Result<(), Box<dyn std::error::Error>> {
    let (db, _temp_dir) = setup_test_db().await?;
    let queue = Queue::new(db);
    let mock_server = require_mock_server!();

    // Simulate login redirect: PDF URL returns 302 to /login, which returns HTML
    Mock::given(method("GET"))
        .and(path("/paper.pdf"))
        .respond_with(ResponseTemplate::new(302).insert_header(
            "Location",
            format!("{}/login?return=/paper.pdf", mock_server.uri()),
        ))
        .mount(&mock_server)
        .await;

    Mock::given(method("GET"))
        .and(path("/login"))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("Content-Type", "text/html; charset=utf-8")
                .set_body_bytes(
                    "<html><body>Please log in to access this resource</body></html>".as_bytes(),
                ),
        )
        .mount(&mock_server)
        .await;

    let url = format!("{}/paper.pdf", mock_server.uri());
    let id = queue.enqueue(&url, "direct_url", None).await?;

    let client = HttpClient::new();
    let engine = create_engine_no_retry(1)?;
    let output_dir = TempDir::new()?;

    let stats = engine
        .process_queue(&queue, &client, output_dir.path())
        .await?;

    assert_eq!(stats.completed(), 0);
    assert_eq!(stats.failed(), 1);

    let item = queue.get(id).await?.unwrap();
    assert_eq!(item.status(), QueueStatus::Failed);
    let err_msg = item.last_error.as_deref().unwrap_or("");
    assert!(
        err_msg.starts_with("[AUTH]"),
        "Login redirect should produce [AUTH] error, got: {err_msg}"
    );

    Ok(())
}

// ==================== Content-Type Extension Detection Tests ====================

#[tokio::test]
async fn test_content_type_extension_detection_html() -> Result<(), Box<dyn std::error::Error>> {
    let (db, _temp_dir) = setup_test_db().await?;
    let queue = Queue::new(db);

    // Mock server that returns HTML without filename in URL (using root path to trigger fallback)
    let mock_server = require_mock_server!();
    Mock::given(method("GET"))
        .and(path("/"))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("content-type", "text/html; charset=utf-8")
                .set_body_bytes("<html><body>Test</body></html>"),
        )
        .mount(&mock_server)
        .await;

    let url = mock_server.uri();
    queue.enqueue(&url, "direct_url", None).await?;

    let client = HttpClient::new();
    let engine = create_engine(1)?;
    let output_dir = TempDir::new()?;

    let stats = engine
        .process_queue(&queue, &client, output_dir.path())
        .await?;

    assert_eq!(stats.completed(), 1);

    // Verify filename has .html extension, not .bin
    let files: Vec<_> = std::fs::read_dir(output_dir.path())?
        .filter_map(|e| e.ok())
        .collect();

    assert_eq!(files.len(), 1);
    let filename = files[0].file_name();
    let filename_str = filename.to_string_lossy();

    assert!(
        filename_str.ends_with(".html"),
        "Expected .html extension but got: {}",
        filename_str
    );

    Ok(())
}

#[tokio::test]
async fn test_content_type_extension_detection_json() -> Result<(), Box<dyn std::error::Error>> {
    let (db, _temp_dir) = setup_test_db().await?;
    let queue = Queue::new(db);

    let mock_server = require_mock_server!();
    Mock::given(method("GET"))
        .and(path("/"))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("content-type", "application/json")
                .set_body_bytes(r#"{"status": "ok"}"#),
        )
        .mount(&mock_server)
        .await;

    let url = mock_server.uri();
    queue.enqueue(&url, "direct_url", None).await?;

    let client = HttpClient::new();
    let engine = create_engine(1)?;
    let output_dir = TempDir::new()?;

    let stats = engine
        .process_queue(&queue, &client, output_dir.path())
        .await?;

    assert_eq!(stats.completed(), 1);

    let files: Vec<_> = std::fs::read_dir(output_dir.path())?
        .filter_map(|e| e.ok())
        .collect();

    assert_eq!(files.len(), 1);
    let filename = files[0].file_name();
    let filename_str = filename.to_string_lossy();

    assert!(
        filename_str.ends_with(".json"),
        "Expected .json extension but got: {}",
        filename_str
    );

    Ok(())
}

#[tokio::test]
async fn test_content_type_fallback_to_bin() -> Result<(), Box<dyn std::error::Error>> {
    let (db, _temp_dir) = setup_test_db().await?;
    let queue = Queue::new(db);

    let mock_server = require_mock_server!();
    Mock::given(method("GET"))
        .and(path("/"))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("content-type", "application/octet-stream")
                .set_body_bytes(vec![0x00, 0x01, 0x02, 0x03]),
        )
        .mount(&mock_server)
        .await;

    let url = mock_server.uri();
    queue.enqueue(&url, "direct_url", None).await?;

    let client = HttpClient::new();
    let engine = create_engine(1)?;
    let output_dir = TempDir::new()?;

    let stats = engine
        .process_queue(&queue, &client, output_dir.path())
        .await?;

    assert_eq!(stats.completed(), 1);

    let files: Vec<_> = std::fs::read_dir(output_dir.path())?
        .filter_map(|e| e.ok())
        .collect();

    assert_eq!(files.len(), 1);
    let filename = files[0].file_name();
    let filename_str = filename.to_string_lossy();

    assert!(
        filename_str.ends_with(".bin"),
        "Expected .bin extension for unknown content-type but got: {}",
        filename_str
    );

    Ok(())
}

#[tokio::test]
async fn test_metadata_suggested_filename_is_used_for_download_path()
-> Result<(), Box<dyn std::error::Error>> {
    let (db, _temp_dir) = setup_test_db().await?;
    let queue = Queue::new(db);

    let mock_server = require_mock_server!();
    Mock::given(method("GET"))
        .and(path("/paper.pdf"))
        .respond_with(ResponseTemplate::new(200).set_body_bytes(b"pdf-bytes"))
        .mount(&mock_server)
        .await;

    let url = format!("{}/paper.pdf", mock_server.uri());
    let metadata = QueueMetadata {
        suggested_filename: Some("Smith_2024_Climate_Study.pdf".to_string()),
        title: Some("Climate Study".to_string()),
        authors: Some("Smith, John".to_string()),
        year: Some("2024".to_string()),
        doi: Some("10.1000/test".to_string()),
        topics: None,
        parse_confidence: None,
        parse_confidence_factors: None,
    };
    queue
        .enqueue_with_metadata(&url, "doi", Some("10.1000/test"), Some(&metadata))
        .await?;

    let client = HttpClient::new();
    let engine = create_engine_no_retry(1)?;
    let output_dir = TempDir::new()?;

    let stats = engine
        .process_queue(&queue, &client, output_dir.path())
        .await?;
    assert_eq!(stats.completed(), 1);

    let expected = output_dir.path().join("Smith_2024_Climate_Study.pdf");
    assert!(
        expected.exists(),
        "expected metadata-driven filename at {:?}",
        expected
    );

    Ok(())
}

#[tokio::test]
async fn test_metadata_duplicate_suffix_starts_at_two() -> Result<(), Box<dyn std::error::Error>> {
    let (db, _temp_dir) = setup_test_db().await?;
    let queue = Queue::new(db);

    let mock_server = require_mock_server!();
    Mock::given(method("GET"))
        .and(path("/paper.pdf"))
        .respond_with(ResponseTemplate::new(200).set_body_bytes(b"pdf-bytes"))
        .mount(&mock_server)
        .await;

    let output_dir = TempDir::new()?;
    std::fs::write(
        output_dir.path().join("Smith_2024_Climate_Study.pdf"),
        b"existing",
    )?;

    let url = format!("{}/paper.pdf", mock_server.uri());
    let metadata = QueueMetadata {
        suggested_filename: Some("Smith_2024_Climate_Study.pdf".to_string()),
        title: Some("Climate Study".to_string()),
        authors: Some("Smith, John".to_string()),
        year: Some("2024".to_string()),
        doi: Some("10.1000/test".to_string()),
        topics: None,
        parse_confidence: None,
        parse_confidence_factors: None,
    };
    queue
        .enqueue_with_metadata(&url, "doi", Some("10.1000/test"), Some(&metadata))
        .await?;

    let client = HttpClient::new();
    let engine = create_engine_no_retry(1)?;

    let stats = engine
        .process_queue(&queue, &client, output_dir.path())
        .await?;
    assert_eq!(stats.completed(), 1);

    let expected = output_dir.path().join("Smith_2024_Climate_Study_2.pdf");
    assert!(
        expected.exists(),
        "expected duplicate metadata filename with _2 suffix at {:?}",
        expected
    );

    Ok(())
}
