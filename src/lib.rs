//! Downloader Core Library
//!
//! This library provides the core functionality for the downloader tool,
//! which transforms curated lists of sources (URLs, DOIs, bibliographies)
//! into organized, searchable, LLM-ready knowledge.
//!
//! # Architecture
//!
//! The library is organized into the following modules:
//! - [`db`] - Database connection and schema management
//! - [`download`] - HTTP download engine with streaming support
//! - [`parser`] - Input parsing for URLs, DOIs, references
//! - [`queue`] - Download queue persistence and management
//!
//! Future modules will include:
//! - `resolver` - URL resolution pipeline
//! - `auth` - Cookie/credential management

// Clippy lints - strict for library code
#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![warn(clippy::pedantic)]
#![allow(clippy::module_name_repetitions)]

pub mod db;
pub mod download;
pub mod parser;
pub mod queue;

// Re-export commonly used types
pub use db::Database;
pub use download::{
    DEFAULT_CONCURRENCY, DEFAULT_MAX_RETRIES, DownloadEngine, DownloadStats, EngineError,
    FailureType, HttpClient, RateLimiter, RetryDecision, RetryPolicy, classify_error,
};
pub use parser::{InputType, ParseResult, parse_input};
pub use queue::{Queue, QueueError, QueueItem, QueueStatus};
