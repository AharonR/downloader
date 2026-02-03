//! Download engine for concurrent file downloads with retry support.
//!
//! This module provides the `DownloadEngine` which coordinates concurrent
//! downloads using a semaphore-based concurrency control pattern, with
//! automatic retry on transient failures using exponential backoff.
//!
//! # Overview
//!
//! The engine processes items from a [`Queue`], downloading each item
//! using an [`HttpClient`], with configurable concurrency limits and
//! retry policies.
//!
//! # Example
//!
//! ```no_run
//! use downloader_core::download::{DownloadEngine, HttpClient, RetryPolicy, RateLimiter};
//! use downloader_core::queue::Queue;
//! use downloader_core::Database;
//! use std::path::Path;
//! use std::sync::Arc;
//! use std::time::Duration;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let db = Database::new_in_memory().await?;
//! let queue = Queue::new(db);
//! let rate_limiter = Arc::new(RateLimiter::new(Duration::from_millis(1000)));
//! let engine = DownloadEngine::new(10, RetryPolicy::default(), rate_limiter)?;
//! let client = HttpClient::new();
//! let stats = engine.process_queue(&queue, &client, Path::new("./downloads")).await?;
//! println!("Completed: {}, Failed: {}, Retried: {}", stats.completed(), stats.failed(), stats.retried());
//! # Ok(())
//! # }
//! ```

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;

use tokio::sync::Semaphore;
use tracing::{debug, info, instrument, warn};

use super::rate_limiter::{RateLimiter, parse_retry_after};
use super::retry::{FailureType, RetryDecision, RetryPolicy, classify_error};
use super::{DownloadError, HttpClient};
use crate::queue::{Queue, QueueError, QueueItem};

/// Minimum allowed concurrency value.
const MIN_CONCURRENCY: usize = 1;

/// Maximum allowed concurrency value.
const MAX_CONCURRENCY: usize = 100;

/// Default concurrency if not specified.
pub const DEFAULT_CONCURRENCY: usize = 10;

/// Error type for download engine operations.
#[derive(Debug, thiserror::Error)]
pub enum EngineError {
    /// Invalid concurrency value provided.
    #[error(
        "invalid concurrency value {value}: must be between {MIN_CONCURRENCY} and {MAX_CONCURRENCY}"
    )]
    InvalidConcurrency {
        /// The invalid value that was provided.
        value: usize,
    },

    /// Queue operation failed.
    #[error("queue error: {0}")]
    Queue(#[from] QueueError),

    /// Semaphore was closed unexpectedly.
    #[error("semaphore closed unexpectedly")]
    SemaphoreClosed,
}

/// Statistics from a download batch run.
///
/// Tracks the number of completed, failed, and retried downloads during a
/// `process_queue()` invocation. Uses atomic counters for thread-safe
/// updates from concurrent download tasks.
#[derive(Debug, Default)]
pub struct DownloadStats {
    completed: AtomicUsize,
    failed: AtomicUsize,
    retried: AtomicUsize,
}

impl DownloadStats {
    /// Creates a new stats tracker with zero counts.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Returns the number of successfully completed downloads.
    #[must_use]
    pub fn completed(&self) -> usize {
        self.completed.load(Ordering::SeqCst)
    }

    /// Returns the number of failed downloads.
    #[must_use]
    pub fn failed(&self) -> usize {
        self.failed.load(Ordering::SeqCst)
    }

    /// Returns the total number of items processed (completed + failed).
    #[must_use]
    pub fn total(&self) -> usize {
        self.completed() + self.failed()
    }

    /// Returns the number of retry attempts made.
    #[must_use]
    pub fn retried(&self) -> usize {
        self.retried.load(Ordering::SeqCst)
    }

    /// Increments the completed counter.
    fn increment_completed(&self) {
        self.completed.fetch_add(1, Ordering::SeqCst);
    }

    /// Increments the failed counter.
    fn increment_failed(&self) {
        self.failed.fetch_add(1, Ordering::SeqCst);
    }

    /// Increments the retried counter.
    fn increment_retried(&self) {
        self.retried.fetch_add(1, Ordering::SeqCst);
    }
}

/// Download engine for concurrent file downloads with retry support.
///
/// The engine uses a semaphore to limit the number of concurrent downloads,
/// preventing resource exhaustion and respecting server rate limits. Failed
/// downloads are automatically retried with exponential backoff for transient
/// errors. Per-domain rate limiting ensures requests to the same domain are
/// properly spaced to avoid overwhelming servers.
///
/// # Concurrency Model
///
/// - Each download runs in its own Tokio task
/// - A semaphore permit is acquired before starting each download
/// - Permits are released automatically when downloads complete (RAII)
/// - The main loop dequeues items atomically from the queue
///
/// # Rate Limiting
///
/// - Per-domain rate limiting enforces minimum delays between requests
/// - Different domains can proceed in parallel without waiting for each other
/// - Retry-After headers are respected when servers return 429
///
/// # Retry Behavior
///
/// - Transient errors (network issues, 5xx) are retried with exponential backoff
/// - Permanent errors (404, 400) fail immediately without retry
/// - Retry count is tracked in-memory during the retry loop
/// - Final retry count is persisted to database when marking item as failed
#[derive(Debug)]
pub struct DownloadEngine {
    /// Semaphore for concurrency control.
    semaphore: Arc<Semaphore>,
    /// Configured concurrency limit.
    concurrency: usize,
    /// Retry policy for failed downloads.
    retry_policy: RetryPolicy,
    /// Per-domain rate limiter.
    rate_limiter: Arc<RateLimiter>,
}

impl DownloadEngine {
    /// Creates a new download engine with the specified concurrency limit, retry policy,
    /// and rate limiter.
    ///
    /// # Arguments
    ///
    /// * `concurrency` - Maximum number of concurrent downloads (1-100)
    /// * `retry_policy` - Policy for retrying failed downloads
    /// * `rate_limiter` - Per-domain rate limiter wrapped in Arc for sharing
    ///
    /// # Errors
    ///
    /// Returns [`EngineError::InvalidConcurrency`] if the value is outside
    /// the valid range (1-100).
    ///
    /// # Example
    ///
    /// ```
    /// use std::sync::Arc;
    /// use std::time::Duration;
    /// use downloader_core::download::{DownloadEngine, RetryPolicy, RateLimiter};
    ///
    /// let rate_limiter = Arc::new(RateLimiter::new(Duration::from_millis(1000)));
    /// let engine = DownloadEngine::new(10, RetryPolicy::default(), rate_limiter).unwrap();
    /// ```
    #[instrument(level = "debug", skip(retry_policy, rate_limiter))]
    pub fn new(
        concurrency: usize,
        retry_policy: RetryPolicy,
        rate_limiter: Arc<RateLimiter>,
    ) -> Result<Self, EngineError> {
        if !(MIN_CONCURRENCY..=MAX_CONCURRENCY).contains(&concurrency) {
            return Err(EngineError::InvalidConcurrency { value: concurrency });
        }

        debug!(
            concurrency,
            max_retries = retry_policy.max_attempts(),
            rate_limit_ms = rate_limiter.default_delay().as_millis(),
            rate_limit_disabled = rate_limiter.is_disabled(),
            "creating download engine"
        );

        Ok(Self {
            semaphore: Arc::new(Semaphore::new(concurrency)),
            concurrency,
            retry_policy,
            rate_limiter,
        })
    }

    /// Returns the configured concurrency limit.
    #[must_use]
    #[instrument(skip(self))]
    pub fn concurrency(&self) -> usize {
        self.concurrency
    }

    /// Returns the configured retry policy.
    #[must_use]
    pub fn retry_policy(&self) -> &RetryPolicy {
        &self.retry_policy
    }

    /// Processes all pending items in the queue concurrently.
    ///
    /// This method:
    /// 1. Dequeues items atomically from the queue
    /// 2. Spawns download tasks up to the concurrency limit
    /// 3. Retries transient failures with exponential backoff
    /// 4. Updates queue status on completion/failure
    /// 5. Returns statistics when all downloads complete
    ///
    /// # Arguments
    ///
    /// * `queue` - The download queue to process
    /// * `client` - HTTP client for downloads
    /// * `output_dir` - Directory to save downloaded files
    ///
    /// # Returns
    ///
    /// Statistics containing completed, failed, and retry counts.
    ///
    /// # Errors
    ///
    /// Returns [`EngineError::Queue`] if queue operations fail.
    /// Returns [`EngineError::SemaphoreClosed`] if the semaphore is closed.
    ///
    /// Note: Individual download failures do NOT cause this method to error.
    /// Failed downloads are marked in the queue and counted in stats.
    #[instrument(skip(self, queue, client), fields(output_dir = %output_dir.display()))]
    pub async fn process_queue(
        &self,
        queue: &Queue,
        client: &HttpClient,
        output_dir: &Path,
    ) -> Result<DownloadStats, EngineError> {
        let stats = Arc::new(DownloadStats::new());
        let mut handles = Vec::new();

        info!("starting queue processing");

        // Keep dequeuing until no more pending items
        loop {
            let Some(item) = queue.dequeue().await? else {
                break; // No more pending items
            };

            debug!(item_id = item.id, url = %item.url, "dequeued item");

            // Acquire semaphore permit (blocks if at concurrency limit)
            let permit = self
                .semaphore
                .clone()
                .acquire_owned()
                .await
                .map_err(|_| EngineError::SemaphoreClosed)?;

            // Clone values for the spawned task
            let queue = queue.clone();
            let client = client.clone();
            let stats = Arc::clone(&stats);
            let output_dir = output_dir.to_path_buf();
            let retry_policy = self.retry_policy.clone();
            let rate_limiter = Arc::clone(&self.rate_limiter);

            // Spawn download task with retry logic
            handles.push(tokio::spawn(async move {
                // Permit is dropped when this block exits (RAII)
                let _permit = permit;

                let result = download_with_retry(
                    &client,
                    &item,
                    &output_dir,
                    &retry_policy,
                    &stats,
                    &rate_limiter,
                )
                .await;

                match result {
                    Ok(path) => {
                        info!(item_id = item.id, path = %path.display(), "download completed");
                        // Best-effort status update - don't crash if it fails
                        if let Err(e) = queue.mark_completed(item.id).await {
                            warn!(item_id = item.id, error = %e, "failed to mark item completed");
                        }
                        stats.increment_completed();
                    }
                    Err((e, attempts)) => {
                        warn!(
                            item_id = item.id,
                            url = %item.url,
                            error = %e,
                            attempts,
                            "download failed after all attempts"
                        );
                        // Best-effort status update with retry count
                        if let Err(qe) = queue.mark_failed(item.id, &e.to_string()).await {
                            warn!(item_id = item.id, error = %qe, "failed to mark item failed");
                        }
                        stats.increment_failed();
                    }
                }
            }));
        }

        debug!(
            task_count = handles.len(),
            "waiting for downloads to complete"
        );

        // Wait for all tasks to complete
        for handle in handles {
            // Ignore JoinError - task panics are logged but don't fail the batch
            if let Err(e) = handle.await {
                warn!(error = %e, "download task panicked");
            }
        }

        let completed = stats.completed();
        let failed = stats.failed();
        let retried = stats.retried();
        info!(
            completed,
            failed,
            retried,
            total = completed + failed,
            "queue processing complete"
        );

        // We need to return the stats, but we have an Arc.
        // Since all tasks are done, we should have sole ownership.
        // If not (which would be a bug), create new stats from the atomic values.
        match Arc::try_unwrap(stats) {
            Ok(stats) => Ok(stats),
            Err(arc_stats) => {
                // Fallback: create new stats from atomic values
                // This shouldn't happen, but handles the edge case gracefully
                let new_stats = DownloadStats::new();
                new_stats
                    .completed
                    .store(arc_stats.completed(), Ordering::SeqCst);
                new_stats.failed.store(arc_stats.failed(), Ordering::SeqCst);
                new_stats
                    .retried
                    .store(arc_stats.retried(), Ordering::SeqCst);
                Ok(new_stats)
            }
        }
    }
}

/// Extracts and parses the Retry-After delay from a rate-limited error.
///
/// If the error contains a valid Retry-After header, this function:
/// 1. Parses the header value into a Duration
/// 2. Records the delay with the rate limiter for the domain
/// 3. Returns the parsed duration
///
/// Returns `None` if the error doesn't contain a Retry-After header or
/// if the header cannot be parsed.
fn extract_retry_after_delay(
    error: &DownloadError,
    url: &str,
    rate_limiter: &RateLimiter,
) -> Option<Duration> {
    // Extract retry_after from HttpStatus error
    let retry_after_header = match error {
        DownloadError::HttpStatus { retry_after, .. } => retry_after.as_ref()?,
        _ => return None,
    };

    // Parse the Retry-After header
    let delay = parse_retry_after(retry_after_header)?;

    // Record the server-mandated rate limit with the rate limiter
    rate_limiter.record_rate_limit(url, delay);

    debug!(
        url = %url,
        retry_after = %retry_after_header,
        delay_ms = delay.as_millis(),
        "using Retry-After header delay"
    );

    Some(delay)
}

/// Downloads a file with retry logic for transient errors.
///
/// Retry attempts are tracked in-memory during the retry loop. Only the final
/// error and attempt count are returned if all retries are exhausted.
///
/// Rate limiting is applied before each download attempt to respect per-domain
/// delays.
///
/// # Returns
///
/// - `Ok(PathBuf)` - Path to the downloaded file on success
/// - `Err((DownloadError, u32))` - Error and total attempt count on failure
#[instrument(skip(client, item, output_dir, policy, stats, rate_limiter), fields(item_id = item.id, url = %item.url))]
async fn download_with_retry(
    client: &HttpClient,
    item: &QueueItem,
    output_dir: &Path,
    policy: &RetryPolicy,
    stats: &DownloadStats,
    rate_limiter: &RateLimiter,
) -> Result<PathBuf, (DownloadError, u32)> {
    let mut attempt = 0u32;

    loop {
        attempt += 1;
        debug!(attempt, "attempting download");

        // Acquire rate limit permission before making request
        rate_limiter.acquire(&item.url).await;

        match client.download_to_file(&item.url, output_dir).await {
            Ok(path) => return Ok(path),
            Err(e) => {
                let failure_type = classify_error(&e);

                // Check for Retry-After header on 429 responses
                let retry_after_delay = if failure_type == FailureType::RateLimited {
                    extract_retry_after_delay(&e, &item.url, rate_limiter)
                } else {
                    None
                };

                match policy.should_retry(failure_type, attempt) {
                    RetryDecision::Retry {
                        delay: backoff_delay,
                        attempt: next_attempt,
                    } => {
                        // Use Retry-After delay if available, otherwise use exponential backoff
                        let delay = retry_after_delay.unwrap_or(backoff_delay);

                        info!(
                            url = %item.url,
                            attempt = next_attempt,
                            max_attempts = policy.max_attempts(),
                            delay_ms = delay.as_millis(),
                            using_retry_after = retry_after_delay.is_some(),
                            error = %e,
                            "retrying download"
                        );
                        stats.increment_retried();
                        tokio::time::sleep(delay).await;
                    }
                    RetryDecision::DoNotRetry { reason } => {
                        debug!(
                            url = %item.url,
                            %reason,
                            "not retrying download"
                        );
                        return Err((e, attempt));
                    }
                }
            }
        }
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use std::time::Duration;

    use super::*;

    /// Helper to create a default rate limiter for tests.
    fn test_rate_limiter() -> Arc<RateLimiter> {
        Arc::new(RateLimiter::new(Duration::from_millis(100)))
    }

    #[test]
    fn test_engine_new_valid_concurrency() {
        // Test minimum valid value
        let engine = DownloadEngine::new(1, RetryPolicy::default(), test_rate_limiter()).unwrap();
        assert_eq!(engine.concurrency(), 1);

        // Test default value
        let engine = DownloadEngine::new(10, RetryPolicy::default(), test_rate_limiter()).unwrap();
        assert_eq!(engine.concurrency(), 10);

        // Test maximum valid value
        let engine = DownloadEngine::new(100, RetryPolicy::default(), test_rate_limiter()).unwrap();
        assert_eq!(engine.concurrency(), 100);
    }

    #[test]
    fn test_engine_new_invalid_concurrency_zero() {
        let result = DownloadEngine::new(0, RetryPolicy::default(), test_rate_limiter());
        assert!(result.is_err());
        assert!(matches!(
            result,
            Err(EngineError::InvalidConcurrency { value: 0 })
        ));
    }

    #[test]
    fn test_engine_new_invalid_concurrency_too_high() {
        let result = DownloadEngine::new(101, RetryPolicy::default(), test_rate_limiter());
        assert!(result.is_err());
        assert!(matches!(
            result,
            Err(EngineError::InvalidConcurrency { value: 101 })
        ));
    }

    #[test]
    fn test_engine_stores_retry_policy() {
        let policy = RetryPolicy::with_max_attempts(5);
        let engine = DownloadEngine::new(10, policy, test_rate_limiter()).unwrap();
        assert_eq!(engine.retry_policy().max_attempts(), 5);
    }

    #[test]
    fn test_download_stats_default() {
        let stats = DownloadStats::default();
        assert_eq!(stats.completed(), 0);
        assert_eq!(stats.failed(), 0);
        assert_eq!(stats.retried(), 0);
        assert_eq!(stats.total(), 0);
    }

    #[test]
    fn test_download_stats_increment() {
        let stats = DownloadStats::new();

        stats.increment_completed();
        stats.increment_completed();
        stats.increment_failed();
        stats.increment_retried();
        stats.increment_retried();
        stats.increment_retried();

        assert_eq!(stats.completed(), 2);
        assert_eq!(stats.failed(), 1);
        assert_eq!(stats.retried(), 3);
        assert_eq!(stats.total(), 3);
    }

    #[test]
    fn test_download_stats_thread_safe() {
        use std::thread;

        let stats = Arc::new(DownloadStats::new());
        let mut handles = Vec::new();

        // Spawn multiple threads incrementing counters
        for _ in 0..10 {
            let stats = Arc::clone(&stats);
            handles.push(thread::spawn(move || {
                for _ in 0..100 {
                    stats.increment_completed();
                    stats.increment_failed();
                    stats.increment_retried();
                }
            }));
        }

        for handle in handles {
            handle.join().unwrap();
        }

        // 10 threads * 100 increments each
        assert_eq!(stats.completed(), 1000);
        assert_eq!(stats.failed(), 1000);
        assert_eq!(stats.retried(), 1000);
        assert_eq!(stats.total(), 2000);
    }

    #[test]
    fn test_engine_error_display() {
        let error = EngineError::InvalidConcurrency { value: 0 };
        let msg = error.to_string();
        assert!(msg.contains("invalid concurrency"));
        assert!(msg.contains("0"));
        assert!(msg.contains("1")); // min
        assert!(msg.contains("100")); // max
    }

    #[test]
    fn test_default_concurrency_constant() {
        assert_eq!(DEFAULT_CONCURRENCY, 10);
    }
}
