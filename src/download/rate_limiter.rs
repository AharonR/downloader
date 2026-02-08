//! Per-domain rate limiting for download requests.
//!
//! This module provides the [`RateLimiter`] struct which enforces minimum delays
//! between requests to the same domain, preventing servers from blocking the client
//! due to excessive request rates.
//!
//! # Overview
//!
//! Rate limiting is applied per-domain, meaning requests to different domains
//! can proceed in parallel without waiting for each other. Only subsequent
//! requests to the *same* domain are delayed.
//!
//! # Example
//!
//! ```
//! use std::sync::Arc;
//! use std::time::Duration;
//! use downloader_core::download::RateLimiter;
//!
//! # async fn example() {
//! // Create a rate limiter with 1 second delay between requests
//! let limiter = Arc::new(RateLimiter::new(Duration::from_secs(1)));
//!
//! // First request proceeds immediately
//! limiter.acquire("https://example.com/file1.pdf").await;
//!
//! // Second request to same domain waits for the delay
//! limiter.acquire("https://example.com/file2.pdf").await;
//!
//! // Request to different domain proceeds immediately
//! limiter.acquire("https://other.com/file.pdf").await;
//! # }
//! ```

use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

use dashmap::DashMap;
use tokio::sync::Mutex;
use tokio::time::Instant;
use tracing::{debug, instrument, warn};

/// Warning threshold for cumulative delay per domain (30 seconds).
const CUMULATIVE_DELAY_WARNING_THRESHOLD: Duration = Duration::from_secs(30);

/// Maximum Retry-After value (1 hour) to prevent excessive delays.
const MAX_RETRY_AFTER: Duration = Duration::from_secs(3600);

/// Per-domain rate limiter for download requests.
///
/// This struct is designed to be wrapped in `Arc` and shared across multiple
/// Tokio tasks. It uses `DashMap` for lock-free concurrent access to per-domain
/// state, and `tokio::sync::Mutex` for atomic read-update operations on timing.
///
/// # Thread Safety
///
/// `RateLimiter` is `Send + Sync`, making it safe to use with `Arc` and share
/// across spawned Tokio tasks.
///
/// # Usage Pattern
///
/// ```no_run
/// use std::sync::Arc;
/// use std::time::Duration;
/// use downloader_core::download::RateLimiter;
///
/// # async fn example() {
/// let rate_limiter = Arc::new(RateLimiter::new(Duration::from_millis(1000)));
///
/// // Clone Arc for spawned task
/// let limiter = Arc::clone(&rate_limiter);
/// tokio::spawn(async move {
///     limiter.acquire("https://example.com/file.pdf").await;
///     // ... download
/// });
/// # }
/// ```
#[derive(Debug)]
pub struct RateLimiter {
    /// Default minimum delay between requests to the same domain.
    default_delay: Duration,

    /// Whether rate limiting is disabled (for `--rate-limit 0`).
    disabled: bool,

    /// Per-domain state tracking.
    /// Uses Arc to allow cloning the state and releasing the `DashMap` lock
    /// before awaiting on the inner Mutex (prevents shard lock across await).
    domains: DashMap<String, Arc<DomainState>>,
}

/// State tracked for each domain.
#[derive(Debug)]
struct DomainState {
    /// Time of the last request to this domain.
    /// Protected by Mutex for atomic read-update operations.
    /// `None` indicates this domain has not been requested yet (first request is immediate).
    last_request: Mutex<Option<Instant>>,

    /// Cumulative delay applied to this domain (in milliseconds).
    /// Used to warn when excessive rate limiting occurs.
    cumulative_delay_ms: AtomicU64,
}

impl DomainState {
    /// Creates a new domain state for a domain that hasn't been requested yet.
    fn new() -> Self {
        Self {
            // None means first request - no delay needed
            last_request: Mutex::new(None),
            cumulative_delay_ms: AtomicU64::new(0),
        }
    }

    /// Adds to the cumulative delay and returns the new total.
    #[allow(clippy::cast_possible_truncation)]
    fn add_cumulative_delay(&self, delay: Duration) -> Duration {
        let delay_ms = delay.as_millis() as u64;
        let new_total = self
            .cumulative_delay_ms
            .fetch_add(delay_ms, Ordering::SeqCst)
            + delay_ms;
        Duration::from_millis(new_total)
    }
}

impl RateLimiter {
    /// Creates a new rate limiter with the specified default delay.
    ///
    /// # Arguments
    ///
    /// * `default_delay` - Minimum time between requests to the same domain
    ///
    /// # Example
    ///
    /// ```
    /// use std::time::Duration;
    /// use downloader_core::download::RateLimiter;
    ///
    /// let limiter = RateLimiter::new(Duration::from_millis(1000));
    /// ```
    #[must_use]
    #[instrument(skip_all, fields(delay_ms = default_delay.as_millis()))]
    pub fn new(default_delay: Duration) -> Self {
        debug!("creating rate limiter");
        Self {
            default_delay,
            disabled: false,
            domains: DashMap::new(),
        }
    }

    /// Creates a disabled rate limiter that applies no delays.
    ///
    /// Use this when `--rate-limit 0` is specified.
    ///
    /// # Example
    ///
    /// ```
    /// use downloader_core::download::RateLimiter;
    ///
    /// let limiter = RateLimiter::disabled();
    /// ```
    #[must_use]
    #[instrument]
    pub fn disabled() -> Self {
        debug!("creating disabled rate limiter");
        Self {
            default_delay: Duration::ZERO,
            disabled: true,
            domains: DashMap::new(),
        }
    }

    /// Returns whether rate limiting is disabled.
    #[must_use]
    pub fn is_disabled(&self) -> bool {
        self.disabled
    }

    /// Returns the default delay between requests.
    #[must_use]
    pub fn default_delay(&self) -> Duration {
        self.default_delay
    }

    /// Acquires permission to make a request to the given URL's domain.
    ///
    /// This method will:
    /// 1. Extract the domain from the URL
    /// 2. Wait if necessary to respect the rate limit
    /// 3. Update the domain's last request time
    ///
    /// The first request to any domain proceeds immediately without delay.
    ///
    /// # Arguments
    ///
    /// * `url` - The URL to make a request to
    ///
    /// # Example
    ///
    /// ```no_run
    /// use std::time::Duration;
    /// use downloader_core::download::RateLimiter;
    ///
    /// # async fn example() {
    /// let limiter = RateLimiter::new(Duration::from_secs(1));
    /// limiter.acquire("https://example.com/file.pdf").await;
    /// # }
    /// ```
    #[instrument(skip(self), fields(domain))]
    pub async fn acquire(&self, url: &str) {
        if self.disabled {
            return;
        }

        let domain = extract_domain(url);
        tracing::Span::current().record("domain", &domain);

        // Get or create domain state, clone Arc to release DashMap lock before awaiting
        let state = self
            .domains
            .entry(domain.clone())
            .or_insert_with(|| Arc::new(DomainState::new()))
            .clone();

        // Lock the state to atomically check and update
        // Note: DashMap lock is released above, only Mutex lock is held during await
        let mut last_request_guard = state.last_request.lock().await;

        // Check if this is the first request (None) or a subsequent request
        if let Some(last_request) = *last_request_guard {
            let elapsed = last_request.elapsed();

            if elapsed < self.default_delay {
                let delay = self.default_delay.saturating_sub(elapsed);
                let cumulative = state.add_cumulative_delay(delay);

                debug!(
                    domain = %domain,
                    delay_ms = delay.as_millis(),
                    cumulative_ms = cumulative.as_millis(),
                    "applying rate limit delay"
                );

                // Warn if cumulative delay exceeds threshold
                if cumulative >= CUMULATIVE_DELAY_WARNING_THRESHOLD {
                    warn!(
                        domain = %domain,
                        cumulative_delay_secs = cumulative.as_secs(),
                        "excessive rate limiting - consider reducing request volume to this domain"
                    );
                }

                tokio::time::sleep(delay).await;
            }
        } else {
            debug!(domain = %domain, "first request to domain - no delay");
        }

        // Update last request time after any delay
        *last_request_guard = Some(Instant::now());
    }

    /// Records a server-mandated rate limit delay (from Retry-After header).
    ///
    /// This updates the domain's state to reflect the server's rate limit,
    /// ensuring subsequent requests respect the server's wishes.
    ///
    /// # Arguments
    ///
    /// * `url` - The URL that returned the rate limit
    /// * `delay` - The delay specified by the server
    #[instrument(skip(self), fields(domain))]
    pub fn record_rate_limit(&self, url: &str, delay: Duration) {
        let domain = extract_domain(url);
        tracing::Span::current().record("domain", &domain);

        let state = self
            .domains
            .entry(domain.clone())
            .or_insert_with(|| Arc::new(DomainState::new()));
        let cumulative = state.add_cumulative_delay(delay);

        debug!(
            domain = %domain,
            delay_ms = delay.as_millis(),
            cumulative_ms = cumulative.as_millis(),
            "recorded server rate limit"
        );

        // Warn if cumulative delay exceeds threshold
        if cumulative >= CUMULATIVE_DELAY_WARNING_THRESHOLD {
            warn!(
                domain = %domain,
                cumulative_delay_secs = cumulative.as_secs(),
                "excessive server rate limiting - site may be under heavy load"
            );
        }
    }
}

/// Extracts the domain from a URL.
///
/// Returns "unknown" for malformed URLs, ensuring all requests are still
/// rate limited even if the URL cannot be parsed.
///
/// # Examples
///
/// ```
/// use downloader_core::download::rate_limiter::extract_domain;
///
/// assert_eq!(extract_domain("https://example.com/path"), "example.com");
/// assert_eq!(extract_domain("http://Example.COM/Path"), "example.com");
/// assert_eq!(extract_domain("https://192.168.1.1/file"), "192.168.1.1");
/// assert_eq!(extract_domain("https://localhost:8080/x"), "localhost");
/// assert_eq!(extract_domain("not a url"), "unknown");
/// ```
#[must_use]
pub fn extract_domain(url: &str) -> String {
    url::Url::parse(url)
        .ok()
        .and_then(|u| u.host_str().map(str::to_lowercase))
        .unwrap_or_else(|| "unknown".to_string())
}

/// Parses a Retry-After header value into a Duration.
///
/// Supports two formats as per RFC 7231:
/// - Integer seconds: `Retry-After: 120`
/// - HTTP-date: `Retry-After: Wed, 21 Oct 2025 07:28:00 GMT`
///
/// Returns `None` if the value cannot be parsed. Caps excessive values at 1 hour.
///
/// # Arguments
///
/// * `header_value` - The raw Retry-After header value
///
/// # Examples
///
/// ```
/// use std::time::Duration;
/// use downloader_core::download::rate_limiter::parse_retry_after;
///
/// // Integer seconds
/// assert_eq!(parse_retry_after("120"), Some(Duration::from_secs(120)));
///
/// // Zero seconds
/// assert_eq!(parse_retry_after("0"), Some(Duration::ZERO));
///
/// // Invalid format
/// assert_eq!(parse_retry_after("invalid"), None);
/// ```
#[must_use]
#[instrument]
pub fn parse_retry_after(header_value: &str) -> Option<Duration> {
    let header_value = header_value.trim();

    // Try parsing as integer seconds first (most common)
    if let Ok(seconds) = header_value.parse::<i64>() {
        if seconds < 0 {
            debug!(seconds, "negative Retry-After value, ignoring");
            return None;
        }

        #[allow(clippy::cast_sign_loss)]
        let duration = Duration::from_secs(seconds as u64);

        // Cap at maximum
        if duration > MAX_RETRY_AFTER {
            warn!(
                seconds,
                max_seconds = MAX_RETRY_AFTER.as_secs(),
                "Retry-After exceeds maximum, capping at 1 hour"
            );
            return Some(MAX_RETRY_AFTER);
        }

        return Some(duration);
    }

    // Try parsing as HTTP-date
    if let Ok(datetime) = httpdate::parse_http_date(header_value) {
        let now = std::time::SystemTime::now();

        // Calculate duration until the specified time
        if let Ok(duration) = datetime.duration_since(now) {
            // Cap at maximum
            if duration > MAX_RETRY_AFTER {
                warn!(
                    delay_secs = duration.as_secs(),
                    max_secs = MAX_RETRY_AFTER.as_secs(),
                    "Retry-After date exceeds maximum, capping at 1 hour"
                );
                return Some(MAX_RETRY_AFTER);
            }
            Some(duration)
        } else {
            // Date is in the past
            debug!(
                header_value,
                "Retry-After date is in the past, returning zero"
            );
            Some(Duration::ZERO)
        }
    } else {
        debug!(header_value, "unparseable Retry-After value");
        None
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    // ==================== RateLimiter Tests ====================

    #[test]
    fn test_rate_limiter_new_creates_with_delay() {
        let limiter = RateLimiter::new(Duration::from_millis(500));
        assert_eq!(limiter.default_delay(), Duration::from_millis(500));
        assert!(!limiter.is_disabled());
    }

    #[test]
    fn test_rate_limiter_disabled_has_zero_delay() {
        let limiter = RateLimiter::disabled();
        assert_eq!(limiter.default_delay(), Duration::ZERO);
        assert!(limiter.is_disabled());
    }

    #[tokio::test]
    async fn test_rate_limiter_disabled_no_delay() {
        // With paused time, we can verify no delay is applied
        tokio::time::pause();

        let limiter = RateLimiter::disabled();
        let start = Instant::now();

        limiter.acquire("https://example.com/1").await;
        limiter.acquire("https://example.com/2").await;
        limiter.acquire("https://example.com/3").await;

        // No time should have passed
        assert!(start.elapsed() < Duration::from_millis(10));
    }

    #[tokio::test]
    async fn test_rate_limiter_first_request_no_delay() {
        tokio::time::pause();

        let limiter = RateLimiter::new(Duration::from_secs(1));
        let start = Instant::now();

        // First request should be immediate
        limiter.acquire("https://example.com/file.pdf").await;

        assert!(start.elapsed() < Duration::from_millis(10));
    }

    #[tokio::test]
    async fn test_rate_limiter_delays_same_domain() {
        tokio::time::pause();

        let limiter = RateLimiter::new(Duration::from_secs(1));
        let start = Instant::now();

        // First request - immediate
        limiter.acquire("https://example.com/1").await;
        assert!(start.elapsed() < Duration::from_millis(10));

        // Second request - should delay 1 second
        limiter.acquire("https://example.com/2").await;
        assert!(start.elapsed() >= Duration::from_secs(1));
        assert!(start.elapsed() < Duration::from_millis(1100));

        // Third request - should delay another second
        limiter.acquire("https://example.com/3").await;
        assert!(start.elapsed() >= Duration::from_secs(2));
    }

    #[tokio::test]
    async fn test_rate_limiter_different_domains_independent() {
        tokio::time::pause();

        let limiter = RateLimiter::new(Duration::from_secs(1));

        // Request to first domain
        let start = Instant::now();
        limiter.acquire("https://example.com/file.pdf").await;
        assert!(start.elapsed() < Duration::from_millis(10));

        // Request to second domain - should be immediate
        let start2 = Instant::now();
        limiter.acquire("https://other.com/file.pdf").await;
        assert!(start2.elapsed() < Duration::from_millis(10));

        // Request to third domain - should be immediate
        let start3 = Instant::now();
        limiter.acquire("https://third.com/file.pdf").await;
        assert!(start3.elapsed() < Duration::from_millis(10));
    }

    #[tokio::test]
    async fn test_rate_limiter_tracks_domains_independently() {
        tokio::time::pause();

        let limiter = RateLimiter::new(Duration::from_secs(1));

        // Interleaved requests to two domains
        limiter.acquire("https://a.com/1").await;
        limiter.acquire("https://b.com/1").await;

        // Second request to each domain should delay
        let start_a = Instant::now();
        limiter.acquire("https://a.com/2").await;
        // Should wait ~1 second from first a.com request
        assert!(start_a.elapsed() >= Duration::from_millis(900));

        let start_b = Instant::now();
        limiter.acquire("https://b.com/2").await;
        // Should wait ~1 second from first b.com request
        // But some of that time already passed during a.com wait
        assert!(start_b.elapsed() < Duration::from_secs(1));
    }

    // ==================== extract_domain Tests ====================

    #[test]
    fn test_extract_domain_valid_https() {
        assert_eq!(
            extract_domain("https://example.com/path/file.pdf"),
            "example.com"
        );
    }

    #[test]
    fn test_extract_domain_valid_http() {
        assert_eq!(extract_domain("http://example.com/path"), "example.com");
    }

    #[test]
    fn test_extract_domain_lowercase() {
        assert_eq!(extract_domain("https://Example.COM/Path"), "example.com");
    }

    #[test]
    fn test_extract_domain_with_port() {
        assert_eq!(
            extract_domain("https://example.com:8080/path"),
            "example.com"
        );
    }

    #[test]
    fn test_extract_domain_ip_address() {
        assert_eq!(extract_domain("https://192.168.1.1/file"), "192.168.1.1");
    }

    #[test]
    fn test_extract_domain_localhost() {
        assert_eq!(extract_domain("https://localhost:8080/path"), "localhost");
    }

    #[test]
    fn test_extract_domain_malformed_url() {
        assert_eq!(extract_domain("not a valid url"), "unknown");
    }

    #[test]
    fn test_extract_domain_empty() {
        assert_eq!(extract_domain(""), "unknown");
    }

    #[test]
    fn test_extract_domain_subdomain() {
        assert_eq!(
            extract_domain("https://api.example.com/v1"),
            "api.example.com"
        );
    }

    // ==================== parse_retry_after Tests ====================

    #[test]
    fn test_parse_retry_after_seconds() {
        assert_eq!(parse_retry_after("120"), Some(Duration::from_secs(120)));
    }

    #[test]
    fn test_parse_retry_after_zero() {
        assert_eq!(parse_retry_after("0"), Some(Duration::ZERO));
    }

    #[test]
    fn test_parse_retry_after_negative() {
        assert_eq!(parse_retry_after("-5"), None);
    }

    #[test]
    fn test_parse_retry_after_invalid() {
        assert_eq!(parse_retry_after("invalid"), None);
    }

    #[test]
    fn test_parse_retry_after_empty() {
        assert_eq!(parse_retry_after(""), None);
    }

    #[test]
    fn test_parse_retry_after_whitespace() {
        assert_eq!(parse_retry_after("  120  "), Some(Duration::from_secs(120)));
    }

    #[test]
    fn test_parse_retry_after_caps_at_one_hour() {
        // 2 hours should be capped at 1 hour
        assert_eq!(parse_retry_after("7200"), Some(Duration::from_secs(3600)));
    }

    #[test]
    fn test_parse_retry_after_exactly_one_hour() {
        assert_eq!(parse_retry_after("3600"), Some(Duration::from_secs(3600)));
    }

    #[test]
    fn test_parse_retry_after_http_date_past() {
        // HTTP-date format with a date in the past returns zero
        let past_date = "Wed, 01 Jan 2020 00:00:00 GMT";
        assert_eq!(parse_retry_after(past_date), Some(Duration::ZERO));
    }

    #[test]
    fn test_parse_retry_after_http_date_future() {
        // HTTP-date format with a date in the future returns positive duration
        // Create a date 60 seconds in the future
        let future_time = std::time::SystemTime::now() + Duration::from_secs(60);
        let future_date = httpdate::fmt_http_date(future_time);

        let result = parse_retry_after(&future_date);
        assert!(result.is_some(), "Should parse future HTTP-date");

        let duration = result.unwrap();
        // Should be approximately 60 seconds (allow some tolerance for test execution time)
        assert!(
            duration >= Duration::from_secs(55) && duration <= Duration::from_secs(65),
            "Duration should be ~60s, got {:?}",
            duration
        );
    }

    // ==================== record_rate_limit Tests ====================

    #[test]
    fn test_record_rate_limit_tracks_cumulative() {
        let limiter = RateLimiter::new(Duration::from_secs(1));

        limiter.record_rate_limit("https://example.com/1", Duration::from_secs(5));
        limiter.record_rate_limit("https://example.com/2", Duration::from_secs(10));

        // Access the domain state to verify cumulative tracking
        let state = limiter.domains.get("example.com").unwrap();
        let cumulative = state.cumulative_delay_ms.load(Ordering::SeqCst);
        assert_eq!(cumulative, 15000); // 5s + 10s = 15s in milliseconds
    }

    #[test]
    fn test_record_rate_limit_different_domains() {
        let limiter = RateLimiter::new(Duration::from_secs(1));

        limiter.record_rate_limit("https://a.com/1", Duration::from_secs(5));
        limiter.record_rate_limit("https://b.com/1", Duration::from_secs(10));

        let state_a = limiter.domains.get("a.com").unwrap();
        let state_b = limiter.domains.get("b.com").unwrap();

        assert_eq!(state_a.cumulative_delay_ms.load(Ordering::SeqCst), 5000);
        assert_eq!(state_b.cumulative_delay_ms.load(Ordering::SeqCst), 10000);
    }
}
