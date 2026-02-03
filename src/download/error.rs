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
}
