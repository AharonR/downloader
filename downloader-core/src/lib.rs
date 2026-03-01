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
//! - [`resolver`] - URL resolution pipeline with extensible resolver system
//! - [`topics`] - Topic auto-detection and keyword extraction from paper metadata
//! - [`sidecar`] - JSON-LD sidecar file generation for downloaded documents
//!
//! - [`auth`] - Cookie/credential management

// Clippy lints - strict for library code
#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![warn(clippy::pedantic)]
#![allow(clippy::module_name_repetitions)]

pub mod auth;
pub mod db;
pub mod download;
pub mod parser;
pub mod project;
pub mod queue;
pub mod resolver;
pub mod sidecar;
#[cfg(test)]
pub mod test_support;
pub mod topics;
pub(crate) mod user_agent;

// Re-export commonly used types
pub use auth::{
    CaptureError, CapturedCookieFormat, CapturedCookies, CookieError, CookieLine,
    RuntimeCookieError, StorageError, clear_persisted_cookies, load_cookies_into_jar,
    load_persisted_cookies, load_runtime_cookie_jar, parse_captured_cookies,
    parse_netscape_cookies, persisted_cookie_path, rotate_key, store_persisted_cookies,
    unique_domain_count,
};
pub use db::{Database, DatabaseOptions};
pub use download::{
    DEFAULT_CONCURRENCY, DEFAULT_MAX_RETRIES, DownloadEngine, DownloadFileResult, DownloadStats,
    EngineError, FailureType, HttpClient, QueueProcessingOptions, RateLimiter, RetryDecision,
    RetryPolicy, RobotsCache, RobotsDecision, RobotsError, build_preferred_filename,
    classify_error, origin_for_robots,
};
pub use parser::{
    Confidence, ConfidenceFactors, InputType, ParseResult, ParseTypeCounts, ParsedItem,
    ReferenceConfidence, ReferenceMetadata, extract_reference_confidence, parse_input,
};
pub use queue::{
    DownloadAttempt, DownloadAttemptQuery, DownloadAttemptStatus, DownloadErrorType,
    DownloadSearchCandidate, DownloadSearchQuery, NewDownloadAttempt, Queue, QueueError, QueueItem,
    QueueMetadata, QueueStatus,
};
pub use resolver::{
    ArxivResolver, CrossrefResolver, DirectResolver, IeeeResolver, PubMedResolver, ResolveContext,
    ResolveError, ResolveStep, ResolvedUrl, Resolver, ResolverPriority, ResolverRegistry,
    STANDARD_METADATA_KEYS, ScienceDirectResolver, SpringerResolver,
    build_default_resolver_registry, configure_resolver_http_timeouts,
};
pub use project::{
    ProjectError, escape_markdown_cell, project_history_key, resolve_project_output_dir,
    sanitize_project_name, truncate_field,
};
pub use sidecar::{SidecarConfig, SidecarError, generate_sidecar};
pub use topics::{
    TopicExtractor, extract_keywords, load_custom_topics, match_custom_topics, normalize_topics,
};
