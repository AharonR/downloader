//! Retry logic with exponential backoff for transient download failures.
//!
//! This module provides the [`RetryPolicy`] and [`FailureType`] types for
//! classifying download errors and determining retry behavior.
//!
//! # Overview
//!
//! When a download fails, the error is classified into a [`FailureType`]:
//! - [`FailureType::Transient`] - Temporary failures that may succeed on retry
//! - [`FailureType::Permanent`] - Failures that won't succeed regardless of retries
//! - [`FailureType::NeedsAuth`] - Authentication required (treated as permanent until Epic 4)
//! - [`FailureType::RateLimited`] - Server rate limiting (retries with backoff)
//!
//! The [`RetryPolicy`] then determines whether to retry based on failure type
//! and attempt count, calculating exponential backoff delays with jitter.
//!
//! # Example
//!
//! ```
//! use downloader_core::download::{
//!     DownloadError, RetryPolicy, FailureType, RetryDecision, classify_error
//! };
//!
//! let policy = RetryPolicy::default();
//! let error = DownloadError::http_status("https://example.com/file.pdf", 503);
//! let failure_type = classify_error(&error);
//!
//! match policy.should_retry(failure_type, 1) {
//!     RetryDecision::Retry { delay, attempt } => {
//!         println!("Retrying in {:?} (attempt {})", delay, attempt);
//!     }
//!     RetryDecision::DoNotRetry { reason } => {
//!         println!("Not retrying: {}", reason);
//!     }
//! }
//! ```

use std::time::Duration;

use rand::Rng;
use tracing::{debug, instrument};

use super::DownloadError;

/// Default maximum retry attempts.
pub const DEFAULT_MAX_RETRIES: u32 = 3;

/// Default base delay for exponential backoff (1 second).
const DEFAULT_BASE_DELAY: Duration = Duration::from_secs(1);

/// Default maximum delay cap (32 seconds).
const DEFAULT_MAX_DELAY: Duration = Duration::from_secs(32);

/// Default backoff multiplier (doubles each attempt).
const DEFAULT_BACKOFF_MULTIPLIER: f32 = 2.0;

/// Maximum jitter added to delays (500ms).
const MAX_JITTER: Duration = Duration::from_millis(500);

/// Classification of download failure types.
///
/// Used to determine whether a failed download should be retried.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FailureType {
    /// Temporary failure that may succeed on retry.
    ///
    /// Examples: network timeout, 5xx server errors, connection refused.
    Transient,

    /// Permanent failure that won't succeed regardless of retries.
    ///
    /// Examples: 404 Not Found, 400 Bad Request, invalid URL.
    Permanent,

    /// Authentication or authorization required.
    ///
    /// Currently treated as permanent; will enable auth flow in Epic 4.
    NeedsAuth,

    /// Server rate limiting (HTTP 429).
    ///
    /// Retries with exponential backoff. Retry-After header support
    /// will be added in Story 1.7.
    RateLimited,
}

/// Decision on whether to retry a failed download.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RetryDecision {
    /// Retry the download after the specified delay.
    Retry {
        /// How long to wait before retrying.
        delay: Duration,
        /// Which attempt number this will be (1-indexed, so first retry is attempt 2).
        attempt: u32,
    },

    /// Do not retry the download.
    DoNotRetry {
        /// Human-readable reason why retry is not attempted.
        reason: String,
    },
}

/// Configuration for retry behavior with exponential backoff.
///
/// # Default Values
///
/// - `max_attempts`: 3
/// - `base_delay`: 1 second
/// - `max_delay`: 32 seconds
/// - `backoff_multiplier`: 2.0
///
/// # Delay Calculation
///
/// ```text
/// delay = min(base_delay * multiplier^attempt, max_delay) + jitter
/// ```
///
/// With defaults, delays are approximately: 1s, 2s, 4s (before hitting max attempts).
#[derive(Debug, Clone)]
pub struct RetryPolicy {
    /// Maximum number of attempts (including the initial attempt).
    max_attempts: u32,

    /// Base delay for the first retry.
    base_delay: Duration,

    /// Maximum delay cap.
    max_delay: Duration,

    /// Multiplier applied each attempt (typically 2.0 for doubling).
    backoff_multiplier: f32,
}

impl Default for RetryPolicy {
    fn default() -> Self {
        Self {
            max_attempts: DEFAULT_MAX_RETRIES,
            base_delay: DEFAULT_BASE_DELAY,
            max_delay: DEFAULT_MAX_DELAY,
            backoff_multiplier: DEFAULT_BACKOFF_MULTIPLIER,
        }
    }
}

impl RetryPolicy {
    /// Creates a new retry policy with custom settings.
    ///
    /// # Arguments
    ///
    /// * `max_attempts` - Maximum attempts including initial (must be >= 1)
    /// * `base_delay` - Base delay for first retry
    /// * `max_delay` - Maximum delay cap
    /// * `backoff_multiplier` - Multiplier for exponential increase
    #[must_use]
    pub fn new(
        max_attempts: u32,
        base_delay: Duration,
        max_delay: Duration,
        backoff_multiplier: f32,
    ) -> Self {
        Self {
            max_attempts: max_attempts.max(1),
            base_delay,
            max_delay,
            backoff_multiplier,
        }
    }

    /// Creates a policy with a custom max_attempts, using defaults for other settings.
    #[must_use]
    pub fn with_max_attempts(max_attempts: u32) -> Self {
        Self {
            max_attempts: max_attempts.max(1),
            ..Self::default()
        }
    }

    /// Returns the maximum number of attempts configured.
    #[must_use]
    pub fn max_attempts(&self) -> u32 {
        self.max_attempts
    }

    /// Determines whether to retry a failed download.
    ///
    /// # Arguments
    ///
    /// * `failure_type` - Classification of the failure
    /// * `attempt` - The attempt number that just failed (1-indexed)
    ///
    /// # Returns
    ///
    /// A [`RetryDecision`] indicating whether to retry and with what delay.
    #[instrument(skip(self), fields(max_attempts = self.max_attempts))]
    pub fn should_retry(&self, failure_type: FailureType, attempt: u32) -> RetryDecision {
        // Check if failure type is retryable
        match failure_type {
            FailureType::Permanent => {
                return RetryDecision::DoNotRetry {
                    reason: "permanent failure - retry would not help".to_string(),
                };
            }
            FailureType::NeedsAuth => {
                return RetryDecision::DoNotRetry {
                    reason: "authentication required - retry without auth would not help"
                        .to_string(),
                };
            }
            FailureType::Transient | FailureType::RateLimited => {
                // These are retryable, continue to attempt check
            }
        }

        // Check if we've exhausted attempts
        if attempt >= self.max_attempts {
            debug!(attempt, max = self.max_attempts, "max attempts reached");
            return RetryDecision::DoNotRetry {
                reason: format!("max attempts ({}) exhausted", self.max_attempts),
            };
        }

        // Calculate delay with exponential backoff
        let delay = self.calculate_delay(attempt);

        debug!(
            attempt,
            next_attempt = attempt + 1,
            delay_ms = delay.as_millis(),
            "will retry"
        );

        RetryDecision::Retry {
            delay,
            attempt: attempt + 1,
        }
    }

    /// Calculates the delay for a retry attempt with exponential backoff and jitter.
    ///
    /// Formula: `min(base_delay * multiplier^attempt, max_delay) + jitter`
    fn calculate_delay(&self, attempt: u32) -> Duration {
        let base_ms = self.base_delay.as_millis() as f64;
        let multiplier = self.backoff_multiplier as f64;

        // Exponential: base * multiplier^attempt
        // attempt is 0-indexed for the exponent (attempt 1 = 2^0 = 1x base)
        let exponent = (attempt - 1) as f64;
        let delay_ms = base_ms * multiplier.powf(exponent);

        // Cap at max_delay
        let capped_ms = delay_ms.min(self.max_delay.as_millis() as f64);

        // Add jitter
        let jitter = self.calculate_jitter();

        Duration::from_millis(capped_ms as u64) + jitter
    }

    /// Generates random jitter between 0 and MAX_JITTER.
    ///
    /// Jitter helps prevent thundering herd when multiple downloads
    /// fail simultaneously and retry at the same time.
    fn calculate_jitter(&self) -> Duration {
        let mut rng = rand::thread_rng();
        let jitter_ms = rng.gen_range(0..=MAX_JITTER.as_millis() as u64);
        Duration::from_millis(jitter_ms)
    }
}

/// Classifies a download error into a failure type for retry decisions.
///
/// # HTTP Status Code Classification
///
/// | Status | Type | Rationale |
/// |--------|------|-----------|
/// | 400 | Permanent | Bad request - won't succeed on retry |
/// | 401 | NeedsAuth | Unauthorized - needs authentication |
/// | 403 | NeedsAuth | Forbidden - needs authentication |
/// | 404 | Permanent | Not found - resource doesn't exist |
/// | 408 | Transient | Request timeout - may succeed |
/// | 410 | Permanent | Gone - permanently removed |
/// | 429 | RateLimited | Rate limited - retry with backoff |
/// | 451 | Permanent | Legal block - won't succeed |
/// | 500 | Transient | Server error - may be temporary |
/// | 502 | Transient | Bad gateway - proxy issue |
/// | 503 | Transient | Service unavailable - temporary |
/// | 504 | Transient | Gateway timeout - temporary |
///
/// # Non-HTTP Errors
///
/// | Error | Type | Rationale |
/// |-------|------|-----------|
/// | Timeout | Transient | Network may recover |
/// | Network (most) | Transient | Server may come back |
/// | Network (TLS) | Permanent | Certificate/config issue |
/// | IO | Permanent | Local file system issue |
/// | InvalidUrl | Permanent | Won't succeed |
#[instrument]
pub fn classify_error(error: &DownloadError) -> FailureType {
    match error {
        DownloadError::HttpStatus { status, .. } => classify_http_status(*status),

        DownloadError::Timeout { .. } => FailureType::Transient,

        DownloadError::Network { source, .. } => {
            // Check if it's a TLS/certificate error which is permanent
            if is_tls_error(source) {
                FailureType::Permanent
            } else {
                FailureType::Transient
            }
        }

        DownloadError::Io { .. } => FailureType::Permanent,

        DownloadError::InvalidUrl { .. } => FailureType::Permanent,
    }
}

/// Classifies an HTTP status code into a failure type.
///
/// Explicit match arms are used for each status code for documentation purposes,
/// even though some return the same value.
#[allow(clippy::match_same_arms)]
fn classify_http_status(status: u16) -> FailureType {
    match status {
        // Client errors - mostly permanent
        400 => FailureType::Permanent,   // Bad Request
        401 => FailureType::NeedsAuth,   // Unauthorized
        403 => FailureType::NeedsAuth,   // Forbidden
        404 => FailureType::Permanent,   // Not Found
        408 => FailureType::Transient,   // Request Timeout
        410 => FailureType::Permanent,   // Gone
        429 => FailureType::RateLimited, // Too Many Requests
        451 => FailureType::Permanent,   // Unavailable For Legal Reasons

        // Server errors - transient
        500 => FailureType::Transient, // Internal Server Error
        502 => FailureType::Transient, // Bad Gateway
        503 => FailureType::Transient, // Service Unavailable
        504 => FailureType::Transient, // Gateway Timeout

        // Other 4xx are generally permanent
        status if (400..500).contains(&status) => FailureType::Permanent,

        // Other 5xx are generally transient
        status if (500..600).contains(&status) => FailureType::Transient,

        // Anything else is unexpected, treat as permanent
        _ => FailureType::Permanent,
    }
}

/// Checks if a reqwest error is a TLS/certificate error.
fn is_tls_error(error: &reqwest::Error) -> bool {
    // reqwest errors have methods to check error type
    // TLS errors typically appear in the error chain
    let error_string = error.to_string().to_lowercase();
    error_string.contains("certificate")
        || error_string.contains("tls")
        || error_string.contains("ssl")
        || error_string.contains("handshake")
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    // ==================== RetryPolicy Tests ====================

    #[test]
    fn test_retry_policy_default_values() {
        let policy = RetryPolicy::default();
        assert_eq!(policy.max_attempts, 3);
        assert_eq!(policy.base_delay, Duration::from_secs(1));
        assert_eq!(policy.max_delay, Duration::from_secs(32));
        assert!((policy.backoff_multiplier - 2.0).abs() < f32::EPSILON);
    }

    #[test]
    fn test_retry_policy_with_max_attempts() {
        let policy = RetryPolicy::with_max_attempts(5);
        assert_eq!(policy.max_attempts(), 5);
        // Other values should be defaults
        assert_eq!(policy.base_delay, Duration::from_secs(1));
    }

    #[test]
    fn test_retry_policy_max_attempts_minimum_is_one() {
        let policy = RetryPolicy::with_max_attempts(0);
        assert_eq!(policy.max_attempts(), 1);
    }

    #[test]
    fn test_retry_policy_custom() {
        let policy = RetryPolicy::new(5, Duration::from_millis(500), Duration::from_secs(60), 3.0);
        assert_eq!(policy.max_attempts, 5);
        assert_eq!(policy.base_delay, Duration::from_millis(500));
        assert_eq!(policy.max_delay, Duration::from_secs(60));
        assert!((policy.backoff_multiplier - 3.0).abs() < f32::EPSILON);
    }

    // ==================== Delay Calculation Tests ====================

    #[test]
    fn test_delay_calculation_first_attempt() {
        let policy = RetryPolicy::new(5, Duration::from_secs(1), Duration::from_secs(32), 2.0);
        // First attempt (attempt=1): base * 2^0 = 1s + jitter
        let delay = policy.calculate_delay(1);
        // Should be between 1000ms and 1500ms (base + up to 500ms jitter)
        assert!(delay >= Duration::from_secs(1));
        assert!(delay <= Duration::from_millis(1500));
    }

    #[test]
    fn test_delay_calculation_second_attempt() {
        let policy = RetryPolicy::new(5, Duration::from_secs(1), Duration::from_secs(32), 2.0);
        // Second attempt (attempt=2): base * 2^1 = 2s + jitter
        let delay = policy.calculate_delay(2);
        assert!(delay >= Duration::from_secs(2));
        assert!(delay <= Duration::from_millis(2500));
    }

    #[test]
    fn test_delay_calculation_third_attempt() {
        let policy = RetryPolicy::new(5, Duration::from_secs(1), Duration::from_secs(32), 2.0);
        // Third attempt (attempt=3): base * 2^2 = 4s + jitter
        let delay = policy.calculate_delay(3);
        assert!(delay >= Duration::from_secs(4));
        assert!(delay <= Duration::from_millis(4500));
    }

    #[test]
    fn test_delay_calculation_respects_max_delay() {
        let policy = RetryPolicy::new(
            10,
            Duration::from_secs(1),
            Duration::from_secs(5), // Low max
            2.0,
        );
        // 6th attempt would be 1 * 2^5 = 32s, but capped at 5s
        let delay = policy.calculate_delay(6);
        assert!(delay >= Duration::from_secs(5));
        assert!(delay <= Duration::from_millis(5500));
    }

    // ==================== Jitter Tests ====================

    #[test]
    fn test_jitter_within_bounds() {
        let policy = RetryPolicy::default();
        // Test 100 samples to verify bounds
        for _ in 0..100 {
            let jitter = policy.calculate_jitter();
            assert!(
                jitter <= MAX_JITTER,
                "Jitter {} exceeds max",
                jitter.as_millis()
            );
        }
    }

    #[test]
    fn test_jitter_distribution() {
        let policy = RetryPolicy::default();
        let samples: Vec<Duration> = (0..100).map(|_| policy.calculate_jitter()).collect();

        // All samples in valid range
        assert!(samples.iter().all(|d| d.as_millis() <= 500));

        // Mean should be roughly 250ms (within 100ms tolerance for randomness)
        let mean_ms = samples.iter().map(|d| d.as_millis()).sum::<u128>() / 100;
        assert!(
            (150..350).contains(&mean_ms),
            "Jitter mean {}ms not near 250ms (expected 150-350ms range)",
            mean_ms
        );
    }

    // ==================== Error Classification Tests ====================

    #[test]
    fn test_classify_http_400_permanent() {
        let error = DownloadError::http_status("http://example.com", 400);
        assert_eq!(classify_error(&error), FailureType::Permanent);
    }

    #[test]
    fn test_classify_http_401_needs_auth() {
        let error = DownloadError::http_status("http://example.com", 401);
        assert_eq!(classify_error(&error), FailureType::NeedsAuth);
    }

    #[test]
    fn test_classify_http_403_needs_auth() {
        let error = DownloadError::http_status("http://example.com", 403);
        assert_eq!(classify_error(&error), FailureType::NeedsAuth);
    }

    #[test]
    fn test_classify_http_404_permanent() {
        let error = DownloadError::http_status("http://example.com", 404);
        assert_eq!(classify_error(&error), FailureType::Permanent);
    }

    #[test]
    fn test_classify_http_408_transient() {
        let error = DownloadError::http_status("http://example.com", 408);
        assert_eq!(classify_error(&error), FailureType::Transient);
    }

    #[test]
    fn test_classify_http_410_permanent() {
        let error = DownloadError::http_status("http://example.com", 410);
        assert_eq!(classify_error(&error), FailureType::Permanent);
    }

    #[test]
    fn test_classify_http_429_rate_limited() {
        let error = DownloadError::http_status("http://example.com", 429);
        assert_eq!(classify_error(&error), FailureType::RateLimited);
    }

    #[test]
    fn test_classify_http_451_permanent() {
        let error = DownloadError::http_status("http://example.com", 451);
        assert_eq!(classify_error(&error), FailureType::Permanent);
    }

    #[test]
    fn test_classify_http_500_transient() {
        let error = DownloadError::http_status("http://example.com", 500);
        assert_eq!(classify_error(&error), FailureType::Transient);
    }

    #[test]
    fn test_classify_http_502_transient() {
        let error = DownloadError::http_status("http://example.com", 502);
        assert_eq!(classify_error(&error), FailureType::Transient);
    }

    #[test]
    fn test_classify_http_503_transient() {
        let error = DownloadError::http_status("http://example.com", 503);
        assert_eq!(classify_error(&error), FailureType::Transient);
    }

    #[test]
    fn test_classify_http_504_transient() {
        let error = DownloadError::http_status("http://example.com", 504);
        assert_eq!(classify_error(&error), FailureType::Transient);
    }

    #[test]
    fn test_classify_timeout_transient() {
        let error = DownloadError::timeout("http://example.com");
        assert_eq!(classify_error(&error), FailureType::Transient);
    }

    #[test]
    fn test_classify_invalid_url_permanent() {
        let error = DownloadError::invalid_url("not-a-url");
        assert_eq!(classify_error(&error), FailureType::Permanent);
    }

    #[test]
    fn test_classify_io_error_permanent() {
        let io_err = std::io::Error::new(std::io::ErrorKind::PermissionDenied, "denied");
        let error = DownloadError::io("/path/to/file", io_err);
        assert_eq!(classify_error(&error), FailureType::Permanent);
    }

    // ==================== Should Retry Decision Tests ====================

    #[test]
    fn test_should_retry_permanent_does_not_retry() {
        let policy = RetryPolicy::default();
        let decision = policy.should_retry(FailureType::Permanent, 1);
        assert!(matches!(decision, RetryDecision::DoNotRetry { .. }));
        if let RetryDecision::DoNotRetry { reason } = decision {
            assert!(reason.contains("permanent"));
        }
    }

    #[test]
    fn test_should_retry_needs_auth_does_not_retry() {
        let policy = RetryPolicy::default();
        let decision = policy.should_retry(FailureType::NeedsAuth, 1);
        assert!(matches!(decision, RetryDecision::DoNotRetry { .. }));
        if let RetryDecision::DoNotRetry { reason } = decision {
            assert!(reason.contains("auth"));
        }
    }

    #[test]
    fn test_should_retry_transient_retries() {
        let policy = RetryPolicy::default();
        let decision = policy.should_retry(FailureType::Transient, 1);
        assert!(matches!(decision, RetryDecision::Retry { .. }));
        if let RetryDecision::Retry { attempt, .. } = decision {
            assert_eq!(attempt, 2);
        }
    }

    #[test]
    fn test_should_retry_rate_limited_retries() {
        let policy = RetryPolicy::default();
        let decision = policy.should_retry(FailureType::RateLimited, 1);
        assert!(matches!(decision, RetryDecision::Retry { .. }));
    }

    #[test]
    fn test_should_retry_respects_max_attempts() {
        let policy = RetryPolicy::with_max_attempts(3);

        // Attempt 1 should retry
        let decision = policy.should_retry(FailureType::Transient, 1);
        assert!(matches!(decision, RetryDecision::Retry { .. }));

        // Attempt 2 should retry
        let decision = policy.should_retry(FailureType::Transient, 2);
        assert!(matches!(decision, RetryDecision::Retry { .. }));

        // Attempt 3 (max) should not retry
        let decision = policy.should_retry(FailureType::Transient, 3);
        assert!(matches!(decision, RetryDecision::DoNotRetry { .. }));
        if let RetryDecision::DoNotRetry { reason } = decision {
            assert!(reason.contains("exhausted"));
        }
    }

    #[test]
    fn test_should_retry_delay_increases() {
        let policy = RetryPolicy::default();

        let decision1 = policy.should_retry(FailureType::Transient, 1);
        let decision2 = policy.should_retry(FailureType::Transient, 2);

        if let (
            RetryDecision::Retry { delay: delay1, .. },
            RetryDecision::Retry { delay: delay2, .. },
        ) = (decision1, decision2)
        {
            // delay2 should be approximately double delay1 (accounting for jitter)
            // delay1 is ~1s + jitter, delay2 is ~2s + jitter
            // So delay2 should be at least 1.5x delay1 (conservative check)
            assert!(
                delay2 > delay1,
                "delay2 ({:?}) should be greater than delay1 ({:?})",
                delay2,
                delay1
            );
        } else {
            panic!("Expected both to be Retry decisions");
        }
    }

    // ==================== Constants Tests ====================

    #[test]
    fn test_default_max_retries_constant() {
        assert_eq!(DEFAULT_MAX_RETRIES, 3);
    }
}
