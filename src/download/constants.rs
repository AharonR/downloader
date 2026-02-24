//! Constants for the download module (timeouts, rate limiting).

use std::time::Duration;

/// Default HTTP connect timeout (30 seconds).
pub const CONNECT_TIMEOUT_SECS: u64 = 30;

/// Default HTTP read timeout (5 minutes for large files).
pub const READ_TIMEOUT_SECS: u64 = 300;

/// Warning threshold for cumulative rate limit delay per domain (30 seconds).
pub const CUMULATIVE_DELAY_WARNING_THRESHOLD: Duration = Duration::from_secs(30);

/// Maximum Retry-After header value (1 hour) to prevent excessive delays.
pub const MAX_RETRY_AFTER: Duration = Duration::from_secs(3600);
