//! HTTP download engine for streaming files to disk.
//!
//! This module provides functionality for downloading files from HTTP/HTTPS URLs
//! with streaming support to handle large files efficiently.
//!
//! # Features
//!
//! - Streaming downloads (memory-efficient for large files)
//! - Automatic filename extraction from Content-Disposition headers
//! - Configurable timeouts (30s connect, 5min read by default)
//! - Structured error types with full context
//! - Duplicate filename handling (adds numeric suffix)
//!
//! # Example
//!
//! ```no_run
//! use downloader_core::download::HttpClient;
//! use std::path::Path;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let client = HttpClient::new();
//! let file_path = client
//!     .download_to_file("https://example.com/paper.pdf", Path::new("./downloads"))
//!     .await?;
//! println!("Downloaded: {}", file_path.display());
//! # Ok(())
//! # }
//! ```

mod client;
mod constants;
mod engine;
mod error;
mod filename;
pub mod rate_limiter;
mod retry;
mod robots;

pub use client::{BROWSER_USER_AGENT, DownloadFileResult, HttpClient};
pub use engine::{
    DEFAULT_CONCURRENCY, DownloadEngine, DownloadStats, EngineError, QueueProcessingOptions,
};
pub use error::DownloadError;
pub use filename::build_preferred_filename;
pub use rate_limiter::{RateLimiter, extract_domain, parse_retry_after};
pub use retry::{DEFAULT_MAX_RETRIES, FailureType, RetryDecision, RetryPolicy, classify_error};
pub use robots::{RobotsCache, RobotsDecision, RobotsError, origin_for_robots};

// Note: Per project-context.md, we do NOT define module-local Result aliases.
// Use `Result<T, DownloadError>` explicitly in function signatures.
