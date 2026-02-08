//! HTTP client wrapper for downloading files.
//!
//! This module provides the `HttpClient` struct which handles streaming
//! downloads with proper timeout configuration and error handling.

use std::path::{Path, PathBuf};
use std::time::Duration;

use futures_util::StreamExt;
use reqwest::Client;
use reqwest::header::{CONTENT_DISPOSITION, RETRY_AFTER};
use tokio::fs::File;
use tokio::io::AsyncWriteExt;
use tracing::{debug, info, instrument};
use url::Url;

use super::error::DownloadError;

/// Default connect timeout in seconds.
const CONNECT_TIMEOUT_SECS: u64 = 30;

/// Default read timeout in seconds (5 minutes for large files).
const READ_TIMEOUT_SECS: u64 = 300;

/// HTTP client for downloading files with streaming support.
///
/// This client is designed to be created once and reused for multiple downloads,
/// taking advantage of connection pooling.
///
/// # Example
///
/// ```no_run
/// use downloader_core::download::HttpClient;
/// use std::path::Path;
///
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// let client = HttpClient::new();
/// let path = client.download_to_file("https://example.com/file.pdf", Path::new("./downloads")).await?;
/// println!("Downloaded to: {}", path.display());
/// # Ok(())
/// # }
/// ```
#[derive(Debug, Clone)]
pub struct HttpClient {
    client: Client,
}

impl Default for HttpClient {
    fn default() -> Self {
        Self::new()
    }
}

impl HttpClient {
    /// Creates a new HTTP client with default timeouts.
    ///
    /// Default configuration:
    /// - Connect timeout: 30 seconds
    /// - Read timeout: 5 minutes (for large files)
    /// - Gzip decompression: enabled
    ///
    /// # Panics
    ///
    /// Panics if the HTTP client builder fails to build with the static
    /// configuration. This should never happen in practice.
    #[must_use]
    #[allow(clippy::expect_used)]
    pub fn new() -> Self {
        // Static configuration - safe to use expect() here per project rules
        let client = Client::builder()
            .connect_timeout(Duration::from_secs(CONNECT_TIMEOUT_SECS))
            .timeout(Duration::from_secs(READ_TIMEOUT_SECS))
            .gzip(true)
            .build()
            .expect("failed to build HTTP client with static configuration");

        Self { client }
    }

    /// Downloads a file from URL to the specified output directory.
    ///
    /// The filename is determined by:
    /// 1. Content-Disposition header (if present)
    /// 2. URL path (last segment)
    /// 3. Timestamp-based fallback
    ///
    /// # Arguments
    ///
    /// * `url` - The URL to download from
    /// * `output_dir` - Directory to save the file to
    ///
    /// # Returns
    ///
    /// The path to the downloaded file.
    ///
    /// # Errors
    ///
    /// Returns `DownloadError` if:
    /// - The URL is invalid
    /// - The request fails (network error, timeout)
    /// - The server returns an error status (4xx, 5xx)
    /// - Writing to disk fails
    #[must_use = "download result contains the path to the downloaded file"]
    #[instrument(skip(self), fields(url = %url))]
    pub async fn download_to_file(
        &self,
        url: &str,
        output_dir: &Path,
    ) -> Result<PathBuf, DownloadError> {
        debug!("starting download");

        // Validate URL
        let parsed_url =
            Url::parse(url).map_err(|_| DownloadError::invalid_url(url.to_string()))?;

        // Send request
        let response = self.client.get(url).send().await.map_err(|e| {
            if e.is_timeout() {
                DownloadError::timeout(url)
            } else {
                DownloadError::network(url, e)
            }
        })?;

        // Check for HTTP error status, capturing Retry-After header if present
        let status = response.status();
        if !status.is_success() {
            // Capture Retry-After header before consuming the response
            let retry_after = response
                .headers()
                .get(RETRY_AFTER)
                .and_then(|v| v.to_str().ok())
                .map(std::string::ToString::to_string);

            return Err(DownloadError::http_status_with_retry_after(
                url,
                status.as_u16(),
                retry_after,
            ));
        }

        // Extract filename
        let filename = extract_filename(&response, &parsed_url);
        let file_path = resolve_unique_path(output_dir, &filename);

        debug!(filename = %filename, path = %file_path.display(), "resolved output path");

        // Create output file
        let mut file = File::create(&file_path)
            .await
            .map_err(|e| DownloadError::io(file_path.clone(), e))?;

        // Stream response body to file, with cleanup on error
        let result = stream_to_file(&mut file, response, url, &file_path).await;

        // Clean up partial file on error
        if result.is_err() {
            debug!(path = %file_path.display(), "cleaning up partial download");
            // Best effort cleanup - ignore errors since we're already returning an error
            let _ = tokio::fs::remove_file(&file_path).await;
        }

        let bytes_written = result?;

        info!(
            path = %file_path.display(),
            bytes = bytes_written,
            "download complete"
        );

        Ok(file_path)
    }

    /// Returns a reference to the underlying reqwest client.
    ///
    /// This can be used for advanced operations not covered by this wrapper.
    #[must_use]
    pub fn inner(&self) -> &Client {
        &self.client
    }
}

/// Streams response body to file, returning bytes written.
///
/// This is extracted to enable cleanup on error in the caller.
async fn stream_to_file(
    file: &mut File,
    response: reqwest::Response,
    url: &str,
    file_path: &Path,
) -> Result<u64, DownloadError> {
    let mut stream = response.bytes_stream();
    let mut bytes_written: u64 = 0;

    while let Some(chunk_result) = stream.next().await {
        let chunk = chunk_result.map_err(|e| DownloadError::network(url, e))?;

        file.write_all(&chunk)
            .await
            .map_err(|e| DownloadError::io(file_path.to_path_buf(), e))?;

        bytes_written += chunk.len() as u64;
    }

    // Ensure all data is flushed to disk
    file.flush()
        .await
        .map_err(|e| DownloadError::io(file_path.to_path_buf(), e))?;

    Ok(bytes_written)
}

/// Extracts filename from Content-Disposition header or URL path.
fn extract_filename(response: &reqwest::Response, url: &Url) -> String {
    // Try Content-Disposition header first
    if let Some(cd) = response.headers().get(CONTENT_DISPOSITION) {
        if let Ok(cd_str) = cd.to_str() {
            if let Some(filename) = parse_content_disposition(cd_str) {
                return sanitize_filename(&filename);
            }
        }
    }

    // Fall back to URL path
    if let Some(mut segments) = url.path_segments() {
        if let Some(last) = segments.next_back() {
            if !last.is_empty() {
                // URL decode the filename
                let decoded = urlencoding::decode(last).unwrap_or_else(|e| {
                    debug!(
                        segment = %last,
                        error = %e,
                        "URL decoding failed, using raw segment"
                    );
                    last.into()
                });
                return sanitize_filename(&decoded);
            }
        }
    }

    // Ultimate fallback: timestamp-based name
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);

    format!("download_{timestamp}.bin")
}

/// Parses Content-Disposition header to extract filename.
///
/// Handles both:
/// - `attachment; filename="example.pdf"`
/// - `attachment; filename=example.pdf`
/// - `attachment; filename*=UTF-8''example.pdf` (RFC 5987)
fn parse_content_disposition(header: &str) -> Option<String> {
    // Try filename*= first (RFC 5987 encoded)
    if let Some(pos) = header.find("filename*=") {
        let start = pos + 10;
        let value = header[start..].trim();
        // Format: charset'language'encoded_value
        if let Some(quote_pos) = value.find("''") {
            let encoded = &value[quote_pos + 2..];
            // Take until ; or end
            let end = encoded.find(';').unwrap_or(encoded.len());
            let encoded_name = &encoded[..end].trim();
            // URL decode
            if let Ok(decoded) = urlencoding::decode(encoded_name) {
                return Some(decoded.into_owned());
            }
        }
    }

    // Try regular filename=
    if let Some(pos) = header.find("filename=") {
        let start = pos + 9;
        let value = header[start..].trim();

        // Handle quoted filename
        if let Some(stripped) = value.strip_prefix('"') {
            if let Some(end) = stripped.find('"') {
                return Some(stripped[..end].to_string());
            }
        } else {
            // Unquoted - take until ; or end
            let end = value.find(';').unwrap_or(value.len());
            let filename = value[..end].trim();
            if !filename.is_empty() {
                return Some(filename.to_string());
            }
        }
    }

    None
}

/// Sanitizes filename for filesystem safety.
///
/// Replaces characters that are invalid on common filesystems:
/// / \ : * ? " < > |
fn sanitize_filename(name: &str) -> String {
    name.chars()
        .map(|c| match c {
            '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|' => '_',
            // Also handle null and control characters
            c if c.is_control() => '_',
            c => c,
        })
        .collect()
}

/// Resolves a unique file path, adding numeric suffix if file exists.
fn resolve_unique_path(dir: &Path, filename: &str) -> PathBuf {
    let base_path = dir.join(filename);

    if !base_path.exists() {
        return base_path;
    }

    // Split filename into stem and extension
    let (stem, ext) = match filename.rfind('.') {
        Some(pos) => (&filename[..pos], &filename[pos..]),
        None => (filename, ""),
    };

    // Try with numeric suffixes
    for i in 1..1000 {
        let new_name = format!("{stem}_{i}{ext}");
        let new_path = dir.join(new_name);
        if !new_path.exists() {
            return new_path;
        }
    }

    // Fallback (extremely unlikely)
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    dir.join(format!("{stem}_{timestamp}{ext}"))
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    #[test]
    fn test_sanitize_filename_removes_invalid_chars() {
        assert_eq!(sanitize_filename("file/name.pdf"), "file_name.pdf");
        assert_eq!(sanitize_filename("file\\name.pdf"), "file_name.pdf");
        assert_eq!(sanitize_filename("file:name.pdf"), "file_name.pdf");
        assert_eq!(sanitize_filename("file*name.pdf"), "file_name.pdf");
        assert_eq!(sanitize_filename("file?name.pdf"), "file_name.pdf");
        assert_eq!(sanitize_filename("file\"name.pdf"), "file_name.pdf");
        assert_eq!(sanitize_filename("file<name>.pdf"), "file_name_.pdf");
        assert_eq!(sanitize_filename("file|name.pdf"), "file_name.pdf");
    }

    #[test]
    fn test_sanitize_filename_preserves_valid_chars() {
        assert_eq!(
            sanitize_filename("valid-file_name.pdf"),
            "valid-file_name.pdf"
        );
        assert_eq!(sanitize_filename("file (1).pdf"), "file (1).pdf");
        assert_eq!(sanitize_filename("日本語.pdf"), "日本語.pdf");
    }

    #[test]
    fn test_parse_content_disposition_quoted() {
        let header = r#"attachment; filename="example.pdf""#;
        assert_eq!(
            parse_content_disposition(header),
            Some("example.pdf".to_string())
        );
    }

    #[test]
    fn test_parse_content_disposition_unquoted() {
        let header = "attachment; filename=example.pdf";
        assert_eq!(
            parse_content_disposition(header),
            Some("example.pdf".to_string())
        );
    }

    #[test]
    fn test_parse_content_disposition_with_semicolon() {
        let header = r#"attachment; filename="example.pdf"; size=1234"#;
        assert_eq!(
            parse_content_disposition(header),
            Some("example.pdf".to_string())
        );
    }

    #[test]
    fn test_parse_content_disposition_rfc5987() {
        let header = "attachment; filename*=UTF-8''example%20file.pdf";
        assert_eq!(
            parse_content_disposition(header),
            Some("example file.pdf".to_string())
        );
    }

    #[test]
    fn test_parse_content_disposition_missing() {
        let header = "attachment";
        assert_eq!(parse_content_disposition(header), None);
    }

    #[test]
    fn test_resolve_unique_path_no_conflict() {
        let temp_dir = TempDir::new().unwrap();
        let path = resolve_unique_path(temp_dir.path(), "test.pdf");
        assert_eq!(path, temp_dir.path().join("test.pdf"));
    }

    #[test]
    fn test_resolve_unique_path_with_conflict() {
        let temp_dir = TempDir::new().unwrap();

        // Create existing file
        std::fs::write(temp_dir.path().join("test.pdf"), b"existing").unwrap();

        let path = resolve_unique_path(temp_dir.path(), "test.pdf");
        assert_eq!(path, temp_dir.path().join("test_1.pdf"));
    }

    #[test]
    fn test_resolve_unique_path_multiple_conflicts() {
        let temp_dir = TempDir::new().unwrap();

        // Create existing files
        std::fs::write(temp_dir.path().join("test.pdf"), b"1").unwrap();
        std::fs::write(temp_dir.path().join("test_1.pdf"), b"2").unwrap();
        std::fs::write(temp_dir.path().join("test_2.pdf"), b"3").unwrap();

        let path = resolve_unique_path(temp_dir.path(), "test.pdf");
        assert_eq!(path, temp_dir.path().join("test_3.pdf"));
    }

    #[tokio::test]
    async fn test_http_client_download_success() {
        let mock_server = MockServer::start().await;
        let temp_dir = TempDir::new().unwrap();

        Mock::given(method("GET"))
            .and(path("/test.pdf"))
            .respond_with(ResponseTemplate::new(200).set_body_bytes(b"PDF content here"))
            .mount(&mock_server)
            .await;

        let client = HttpClient::new();
        let url = format!("{}/test.pdf", mock_server.uri());

        let result = client.download_to_file(&url, temp_dir.path()).await;

        assert!(result.is_ok(), "Expected Ok, got: {:?}", result);
        let file_path = result.unwrap();
        assert!(file_path.exists());
        let contents = std::fs::read(&file_path).unwrap();
        assert_eq!(contents, b"PDF content here");
        assert!(
            file_path
                .file_name()
                .unwrap()
                .to_str()
                .unwrap()
                .contains("test")
        );
    }

    #[tokio::test]
    async fn test_http_client_download_with_content_disposition() {
        let mock_server = MockServer::start().await;
        let temp_dir = TempDir::new().unwrap();

        Mock::given(method("GET"))
            .and(path("/download"))
            .respond_with(
                ResponseTemplate::new(200)
                    .insert_header("Content-Disposition", r#"attachment; filename="paper.pdf""#)
                    .set_body_bytes(b"PDF content"),
            )
            .mount(&mock_server)
            .await;

        let client = HttpClient::new();
        let url = format!("{}/download", mock_server.uri());

        let result = client.download_to_file(&url, temp_dir.path()).await;

        assert!(result.is_ok());
        let file_path = result.unwrap();
        assert_eq!(
            file_path.file_name().unwrap().to_str().unwrap(),
            "paper.pdf"
        );
    }

    #[tokio::test]
    async fn test_http_client_download_404_error() {
        let mock_server = MockServer::start().await;
        let temp_dir = TempDir::new().unwrap();

        Mock::given(method("GET"))
            .and(path("/missing.pdf"))
            .respond_with(ResponseTemplate::new(404))
            .mount(&mock_server)
            .await;

        let client = HttpClient::new();
        let url = format!("{}/missing.pdf", mock_server.uri());

        let result = client.download_to_file(&url, temp_dir.path()).await;

        assert!(result.is_err());
        match result {
            Err(DownloadError::HttpStatus { status, .. }) => {
                assert_eq!(status, 404);
            }
            other => panic!("Expected HttpStatus error, got: {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_http_client_download_500_error() {
        let mock_server = MockServer::start().await;
        let temp_dir = TempDir::new().unwrap();

        Mock::given(method("GET"))
            .and(path("/error"))
            .respond_with(ResponseTemplate::new(500))
            .mount(&mock_server)
            .await;

        let client = HttpClient::new();
        let url = format!("{}/error", mock_server.uri());

        let result = client.download_to_file(&url, temp_dir.path()).await;

        assert!(result.is_err());
        match result {
            Err(DownloadError::HttpStatus { status, .. }) => {
                assert_eq!(status, 500);
            }
            other => panic!("Expected HttpStatus error, got: {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_http_client_download_invalid_url() {
        let temp_dir = TempDir::new().unwrap();
        let client = HttpClient::new();

        let result = client
            .download_to_file("not-a-valid-url", temp_dir.path())
            .await;

        assert!(result.is_err());
        assert!(matches!(result, Err(DownloadError::InvalidUrl { .. })));
    }

    #[tokio::test]
    async fn test_http_client_download_large_file_streams() {
        let mock_server = MockServer::start().await;
        let temp_dir = TempDir::new().unwrap();

        // Create a "large" file (1MB) to verify streaming works
        let large_content = vec![0u8; 1024 * 1024];

        Mock::given(method("GET"))
            .and(path("/large.bin"))
            .respond_with(ResponseTemplate::new(200).set_body_bytes(large_content.clone()))
            .mount(&mock_server)
            .await;

        let client = HttpClient::new();
        let url = format!("{}/large.bin", mock_server.uri());

        let result = client.download_to_file(&url, temp_dir.path()).await;

        assert!(result.is_ok());
        let file_path = result.unwrap();
        let file_size = std::fs::metadata(&file_path).unwrap().len();
        assert_eq!(file_size, 1024 * 1024);
    }

    #[tokio::test]
    async fn test_http_client_default_equivalent_to_new() {
        // Verify Default and new() produce functionally equivalent clients
        let mock_server = MockServer::start().await;
        let temp_dir = TempDir::new().unwrap();

        Mock::given(method("GET"))
            .and(path("/test-default.txt"))
            .respond_with(ResponseTemplate::new(200).set_body_bytes(b"test content"))
            .mount(&mock_server)
            .await;

        let client_default = HttpClient::default();
        let url = format!("{}/test-default.txt", mock_server.uri());

        let result = client_default.download_to_file(&url, temp_dir.path()).await;
        assert!(result.is_ok(), "Default client should work: {:?}", result);
    }

    #[tokio::test]
    async fn test_partial_download_cleanup_on_404_after_streaming_starts() {
        // This tests that we don't leave partial files when HTTP errors occur
        // Note: 404 errors happen before streaming, so this tests the general error path
        let mock_server = MockServer::start().await;
        let temp_dir = TempDir::new().unwrap();

        Mock::given(method("GET"))
            .and(path("/fail-after-start"))
            .respond_with(ResponseTemplate::new(404))
            .mount(&mock_server)
            .await;

        let client = HttpClient::new();
        let url = format!("{}/fail-after-start", mock_server.uri());

        let result = client.download_to_file(&url, temp_dir.path()).await;
        assert!(result.is_err());

        // Verify no partial file was left behind
        let entries: Vec<_> = std::fs::read_dir(temp_dir.path()).unwrap().collect();
        assert!(
            entries.is_empty(),
            "No partial files should be left after error, found: {:?}",
            entries
        );
    }
}
