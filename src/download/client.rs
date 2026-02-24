//! HTTP client wrapper for downloading files.
//!
//! This module provides the `HttpClient` struct which handles streaming
//! downloads with proper timeout configuration and error handling.

use std::panic::{AssertUnwindSafe, catch_unwind, set_hook, take_hook};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use futures_util::StreamExt;
use reqwest::Client;
use reqwest::cookie::Jar;
use reqwest::header::{ACCEPT_RANGES, CONTENT_DISPOSITION, CONTENT_LENGTH, RANGE, RETRY_AFTER};
use reqwest::{ClientBuilder, Proxy};
use tokio::fs::File;
use tokio::io::{AsyncSeekExt, AsyncWriteExt, BufWriter};
use tracing::{debug, info, instrument, warn};
use url::Url;

use super::constants::{CONNECT_TIMEOUT_SECS, READ_TIMEOUT_SECS};
use super::error::DownloadError;
use super::filename::{
    extension_from_content_type, fallback_filename_from_url, parse_content_disposition,
    resolve_unique_path, resolve_unique_path_with_suffix_start, sanitize_filename,
};
use crate::user_agent;

/// Browser User-Agent used as fallback when servers return 403.
///
/// The client sends a default User-Agent identifying the tool on the first attempt.
/// If the server responds with 403 (e.g. bot-detection), the engine retries once
/// with this browser-like User-Agent before giving up.
pub const BROWSER_USER_AGENT: &str = "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) \
    AppleWebKit/537.36 (KHTML, like Gecko) Chrome/131.0.0.0 Safari/537.36";

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

/// Download metadata for progress reporting and resumable state persistence.
#[derive(Debug, Clone)]
pub struct DownloadFileResult {
    /// Final output path.
    pub path: PathBuf,
    /// Current file size after download completes.
    pub bytes_downloaded: u64,
    /// Expected file size when known.
    pub content_length: Option<u64>,
    /// Whether an HTTP range resume was used.
    pub resumed: bool,
    /// Whether a resume attempt was made (even if server rejected it).
    pub resume_attempted: bool,
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
        Self::new_with_timeouts(CONNECT_TIMEOUT_SECS, READ_TIMEOUT_SECS)
    }

    /// Creates a new HTTP client with explicit timeout values.
    ///
    /// # Panics
    ///
    /// Panics if the HTTP client builder fails to build with the supplied
    /// timeout configuration.
    #[must_use]
    #[allow(clippy::expect_used)]
    pub fn new_with_timeouts(connect_timeout_secs: u64, read_timeout_secs: u64) -> Self {
        let client = build_client(None, connect_timeout_secs, read_timeout_secs)
            .expect("failed to build HTTP client with static configuration");
        Self { client }
    }

    /// Creates a new HTTP client with a cookie jar for authenticated downloads.
    ///
    /// Cookies in the jar are automatically attached to matching requests
    /// based on domain, path, and secure flag.
    ///
    /// # Panics
    ///
    /// Panics if the HTTP client builder fails to build with the static
    /// configuration. This should never happen in practice.
    #[must_use]
    #[allow(clippy::expect_used)]
    #[instrument(level = "debug", skip(cookie_jar))]
    pub fn with_cookie_jar(cookie_jar: Arc<Jar>) -> Self {
        Self::with_cookie_jar_and_timeouts(cookie_jar, CONNECT_TIMEOUT_SECS, READ_TIMEOUT_SECS)
    }

    /// Creates a new HTTP client with a cookie jar and explicit timeout values.
    ///
    /// # Panics
    ///
    /// Panics if the HTTP client builder fails to build with the supplied
    /// timeout configuration.
    #[must_use]
    #[allow(clippy::expect_used)]
    #[instrument(level = "debug", skip(cookie_jar))]
    pub fn with_cookie_jar_and_timeouts(
        cookie_jar: Arc<Jar>,
        connect_timeout_secs: u64,
        read_timeout_secs: u64,
    ) -> Self {
        let client = build_client(Some(cookie_jar), connect_timeout_secs, read_timeout_secs)
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
        Ok(self
            .download_to_file_with_metadata(url, output_dir)
            .await?
            .path)
    }

    /// Downloads a file and returns metadata needed for resumable queue state.
    ///
    /// # Errors
    ///
    /// Returns the same errors as [`download_to_file`](Self::download_to_file).
    #[must_use = "download result contains path and progress metadata"]
    #[instrument(skip(self), fields(url = %url))]
    pub async fn download_to_file_with_metadata(
        &self,
        url: &str,
        output_dir: &Path,
    ) -> Result<DownloadFileResult, DownloadError> {
        self.download_to_file_with_metadata_and_name(url, output_dir, None, None)
            .await
    }

    /// Downloads a file and optionally applies a preferred filename.
    ///
    /// When `preferred_filename` is provided, the save path is based on that
    /// name instead of response headers/URL, and duplicate suffixes start at `_2`.
    ///
    /// # Errors
    ///
    /// Returns the same errors as [`download_to_file`](Self::download_to_file).
    #[must_use = "download result contains path and progress metadata"]
    #[instrument(skip(self), fields(url = %url))]
    pub async fn download_to_file_with_metadata_and_name(
        &self,
        url: &str,
        output_dir: &Path,
        preferred_filename: Option<&str>,
        resume_bytes_hint: Option<u64>,
    ) -> Result<DownloadFileResult, DownloadError> {
        self.download_to_file_inner(url, output_dir, None, preferred_filename, resume_bytes_hint)
            .await
    }

    /// Downloads a file using a custom User-Agent header.
    ///
    /// Behaves identically to [`download_to_file_with_metadata`](Self::download_to_file_with_metadata)
    /// but overrides the request User-Agent. Used by the engine as a fallback
    /// when servers return 403 for the default UA.
    ///
    /// # Errors
    ///
    /// Returns the same errors as [`download_to_file`](Self::download_to_file).
    #[must_use = "download result contains path and progress metadata"]
    #[instrument(skip(self), fields(url = %url))]
    pub async fn download_to_file_with_user_agent(
        &self,
        url: &str,
        output_dir: &Path,
        user_agent: &str,
    ) -> Result<DownloadFileResult, DownloadError> {
        self.download_to_file_with_user_agent_and_name(url, output_dir, user_agent, None, None)
            .await
    }

    /// Downloads a file with a custom User-Agent and optional preferred filename.
    ///
    /// # Errors
    ///
    /// Returns the same errors as [`download_to_file`](Self::download_to_file).
    #[must_use = "download result contains path and progress metadata"]
    #[instrument(skip(self), fields(url = %url))]
    pub async fn download_to_file_with_user_agent_and_name(
        &self,
        url: &str,
        output_dir: &Path,
        user_agent: &str,
        preferred_filename: Option<&str>,
        resume_bytes_hint: Option<u64>,
    ) -> Result<DownloadFileResult, DownloadError> {
        self.download_to_file_inner(
            url,
            output_dir,
            Some(user_agent),
            preferred_filename,
            resume_bytes_hint,
        )
        .await
    }

    /// Inner implementation shared by both download methods.
    async fn download_to_file_inner(
        &self,
        url: &str,
        output_dir: &Path,
        user_agent: Option<&str>,
        preferred_filename: Option<&str>,
        resume_bytes_hint: Option<u64>,
    ) -> Result<DownloadFileResult, DownloadError> {
        debug!("starting download");

        // Validate URL
        let parsed_url =
            Url::parse(url).map_err(|_| DownloadError::invalid_url(url.to_string()))?;

        let url_filename = fallback_filename_from_url(&parsed_url);
        let preferred_filename = preferred_filename
            .map(sanitize_filename)
            .filter(|name| !name.is_empty());
        let candidate_name = preferred_filename
            .clone()
            .unwrap_or_else(|| url_filename.clone());
        let candidate_path = output_dir.join(&candidate_name);

        let expected_partial_bytes = resume_bytes_hint.unwrap_or(0);
        let allow_resume = preferred_filename.is_none() || expected_partial_bytes > 0;
        let (existing_bytes, supports_ranges, resume_attempted) = self
            .determine_resume_state(
                &candidate_path,
                url,
                user_agent,
                preferred_filename.is_some(),
                allow_resume,
                expected_partial_bytes,
            )
            .await;

        let use_resume = supports_ranges && existing_bytes > 0;
        let range_value = use_resume.then(|| format!("bytes={existing_bytes}-"));

        // Send GET request, optionally with Range and User-Agent overrides.
        let response = self
            .send_request("GET", url, user_agent, range_value.as_deref())
            .await?;

        let response_status = response.status();
        let response_filename = extract_filename(&response, &parsed_url);
        // For resume: use the existing partial file path directly.
        // For fresh downloads: resolve a unique path from the response filename.
        let file_path = if use_resume && response_status.as_u16() == 206 {
            candidate_path
        } else if let Some(preferred) = preferred_filename {
            resolve_unique_path_with_suffix_start(output_dir, &preferred, 2)
        } else {
            resolve_unique_path(output_dir, &response_filename)
        };
        debug!(filename = %response_filename, path = %file_path.display(), "resolved output path");

        // Open output file (append for true resume, create/truncate otherwise)
        let mut file = if use_resume && response_status.as_u16() == 206 {
            let mut handle = tokio::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(&file_path)
                .await
                .map_err(|e| DownloadError::io(file_path.clone(), e))?;
            handle
                .seek(std::io::SeekFrom::End(0))
                .await
                .map_err(|e| DownloadError::io(file_path.clone(), e))?;
            handle
        } else {
            File::create(&file_path)
                .await
                .map_err(|e| DownloadError::io(file_path.clone(), e))?
        };

        let content_length = derive_total_content_length(&response, existing_bytes);

        // Stream response body to file, with cleanup on error
        let stream_result = stream_to_file(&mut file, response, url, &file_path).await;

        if stream_result.is_err() {
            // If the download failed and we weren't resuming (or the server didn't support resume),
            // clean up the partial file to avoid leaving incomplete data.
            if response_status.as_u16() != 206 {
                debug!(path = %file_path.display(), "cleaning up partial file after error");
                let _ = tokio::fs::remove_file(&file_path).await;
            }
        }

        let bytes_written = stream_result?;

        let final_size = if use_resume && response_status.as_u16() == 206 {
            existing_bytes.saturating_add(bytes_written)
        } else {
            bytes_written
        };

        if use_resume
            && response_status.as_u16() == 206
            && content_length.is_some_and(|expected| expected != final_size)
        {
            return Err(DownloadError::integrity(
                file_path.clone(),
                content_length.unwrap_or(0),
                final_size,
            ));
        }

        info!(
            path = %file_path.display(),
            bytes = final_size,
            resumed = use_resume && response_status.as_u16() == 206,
            "download complete"
        );

        Ok(DownloadFileResult {
            path: file_path,
            bytes_downloaded: final_size,
            content_length,
            resumed: use_resume && response_status.as_u16() == 206,
            resume_attempted,
        })
    }

    async fn send_request(
        &self,
        method: &str,
        url: &str,
        user_agent: Option<&str>,
        range_header: Option<&str>,
    ) -> Result<reqwest::Response, DownloadError> {
        let mut request = match method {
            "HEAD" => self.client.head(url),
            _ => self.client.get(url),
        };
        if let Some(ua) = user_agent {
            request = request.header(reqwest::header::USER_AGENT, ua);
        }
        if let Some(range) = range_header {
            request = request.header(RANGE, range);
        }

        let response = request.send().await.map_err(|e| {
            if e.is_timeout() {
                DownloadError::timeout(url)
            } else {
                DownloadError::network(url, e)
            }
        })?;

        if !response.status().is_success() {
            let status = response.status();
            let status_code = status.as_u16();

            // Promote auth-related status codes to AuthRequired
            if matches!(status_code, 401 | 403 | 407) {
                let domain = Url::parse(url)
                    .ok()
                    .and_then(|u| u.host_str().map(std::string::ToString::to_string))
                    .unwrap_or_else(|| url.to_string());
                return Err(DownloadError::auth_required(url, status_code, domain));
            }

            let retry_after = response
                .headers()
                .get(RETRY_AFTER)
                .and_then(|v| v.to_str().ok())
                .map(std::string::ToString::to_string);
            return Err(DownloadError::http_status_with_retry_after(
                url,
                status_code,
                retry_after,
            ));
        }

        // Detect login redirect: server returned 200 with HTML when a binary file
        // was expected. Only flag when the URL path ends in a known binary extension.
        if method == "GET" {
            if let Some(auth_err) = detect_login_redirect(url, &response) {
                return Err(auth_err);
            }
        }

        Ok(response)
    }

    async fn determine_resume_state(
        &self,
        candidate_path: &Path,
        url: &str,
        user_agent: Option<&str>,
        has_preferred_filename: bool,
        allow_resume: bool,
        expected_partial_bytes: u64,
    ) -> (u64, bool, bool) {
        let existing_bytes = if allow_resume {
            tokio::fs::metadata(candidate_path)
                .await
                .map(|meta| meta.len())
                .unwrap_or(0)
        } else {
            0
        };

        let should_probe_resume = if has_preferred_filename {
            expected_partial_bytes > 0 && existing_bytes == expected_partial_bytes
        } else {
            existing_bytes > 0
        };

        if allow_resume && should_probe_resume {
            let head_response = self.send_request("HEAD", url, user_agent, None).await.ok();
            let supports_ranges = head_response
                .as_ref()
                .and_then(|r| r.headers().get(ACCEPT_RANGES))
                .and_then(|v| v.to_str().ok())
                .is_some_and(|v| v.eq_ignore_ascii_case("bytes"));
            (existing_bytes, supports_ranges, true)
        } else {
            (existing_bytes, false, false)
        }
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
    let mut writer = BufWriter::new(file);
    let mut stream = response.bytes_stream();
    let mut bytes_written: u64 = 0;

    while let Some(chunk_result) = stream.next().await {
        let chunk = chunk_result.map_err(|e| DownloadError::network(url, e))?;

        writer
            .write_all(&chunk)
            .await
            .map_err(|e| DownloadError::io(file_path.to_path_buf(), e))?;

        bytes_written += chunk.len() as u64;
    }

    // Ensure all data is flushed to disk
    writer
        .flush()
        .await
        .map_err(|e| DownloadError::io(file_path.to_path_buf(), e))?;

    Ok(bytes_written)
}

fn build_client(
    cookie_jar: Option<Arc<Jar>>,
    connect_timeout_secs: u64,
    read_timeout_secs: u64,
) -> Result<Client, reqwest::Error> {
    let initial = try_build_client(
        cookie_jar.clone(),
        connect_timeout_secs,
        read_timeout_secs,
        false,
    );
    match initial {
        Ok(client) => Ok(client),
        Err(BuildClientFailure::Panic) => {
            warn!(
                "HTTP client builder panicked while loading system proxy settings; retrying with env-proxy fallback"
            );
            match try_build_client(cookie_jar, connect_timeout_secs, read_timeout_secs, true) {
                Ok(client) => Ok(client),
                Err(BuildClientFailure::Build(error)) => Err(error),
                Err(BuildClientFailure::Panic) => {
                    panic!("HTTP client builder panicked while applying env-proxy fallback")
                }
            }
        }
        Err(BuildClientFailure::Build(error)) => Err(error),
    }
}

enum BuildClientFailure {
    Panic,
    Build(reqwest::Error),
}

// `catch_unwind` does not suppress panic-hook stderr output. Guarded client
// builds intentionally catch system-proxy panics, so suppress hook output
// briefly to keep CLI stderr deterministic for expected recovery paths.
static CLIENT_BUILD_PANIC_HOOK_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

fn try_build_client(
    cookie_jar: Option<Arc<Jar>>,
    connect_timeout_secs: u64,
    read_timeout_secs: u64,
    disable_system_proxy_lookup: bool,
) -> Result<Client, BuildClientFailure> {
    catch_unwind_silent(AssertUnwindSafe(move || {
        #[cfg(test)]
        maybe_inject_client_build_panic(disable_system_proxy_lookup);

        let mut builder = base_client_builder(cookie_jar, connect_timeout_secs, read_timeout_secs);
        if disable_system_proxy_lookup {
            builder = apply_env_proxy_fallback(builder.no_proxy());
        }
        builder.build().map_err(BuildClientFailure::Build)
    }))
    .map_err(|_| BuildClientFailure::Panic)?
}

fn catch_unwind_silent<F, T>(operation: F) -> Result<T, Box<dyn std::any::Any + Send + 'static>>
where
    F: FnOnce() -> T + std::panic::UnwindSafe,
{
    let _panic_hook_guard = CLIENT_BUILD_PANIC_HOOK_LOCK
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let previous_hook = take_hook();
    set_hook(Box::new(|_| {}));
    let outcome = catch_unwind(operation);
    set_hook(previous_hook);
    outcome
}

fn base_client_builder(
    cookie_jar: Option<Arc<Jar>>,
    connect_timeout_secs: u64,
    read_timeout_secs: u64,
) -> ClientBuilder {
    let mut builder = Client::builder()
        .connect_timeout(Duration::from_secs(connect_timeout_secs))
        .timeout(Duration::from_secs(read_timeout_secs))
        .gzip(true)
        .user_agent(user_agent::default_download_user_agent());
    if let Some(jar) = cookie_jar {
        builder = builder.cookie_provider(jar);
    }
    builder
}

fn apply_env_proxy_fallback(mut builder: ClientBuilder) -> ClientBuilder {
    if let Some(proxy) = env_proxy_for_scheme("https")
        && let Ok(resolved) = Proxy::https(&proxy)
    {
        builder = builder.proxy(resolved);
    }
    if let Some(proxy) = env_proxy_for_scheme("http")
        && let Ok(resolved) = Proxy::http(&proxy)
    {
        builder = builder.proxy(resolved);
    }
    builder
}

fn env_proxy_for_scheme(scheme: &str) -> Option<String> {
    match scheme {
        "https" => find_first_proxy_var(&["HTTPS_PROXY", "https_proxy", "ALL_PROXY", "all_proxy"]),
        "http" => find_first_proxy_var(&["HTTP_PROXY", "http_proxy", "ALL_PROXY", "all_proxy"]),
        _ => None,
    }
}

fn find_first_proxy_var(names: &[&str]) -> Option<String> {
    names.iter().find_map(|name| {
        std::env::var(name)
            .ok()
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty())
    })
}

#[cfg(test)]
static CLIENT_BUILD_PANIC_INJECTION_COUNT: std::sync::atomic::AtomicUsize =
    std::sync::atomic::AtomicUsize::new(0);

#[cfg(test)]
fn inject_client_build_panics(count: usize) {
    CLIENT_BUILD_PANIC_INJECTION_COUNT.store(count, std::sync::atomic::Ordering::SeqCst);
}

#[cfg(test)]
fn maybe_inject_client_build_panic(disable_system_proxy_lookup: bool) {
    use std::sync::atomic::Ordering;

    if disable_system_proxy_lookup {
        return;
    }

    if CLIENT_BUILD_PANIC_INJECTION_COUNT
        .fetch_update(Ordering::SeqCst, Ordering::SeqCst, |count| {
            if count > 0 { Some(count - 1) } else { None }
        })
        .is_ok()
    {
        panic!("injected HTTP client builder panic");
    }
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

    // Ultimate fallback: timestamp-based name with content-type detection
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);

    // Try to get extension from Content-Type header
    let extension = response
        .headers()
        .get("content-type")
        .and_then(|ct| ct.to_str().ok())
        .map_or(".bin", extension_from_content_type);

    format!("download_{timestamp}{extension}")
}

fn derive_total_content_length(response: &reqwest::Response, existing_bytes: u64) -> Option<u64> {
    let current = response
        .headers()
        .get(CONTENT_LENGTH)
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.parse::<u64>().ok());
    if response.status().as_u16() == 206 {
        current.map(|remaining| existing_bytes.saturating_add(remaining))
    } else {
        current
    }
}

/// Known binary file extensions that indicate the server should NOT return HTML.
const BINARY_EXTENSIONS: &[&str] = &[
    ".pdf", ".doc", ".docx", ".epub", ".zip", ".tar.gz", ".gz", ".xls", ".xlsx", ".ppt", ".pptx",
    ".odt", ".ods", ".odp", ".rtf", ".ps", ".djvu", ".mobi",
];

/// Common URL patterns indicating a login/SSO redirect.
const LOGIN_PATTERNS: &[&str] = &[
    "/login",
    "/signin",
    "/sign-in",
    "/auth/",
    "/sso",
    "/cas/login",
    "/saml",
    "/oauth",
    "/openid",
    "/idp/",
];

/// Returns true if the URL path ends in a known binary extension.
fn is_expected_binary(url: &str) -> bool {
    let path = Url::parse(url)
        .ok()
        .map(|u| u.path().to_lowercase())
        .unwrap_or_default();
    BINARY_EXTENSIONS.iter().any(|ext| path.ends_with(ext))
}

/// Detects login redirect: HTML response returned when a binary file was expected.
/// Returns `Some(DownloadError::AuthRequired)` if a login redirect is detected.
fn detect_login_redirect(
    original_url: &str,
    response: &reqwest::Response,
) -> Option<DownloadError> {
    if !is_expected_binary(original_url) {
        return None;
    }

    let content_type = response
        .headers()
        .get(reqwest::header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");

    if !content_type.to_ascii_lowercase().contains("text/html") {
        return None;
    }

    // Check if the final response URL has login/SSO patterns.
    // Only classify as auth-required when a login pattern IS present.
    // Without a pattern match, it may be a server error page — not auth.
    let response_url = response.url().as_str();
    let has_login_pattern = LOGIN_PATTERNS
        .iter()
        .any(|pattern| response_url.to_lowercase().contains(pattern));

    if !has_login_pattern {
        debug!(
            url = %original_url,
            response_url = %response_url,
            "HTML response for expected binary download but no login pattern — not flagging as auth"
        );
        return None;
    }

    let domain = response
        .url()
        .host_str()
        .map(std::string::ToString::to_string)
        .or_else(|| {
            Url::parse(original_url)
                .ok()
                .and_then(|u| u.host_str().map(std::string::ToString::to_string))
        })
        .unwrap_or_else(|| "unknown".to_string());

    debug!(
        url = %original_url,
        response_url = %response_url,
        domain = %domain,
        "login redirect detected"
    );

    Some(DownloadError::auth_required(original_url, 0, domain))
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    use crate::test_support::socket_guard::start_mock_server_or_skip;
    use tempfile::TempDir;
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, ResponseTemplate};

    static CLIENT_BUILD_TEST_LOCK: Mutex<()> = Mutex::new(());

    struct EnvVarRestore {
        name: &'static str,
        previous: Option<String>,
    }

    impl EnvVarRestore {
        fn set(name: &'static str, value: Option<&str>) -> Self {
            let previous = std::env::var(name).ok();
            // SAFETY: test uses process-local lock to avoid concurrent env mutation.
            unsafe {
                match value {
                    Some(value) => std::env::set_var(name, value),
                    None => std::env::remove_var(name),
                }
            }
            Self { name, previous }
        }
    }

    impl Drop for EnvVarRestore {
        fn drop(&mut self) {
            // SAFETY: paired restoration under process-local test lock.
            unsafe {
                match &self.previous {
                    Some(previous) => std::env::set_var(self.name, previous),
                    None => std::env::remove_var(self.name),
                }
            }
        }
    }

    #[test]
    fn test_http_client_new_recovers_from_primary_builder_panic() {
        let _lock = CLIENT_BUILD_TEST_LOCK.lock().unwrap();
        inject_client_build_panics(1);

        let client = HttpClient::new();
        drop(client);
    }

    #[test]
    fn test_http_client_fallback_path_still_returns_invalid_url_error() {
        let _lock = CLIENT_BUILD_TEST_LOCK.lock().unwrap();
        inject_client_build_panics(1);
        let client = HttpClient::new();

        let temp_dir = TempDir::new().unwrap();
        let result =
            tokio_test::block_on(client.download_to_file("not-a-valid-url", temp_dir.path()));
        assert!(matches!(result, Err(DownloadError::InvalidUrl { .. })));
    }

    #[test]
    fn test_env_proxy_for_scheme_prefers_specific_proxy_var() {
        let _lock = CLIENT_BUILD_TEST_LOCK.lock().unwrap();
        let _restore_https = EnvVarRestore::set("HTTPS_PROXY", Some("http://proxy.example:8443"));
        let _restore_all = EnvVarRestore::set("ALL_PROXY", Some("http://all.example:8080"));

        assert_eq!(
            env_proxy_for_scheme("https"),
            Some("http://proxy.example:8443".to_string())
        );
    }

    #[tokio::test]
    async fn test_http_client_download_success() {
        let Some(mock_server) = start_mock_server_or_skip().await else {
            return;
        };
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
        let Some(mock_server) = start_mock_server_or_skip().await else {
            return;
        };
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
        let Some(mock_server) = start_mock_server_or_skip().await else {
            return;
        };
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
        let Some(mock_server) = start_mock_server_or_skip().await else {
            return;
        };
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
        let Some(mock_server) = start_mock_server_or_skip().await else {
            return;
        };
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
        let Some(mock_server) = start_mock_server_or_skip().await else {
            return;
        };
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
        let Some(mock_server) = start_mock_server_or_skip().await else {
            return;
        };
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

    #[tokio::test]
    async fn test_download_cleanup_on_read_timeout() {
        // Regression: partial file must be removed when stream fails (e.g. read timeout)
        let Some(mock_server) = start_mock_server_or_skip().await else {
            return;
        };
        let temp_dir = TempDir::new().unwrap();

        Mock::given(method("GET"))
            .and(path("/slow"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_bytes(b"data")
                    .set_delay(Duration::from_secs(3)),
            )
            .mount(&mock_server)
            .await;

        let client = HttpClient::new_with_timeouts(30, 1);
        let url = format!("{}/slow", mock_server.uri());

        let result = client.download_to_file(&url, temp_dir.path()).await;
        assert!(result.is_err(), "expected timeout or network error");

        let entries: Vec<_> = std::fs::read_dir(temp_dir.path()).unwrap().collect();
        assert!(
            entries.is_empty(),
            "Partial file must be cleaned up after stream error, found: {:?}",
            entries
        );
    }

    #[tokio::test]
    async fn test_download_with_user_agent_sends_custom_header() {
        use wiremock::{Match, Request};

        /// Matches requests whose User-Agent contains "Chrome".
        struct BrowserUaMatcher;

        impl Match for BrowserUaMatcher {
            fn matches(&self, request: &Request) -> bool {
                request
                    .headers
                    .get("User-Agent")
                    .and_then(|v| v.to_str().ok())
                    .is_some_and(|ua| ua.contains("Chrome"))
            }
        }

        let Some(mock_server) = start_mock_server_or_skip().await else {
            return;
        };
        let temp_dir = TempDir::new().unwrap();

        // Return 200 for requests WITH the browser User-Agent (higher priority)
        Mock::given(method("GET"))
            .and(path("/protected"))
            .and(BrowserUaMatcher)
            .respond_with(ResponseTemplate::new(200).set_body_bytes(b"secret content"))
            .with_priority(1)
            .mount(&mock_server)
            .await;

        // Return 403 for all other requests (lower priority = fallback)
        Mock::given(method("GET"))
            .and(path("/protected"))
            .respond_with(ResponseTemplate::new(403))
            .with_priority(u8::MAX)
            .mount(&mock_server)
            .await;

        let client = HttpClient::new();
        let url = format!("{}/protected", mock_server.uri());

        // Without browser UA → 403 (now AuthRequired)
        let result = client.download_to_file(&url, temp_dir.path()).await;
        assert!(result.is_err());
        match &result {
            Err(DownloadError::AuthRequired { status: 403, .. }) => {}
            other => panic!("Expected AuthRequired 403, got: {other:?}"),
        }

        // With browser UA → 200
        let result = client
            .download_to_file_with_user_agent(&url, temp_dir.path(), BROWSER_USER_AGENT)
            .await;
        assert!(result.is_ok(), "Expected Ok, got: {result:?}");
        let download = result.unwrap();
        let contents = std::fs::read(&download.path).unwrap();
        assert_eq!(contents, b"secret content");
    }

    #[tokio::test]
    async fn test_default_download_sends_user_agent() {
        use wiremock::{Match, Request};

        /// Matches the first request only: User-Agent must be the default identity UA
        /// (downloader + version, no Chrome). This test issues a single GET with no 403/retry.
        struct DefaultUaMatcher;

        impl Match for DefaultUaMatcher {
            fn matches(&self, request: &Request) -> bool {
                request
                    .headers
                    .get("User-Agent")
                    .and_then(|v| v.to_str().ok())
                    .is_some_and(|ua| {
                        ua.contains("downloader")
                            && ua.contains(env!("CARGO_PKG_VERSION"))
                            && !ua.contains("Chrome")
                    })
            }
        }

        let Some(mock_server) = start_mock_server_or_skip().await else {
            return;
        };
        let temp_dir = TempDir::new().unwrap();

        Mock::given(method("GET"))
            .and(path("/default-ua"))
            .and(DefaultUaMatcher)
            .respond_with(ResponseTemplate::new(200).set_body_bytes(b"ok"))
            .mount(&mock_server)
            .await;

        let client = HttpClient::new();
        let url = format!("{}/default-ua", mock_server.uri());
        let result = client.download_to_file(&url, temp_dir.path()).await;
        assert!(
            result.is_ok(),
            "Default client must send User-Agent; got: {result:?}"
        );
    }

    #[tokio::test]
    async fn test_fresh_download_does_not_send_head_request() {
        let Some(mock_server) = start_mock_server_or_skip().await else {
            return;
        };
        let temp_dir = TempDir::new().unwrap();

        // HEAD should never be called for a fresh download (no partial file)
        Mock::given(method("HEAD"))
            .and(path("/fresh.pdf"))
            .respond_with(ResponseTemplate::new(200))
            .expect(0)
            .mount(&mock_server)
            .await;

        Mock::given(method("GET"))
            .and(path("/fresh.pdf"))
            .respond_with(ResponseTemplate::new(200).set_body_bytes(b"fresh content"))
            .expect(1)
            .mount(&mock_server)
            .await;

        let client = HttpClient::new();
        let url = format!("{}/fresh.pdf", mock_server.uri());

        let result = client
            .download_to_file_with_metadata(&url, temp_dir.path())
            .await;
        assert!(result.is_ok(), "Expected Ok, got: {result:?}");

        let download = result.unwrap();
        assert!(!download.resumed, "Fresh download should not be resumed");
        assert!(
            !download.resume_attempted,
            "Fresh download should not attempt resume"
        );
        assert_eq!(std::fs::read(&download.path).unwrap(), b"fresh content");
    }

    #[test]
    fn test_is_expected_binary_pdf() {
        assert!(is_expected_binary("https://example.com/paper.pdf"));
    }

    #[test]
    fn test_is_expected_binary_docx() {
        assert!(is_expected_binary("https://example.com/doc.docx"));
    }

    #[test]
    fn test_is_expected_binary_html_page() {
        assert!(!is_expected_binary("https://example.com/page.html"));
    }

    #[test]
    fn test_is_expected_binary_no_extension() {
        assert!(!is_expected_binary("https://example.com/article/12345"));
    }

    #[test]
    fn test_is_expected_binary_query_param_download() {
        // Extensionless download URLs with query params are NOT treated as binary.
        // This is a known limitation — login redirect detection won't trigger.
        assert!(!is_expected_binary("https://example.com/download?id=123"));
        assert!(!is_expected_binary(
            "https://example.com/gateway/api?file=paper"
        ));
    }

    #[test]
    fn test_is_expected_binary_case_insensitive() {
        assert!(is_expected_binary("https://example.com/Paper.PDF"));
        assert!(is_expected_binary("https://example.com/DOC.Docx"));
    }

    #[tokio::test]
    async fn test_login_redirect_detected_for_pdf() {
        let Some(mock_server) = start_mock_server_or_skip().await else {
            return;
        };
        let temp_dir = TempDir::new().unwrap();

        // Simulate login redirect: PDF URL returns 302 to /login page
        Mock::given(method("GET"))
            .and(path("/paper.pdf"))
            .respond_with(ResponseTemplate::new(302).insert_header(
                "Location",
                format!("{}/login?return=/paper.pdf", mock_server.uri()),
            ))
            .mount(&mock_server)
            .await;

        // Login page returns HTML (reqwest follows the redirect automatically)
        Mock::given(method("GET"))
            .and(path("/login"))
            .respond_with(
                ResponseTemplate::new(200)
                    .insert_header("Content-Type", "text/html; charset=utf-8")
                    .set_body_bytes("<html><body>Please log in</body></html>".as_bytes()),
            )
            .mount(&mock_server)
            .await;

        let client = HttpClient::new();
        let url = format!("{}/paper.pdf", mock_server.uri());

        let result = client.download_to_file(&url, temp_dir.path()).await;
        assert!(result.is_err(), "Expected error for login redirect");
        match result {
            Err(DownloadError::AuthRequired { status: 0, .. }) => {}
            other => panic!("Expected AuthRequired with status 0, got: {other:?}"),
        }
    }

    #[tokio::test]
    async fn test_html_error_page_for_pdf_not_classified_as_auth() {
        let Some(mock_server) = start_mock_server_or_skip().await else {
            return;
        };
        let temp_dir = TempDir::new().unwrap();

        // Server returns HTML error page for PDF URL but NO login pattern in URL
        Mock::given(method("GET"))
            .and(path("/paper.pdf"))
            .respond_with(
                ResponseTemplate::new(200)
                    .insert_header("Content-Type", "text/html; charset=utf-8")
                    .set_body_bytes("<html><body>Server maintenance</body></html>".as_bytes()),
            )
            .mount(&mock_server)
            .await;

        let client = HttpClient::new();
        let url = format!("{}/paper.pdf", mock_server.uri());

        // Should succeed (download the HTML as-is) since no login pattern detected
        let result = client.download_to_file(&url, temp_dir.path()).await;
        assert!(
            result.is_ok(),
            "HTML error page without login pattern should not be classified as auth: {result:?}"
        );
    }

    #[tokio::test]
    async fn test_no_login_redirect_for_html_url() {
        let Some(mock_server) = start_mock_server_or_skip().await else {
            return;
        };
        let temp_dir = TempDir::new().unwrap();

        // Server returns HTML for an HTML URL — not a redirect
        Mock::given(method("GET"))
            .and(path("/article"))
            .respond_with(
                ResponseTemplate::new(200)
                    .insert_header("Content-Type", "text/html; charset=utf-8")
                    .set_body_bytes("<html><body>Article content</body></html>".as_bytes()),
            )
            .mount(&mock_server)
            .await;

        let client = HttpClient::new();
        let url = format!("{}/article", mock_server.uri());

        let result = client.download_to_file(&url, temp_dir.path()).await;
        // Should succeed — HTML URL returning HTML is expected
        assert!(result.is_ok(), "Expected Ok for HTML URL, got: {result:?}");
    }

    #[tokio::test]
    async fn test_login_redirect_detected_with_mixed_case_content_type() {
        let Some(mock_server) = start_mock_server_or_skip().await else {
            return;
        };
        let temp_dir = TempDir::new().unwrap();

        // PDF URL redirects to login page that returns mixed-case Content-Type
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
                    .insert_header("Content-Type", "Text/HTML; charset=utf-8")
                    .set_body_bytes("<html><body>Please log in</body></html>".as_bytes()),
            )
            .mount(&mock_server)
            .await;

        let client = HttpClient::new();
        let url = format!("{}/paper.pdf", mock_server.uri());

        let result = client.download_to_file(&url, temp_dir.path()).await;
        assert!(
            result.is_err(),
            "Mixed-case Content-Type should still be detected"
        );
        match result {
            Err(DownloadError::AuthRequired { status: 0, .. }) => {}
            other => panic!("Expected AuthRequired with status 0, got: {other:?}"),
        }
    }

    #[tokio::test]
    async fn test_extensionless_download_url_not_flagged_as_login_redirect() {
        let Some(mock_server) = start_mock_server_or_skip().await else {
            return;
        };
        let temp_dir = TempDir::new().unwrap();

        // Extensionless download URL redirects to login page — current behavior
        // is to NOT flag this as auth_required (known limitation: login redirect
        // detection only triggers for URLs with known binary extensions).
        Mock::given(method("GET"))
            .and(path("/download"))
            .respond_with(ResponseTemplate::new(302).insert_header(
                "Location",
                format!("{}/login?return=/download", mock_server.uri()),
            ))
            .mount(&mock_server)
            .await;

        Mock::given(method("GET"))
            .and(path("/login"))
            .respond_with(
                ResponseTemplate::new(200)
                    .insert_header("Content-Type", "text/html; charset=utf-8")
                    .set_body_bytes("<html><body>Please log in</body></html>".as_bytes()),
            )
            .mount(&mock_server)
            .await;

        let client = HttpClient::new();
        let url = format!("{}/download?id=123", mock_server.uri());

        // This downloads the HTML as-is — login redirect is NOT detected for
        // extensionless URLs. If this test starts failing, it means the detection
        // was improved to cover extensionless URLs (which is desirable).
        let result = client.download_to_file(&url, temp_dir.path()).await;
        assert!(
            result.is_ok(),
            "Extensionless URL should not trigger login redirect detection: {result:?}"
        );
    }

    #[tokio::test]
    async fn test_401_returns_auth_required() {
        let Some(mock_server) = start_mock_server_or_skip().await else {
            return;
        };
        let temp_dir = TempDir::new().unwrap();

        Mock::given(method("GET"))
            .and(path("/secure.pdf"))
            .respond_with(ResponseTemplate::new(401))
            .mount(&mock_server)
            .await;

        let client = HttpClient::new();
        let url = format!("{}/secure.pdf", mock_server.uri());

        let result = client.download_to_file(&url, temp_dir.path()).await;
        assert!(result.is_err());
        match result {
            Err(DownloadError::AuthRequired { status: 401, .. }) => {}
            other => panic!("Expected AuthRequired 401, got: {other:?}"),
        }
    }
}
