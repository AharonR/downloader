//! Error types for input parsing operations.

use thiserror::Error;

/// Maximum URL length to accept (standard browser limit).
/// URLs longer than this are rejected to prevent memory issues.
pub const MAX_URL_LENGTH: usize = 2000;

/// Errors that can occur during input parsing.
#[derive(Debug, Clone, Error)]
pub enum ParseError {
    /// URL is malformed or uses unsupported scheme
    #[error("invalid URL '{url}': {reason}\n  Suggestion: {suggestion}")]
    InvalidUrl {
        /// The URL that failed validation
        url: String,
        /// Why the URL is invalid
        reason: String,
        /// How to fix the issue
        suggestion: String,
    },

    /// URL exceeds maximum allowed length
    #[error(
        "URL too long ({length} chars, max {max}): {url_preview}...\n  Suggestion: Use a URL shortener or check for extraneous content"
    )]
    UrlTooLong {
        /// Truncated URL for display
        url_preview: String,
        /// Actual length
        length: usize,
        /// Maximum allowed
        max: usize,
    },
}

impl ParseError {
    /// Creates an `InvalidUrl` error for a non-web URL scheme.
    #[must_use]
    pub fn unsupported_scheme(url: &str, scheme: &str) -> Self {
        Self::InvalidUrl {
            url: url.to_string(),
            reason: format!("scheme '{scheme}' is not supported"),
            suggestion: "Use http:// or https:// URLs".to_string(),
        }
    }

    /// Creates an `InvalidUrl` error for a malformed URL.
    #[must_use]
    pub fn malformed(url: &str, parse_error: &str) -> Self {
        Self::InvalidUrl {
            url: url.to_string(),
            reason: parse_error.to_string(),
            suggestion: "Check the URL format and try again".to_string(),
        }
    }

    /// Creates an `InvalidUrl` error for a URL without a host.
    #[must_use]
    pub fn no_host(url: &str) -> Self {
        Self::InvalidUrl {
            url: url.to_string(),
            reason: "URL has no host".to_string(),
            suggestion: "Ensure the URL includes a domain (e.g., example.com)".to_string(),
        }
    }

    /// Creates a `UrlTooLong` error for URLs exceeding the maximum length.
    #[must_use]
    pub fn too_long(url: &str) -> Self {
        Self::UrlTooLong {
            url_preview: url.chars().take(50).collect(),
            length: url.len(),
            max: MAX_URL_LENGTH,
        }
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_error_unsupported_scheme_message() {
        let err = ParseError::unsupported_scheme("ftp://example.com", "ftp");
        let msg = err.to_string();
        assert!(msg.contains("ftp://example.com"), "should contain URL");
        assert!(msg.contains("ftp"), "should contain scheme");
        assert!(msg.contains("http://"), "suggestion should mention http");
    }

    #[test]
    fn test_parse_error_malformed_message() {
        let err = ParseError::malformed("not-a-url", "relative URL without a base");
        let msg = err.to_string();
        assert!(msg.contains("not-a-url"), "should contain URL");
        assert!(msg.contains("relative URL"), "should contain reason");
        assert!(
            msg.contains("Check the URL format"),
            "should have suggestion"
        );
    }

    #[test]
    fn test_parse_error_no_host_message() {
        let err = ParseError::no_host("http:///path");
        let msg = err.to_string();
        assert!(msg.contains("no host"), "should mention no host");
        assert!(msg.contains("domain"), "suggestion should mention domain");
    }

    #[test]
    fn test_parse_error_too_long_message() {
        let long_url = "https://example.com/".to_string() + &"a".repeat(2500);
        let err = ParseError::too_long(&long_url);
        let msg = err.to_string();
        assert!(msg.contains("too long"), "should mention too long");
        assert!(msg.contains("2000"), "should mention max length");
        assert!(
            msg.contains("shortener"),
            "suggestion should mention shortener"
        );
    }

    #[test]
    fn test_parse_error_clone() {
        let err = ParseError::malformed("bad-url", "parse error");
        let cloned = err.clone();
        assert_eq!(err.to_string(), cloned.to_string());
    }
}
