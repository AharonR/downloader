//! Integration tests for the download engine module.
//!
//! These tests verify DownloadEngine with real Queue/Database and mock HTTP server,
//! including retry functionality with exponential backoff.

use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;

use downloader_core::{
    Database, DownloadEngine, HttpClient, Queue, QueueStatus, RateLimiter, RetryPolicy,
};
use tempfile::TempDir;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, Respond, ResponseTemplate};

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
    let mock_server = MockServer::start().await;
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
async fn test_process_queue_single_item_failure() -> Result<(), Box<dyn std::error::Error>> {
    let (db, _temp_dir) = setup_test_db().await?;
    let queue = Queue::new(db);

    // Setup mock server that returns 404 (permanent error - no retry)
    let mock_server = MockServer::start().await;
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

// ==================== Mixed Success/Failure Tests ====================

#[tokio::test]
async fn test_process_queue_mixed_success_and_failure() -> Result<(), Box<dyn std::error::Error>> {
    let (db, _temp_dir) = setup_test_db().await?;
    let queue = Queue::new(db);

    // Setup mock server
    let mock_server = MockServer::start().await;

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
    let mock_server = MockServer::start().await;

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
    let mock_server = MockServer::start().await;

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
    let mock_server = MockServer::start().await;

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
    let mock_server = MockServer::start().await;

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

    let mock_server = MockServer::start().await;

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

    let mock_server = MockServer::start().await;

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

    let mock_server = MockServer::start().await;

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
    Ok(())
}

#[tokio::test]
async fn test_403_does_not_retry() -> Result<(), Box<dyn std::error::Error>> {
    let (db, _temp_dir) = setup_test_db().await?;
    let queue = Queue::new(db);

    let mock_server = MockServer::start().await;

    // 403 is NeedsAuth - should NOT retry (until Epic 4)
    Mock::given(method("GET"))
        .and(path("/forbidden.pdf"))
        .respond_with(ResponseTemplate::new(403))
        .expect(1) // Should only be called once
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
    assert_eq!(stats.retried(), 0);

    let item = queue.get(id).await?.unwrap();
    assert_eq!(item.status(), QueueStatus::Failed);
    Ok(())
}

#[tokio::test]
async fn test_429_triggers_retry_with_backoff() -> Result<(), Box<dyn std::error::Error>> {
    let (db, _temp_dir) = setup_test_db().await?;
    let queue = Queue::new(db);

    let mock_server = MockServer::start().await;

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

    let mock_server = MockServer::start().await;

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

    let mock_server = MockServer::start().await;

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

    let mock_server = MockServer::start().await;

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

    let mock_server = MockServer::start().await;

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

    let mock_server = MockServer::start().await;

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
