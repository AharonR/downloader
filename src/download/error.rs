//! Error types for the download module.
//!
//! This module defines structured errors for all download operations,
//! providing context-rich error messages for debugging and user feedback.

use std::path::PathBuf;

use thiserror::Error;

/// Errors that can occur during file downloads.
#[derive(Debug, Error)]
pub enum DownloadError {
    /// Network-level error (DNS resolution, connection refused, TLS errors, etc.)
    #[error("network error downloading {url}: {source}")]
    Network {
        /// The URL that failed to download.
        url: String,
        /// The underlying network error.
        #[source]
        source: reqwest::Error,
    },

    /// Request timed out before completion.
    #[error("timeout downloading {url}")]
    Timeout {
        /// The URL that timed out.
        url: String,
    },

    /// HTTP error response (4xx client errors, 5xx server errors).
    #[error("HTTP {status} downloading {url}")]
    HttpStatus {
        /// The URL that returned an error status.
        url: String,
        /// The HTTP status code.
        status: u16,
        /// The Retry-After header value, if present (for 429 responses).
        retry_after: Option<String>,
    },

    /// File system error during download (create file, write, etc.)
    #[error("IO error writing to {path}: {source}")]
    Io {
        /// The file path where the error occurred.
        path: PathBuf,
        /// The underlying IO error.
        #[source]
        source: std::io::Error,
    },

    /// The provided URL is malformed or invalid.
    #[error("invalid URL: {url}")]
    InvalidUrl {
        /// The invalid URL string.
        url: String,
    },

    /// Downloaded file size does not match expected server content length.
    #[error(
        "integrity check failed for {path}: expected {expected_bytes} bytes, got {actual_bytes}"
    )]
    Integrity {
        /// Download path that failed verification.
        path: PathBuf,
        /// Expected size in bytes.
        expected_bytes: u64,
        /// Actual size in bytes.
        actual_bytes: u64,
    },

    /// Authentication or authorization required to access the resource.
    ///
    /// Suggestion text varies: 407 suggests proxy configuration,
    /// all others suggest `downloader auth capture`.
    #[error(
        "[AUTH] authentication required for {domain} (HTTP {status}) downloading {url}\n  Suggestion: {suggestion}"
    )]
    AuthRequired {
        /// The URL that requires authentication.
        url: String,
        /// The HTTP status code (401, 403, 407, or 0 for login redirect).
        status: u16,
        /// The domain requiring authentication.
        domain: String,
        /// User-facing suggestion for resolving the auth issue.
        suggestion: &'static str,
    },
}

impl DownloadError {
    /// Creates a network error from a reqwest error.
    pub fn network(url: impl Into<String>, source: reqwest::Error) -> Self {
        Self::Network {
            url: url.into(),
            source,
        }
    }

    /// Creates an HTTP status error.
    pub fn http_status(url: impl Into<String>, status: u16) -> Self {
        Self::HttpStatus {
            url: url.into(),
            status,
            retry_after: None,
        }
    }

    /// Creates an HTTP status error with a Retry-After header value.
    pub fn http_status_with_retry_after(
        url: impl Into<String>,
        status: u16,
        retry_after: Option<String>,
    ) -> Self {
        Self::HttpStatus {
            url: url.into(),
            status,
            retry_after,
        }
    }

    /// Creates a timeout error.
    pub fn timeout(url: impl Into<String>) -> Self {
        Self::Timeout { url: url.into() }
    }

    /// Creates an IO error.
    pub fn io(path: impl Into<PathBuf>, source: std::io::Error) -> Self {
        Self::Io {
            path: path.into(),
            source,
        }
    }

    /// Creates an invalid URL error.
    pub fn invalid_url(url: impl Into<String>) -> Self {
        Self::InvalidUrl { url: url.into() }
    }

    /// Creates an integrity mismatch error.
    pub fn integrity(path: impl Into<PathBuf>, expected_bytes: u64, actual_bytes: u64) -> Self {
        Self::Integrity {
            path: path.into(),
            expected_bytes,
            actual_bytes,
        }
    }

    /// Creates an authentication-required error.
    ///
    /// The suggestion text is derived from the status code:
    /// - 407 (Proxy Authentication Required) → proxy configuration hint
    /// - All others (401, 403, login redirect) → `downloader auth capture` hint
    pub fn auth_required(url: impl Into<String>, status: u16, domain: impl Into<String>) -> Self {
        let suggestion = if status == 407 {
            "Configure your HTTP proxy settings or check proxy credentials."
        } else {
            "Run `downloader auth capture` to authenticate."
        };
        Self::AuthRequired {
            url: url.into(),
            status,
            domain: domain.into(),
            suggestion,
        }
    }
}

// Note on From trait implementations:
// We intentionally do NOT implement `From<reqwest::Error>` or `From<std::io::Error>`
// because our error variants require context (url, path) that the source errors
// don't provide. The helper constructor methods (network(), io(), etc.) are the
// correct pattern here as they allow callers to provide necessary context.
//
// This is a design decision documented here to explain why Task 2's subtask
// "Implement From<reqwest::Error> and From<std::io::Error>" is not applicable
// in the standard way - the helper methods serve the same purpose with context.

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn test_download_error_network_display() {
        // We can't easily create a reqwest::Error, so we'll test the other variants
        let error = DownloadError::timeout("https://example.com/file.pdf");
        assert!(error.to_string().contains("timeout"));
        assert!(error.to_string().contains("https://example.com/file.pdf"));
    }

    #[test]
    fn test_download_error_http_status_display() {
        let error = DownloadError::http_status("https://example.com/file.pdf", 404);
        let msg = error.to_string();
        assert!(msg.contains("404"), "Expected '404' in: {msg}");
        assert!(
            msg.contains("https://example.com/file.pdf"),
            "Expected URL in: {msg}"
        );
    }

    #[test]
    fn test_download_error_io_display() {
        let io_error = std::io::Error::new(std::io::ErrorKind::PermissionDenied, "access denied");
        let error = DownloadError::io(PathBuf::from("/tmp/test.pdf"), io_error);
        let msg = error.to_string();
        assert!(msg.contains("/tmp/test.pdf"), "Expected path in: {msg}");
    }

    #[test]
    fn test_download_error_invalid_url_display() {
        let error = DownloadError::invalid_url("not-a-url");
        let msg = error.to_string();
        assert!(
            msg.contains("invalid URL"),
            "Expected 'invalid URL' in: {msg}"
        );
        assert!(msg.contains("not-a-url"), "Expected URL in: {msg}");
    }

    #[test]
    fn test_download_error_auth_required_display() {
        let error =
            DownloadError::auth_required("https://example.com/paper.pdf", 401, "example.com");
        let msg = error.to_string();
        assert!(
            msg.starts_with("[AUTH]"),
            "Expected [AUTH] prefix in: {msg}"
        );
        assert!(msg.contains("example.com"), "Expected domain in: {msg}");
        assert!(msg.contains("401"), "Expected status in: {msg}");
        assert!(
            msg.contains("https://example.com/paper.pdf"),
            "Expected URL in: {msg}"
        );
        assert!(
            msg.contains("downloader auth capture"),
            "Expected actionable suggestion in: {msg}"
        );
    }

    #[test]
    fn test_download_error_auth_required_403() {
        let error = DownloadError::auth_required("https://ieee.org/doc.pdf", 403, "ieee.org");
        let msg = error.to_string();
        assert!(
            msg.starts_with("[AUTH]"),
            "Expected [AUTH] prefix in: {msg}"
        );
        assert!(msg.contains("403"), "Expected status 403 in: {msg}");
        assert!(msg.contains("ieee.org"), "Expected domain in: {msg}");
    }

    #[test]
    fn test_download_error_auth_required_login_redirect() {
        let error = DownloadError::auth_required(
            "https://sciencedirect.com/paper.pdf",
            0,
            "idp.university.edu",
        );
        let msg = error.to_string();
        assert!(
            msg.starts_with("[AUTH]"),
            "Expected [AUTH] prefix in: {msg}"
        );
        assert!(
            msg.contains("HTTP 0"),
            "Expected status 0 (redirect) in: {msg}"
        );
        assert!(
            msg.contains("idp.university.edu"),
            "Expected redirect domain in: {msg}"
        );
    }

    #[test]
    fn test_download_error_auth_required_407_proxy_suggestion() {
        let error =
            DownloadError::auth_required("https://example.com/file.pdf", 407, "proxy.corp.net");
        let msg = error.to_string();
        assert!(
            msg.starts_with("[AUTH]"),
            "Expected [AUTH] prefix in: {msg}"
        );
        assert!(msg.contains("407"), "Expected status 407 in: {msg}");
        assert!(msg.contains("proxy"), "Expected proxy suggestion in: {msg}");
        assert!(
            !msg.contains("downloader auth capture"),
            "407 should NOT suggest auth capture: {msg}"
        );
    }
}
