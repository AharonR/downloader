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
mod engine;
mod error;
pub mod rate_limiter;
mod retry;

pub use client::HttpClient;
pub use engine::{DEFAULT_CONCURRENCY, DownloadEngine, DownloadStats, EngineError};
pub use error::DownloadError;
pub use rate_limiter::{RateLimiter, extract_domain, parse_retry_after};
pub use retry::{DEFAULT_MAX_RETRIES, FailureType, RetryDecision, RetryPolicy, classify_error};

// Note: Per project-context.md, we do NOT define module-local Result aliases.
// Use `Result<T, DownloadError>` explicitly in function signatures.
