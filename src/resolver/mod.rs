//! URL resolution pipeline for transforming inputs into downloadable URLs.
//!
//! This module provides an extensible resolver system that transforms various
//! input types (URLs, DOIs, references) into final downloadable URLs through
//! a priority-ordered registry with fallback support.
//!
//! # Architecture
//!
//! - [`Resolver`] - Async trait that individual resolvers implement
//! - [`ResolverRegistry`] - Priority-ordered collection of resolvers with resolution loop
//! - [`ResolveStep`] - Result enum from individual resolve operations
//! - [`ArxivResolver`] - Site-specific resolver for `arXiv` URLs/DOIs
//! - [`PubMedResolver`] - Site-specific resolver for PubMed/PMC URL resolution
//! - [`IeeeResolver`] - Site-specific resolver for IEEE Xplore and `10.1109/*` DOI inputs
//! - [`SpringerResolver`] - Site-specific resolver for Springer article/chapter URL inputs
//! - [`ScienceDirectResolver`] - Site-specific resolver for `ScienceDirect` URLs/DOIs
//! - [`DirectResolver`] - Reference implementation (URL passthrough)
//!
//! # Example
//!
//! ```no_run
//! use downloader_core::resolver::{build_default_resolver_registry, ResolveContext};
//! use downloader_core::parser::InputType;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let registry = build_default_resolver_registry(None, "downloader@example.com");
//!
//! let ctx = ResolveContext::default();
//! let resolved = registry
//!     .resolve_to_url("https://example.com/paper.pdf", InputType::Url, &ctx)
//!     .await?;
//! println!("Resolved URL: {}", resolved.url);
//! # Ok(())
//! # }
//! ```

mod arxiv;
mod crossref;
mod direct;
mod error;
mod http_client;
mod ieee;
mod pubmed;
mod registry;
mod sciencedirect;
mod springer;
mod utils;

pub use arxiv::ArxivResolver;
pub use crossref::CrossrefResolver;
pub use direct::DirectResolver;
pub use error::ResolveError;
pub use http_client::configure_resolver_http_timeouts;
pub use ieee::IeeeResolver;
pub use pubmed::PubMedResolver;
pub use registry::ResolverRegistry;
pub use sciencedirect::ScienceDirectResolver;
pub use springer::SpringerResolver;

use std::collections::HashMap;
use std::sync::Arc;

use async_trait::async_trait;
use reqwest::cookie::Jar;
use tracing::warn;

use crate::parser::InputType;

/// Shared metadata contract keys expected across site resolvers.
pub const STANDARD_METADATA_KEYS: [&str; 5] = ["title", "authors", "doi", "year", "source_url"];

/// Builds the default resolver registry used by CLI execution flows.
///
/// Order is deterministic and preserves site-specific priority before
/// general and fallback handlers.
#[must_use]
pub fn build_default_resolver_registry(
    cookie_jar: Option<Arc<Jar>>,
    crossref_mailto: &str,
) -> ResolverRegistry {
    let mut registry = ResolverRegistry::new();

    registry.register(Box::new(ArxivResolver::new()));

    match PubMedResolver::new(cookie_jar.clone()) {
        Ok(resolver) => registry.register(Box::new(resolver)),
        Err(error) => warn!(
            error = %error,
            "PubMed resolver unavailable; continuing with remaining resolvers"
        ),
    }

    match IeeeResolver::new(cookie_jar.clone()) {
        Ok(resolver) => registry.register(Box::new(resolver)),
        Err(error) => warn!(
            error = %error,
            "IEEE resolver unavailable; continuing with remaining resolvers"
        ),
    }

    match SpringerResolver::new(cookie_jar.clone()) {
        Ok(resolver) => registry.register(Box::new(resolver)),
        Err(error) => warn!(
            error = %error,
            "Springer resolver unavailable; continuing with remaining resolvers"
        ),
    }

    match ScienceDirectResolver::new(cookie_jar) {
        Ok(resolver) => registry.register(Box::new(resolver)),
        Err(error) => warn!(
            error = %error,
            "ScienceDirect resolver unavailable; continuing with generic resolvers"
        ),
    }

    match CrossrefResolver::new(crossref_mailto) {
        Ok(resolver) => registry.register(Box::new(resolver)),
        Err(error) => warn!(
            error = %error,
            "Crossref resolver unavailable; continuing with direct fallback only"
        ),
    }
    registry.register(Box::new(DirectResolver::new()));
    registry
}

/// Priority level for resolver ordering.
///
/// Resolvers are tried in priority order: Specialized first, then General, then Fallback.
/// Within the same priority level, resolvers are tried in registration order.
///
/// Derives `Ord` so that `Specialized < General < Fallback` for sorting
/// (try specialized first).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ResolverPriority {
    /// Most specific: site-specific resolvers (e.g., arXiv, `PubMed`)
    Specialized = 0,
    /// General resolvers (e.g., DOI → Crossref)
    General = 1,
    /// Least specific: direct URL passthrough
    Fallback = 2,
}

/// A successfully resolved URL with optional metadata.
#[derive(Debug, Clone)]
pub struct ResolvedUrl {
    /// The final downloadable URL.
    pub url: String,
    /// Optional metadata discovered during resolution (title, authors, etc.)
    pub metadata: HashMap<String, String>,
}

impl ResolvedUrl {
    /// Creates a new resolved URL with no metadata.
    #[must_use]
    pub fn new(url: impl Into<String>) -> Self {
        Self {
            url: url.into(),
            metadata: HashMap::new(),
        }
    }

    /// Creates a new resolved URL with metadata.
    #[must_use]
    pub fn with_metadata(url: impl Into<String>, metadata: HashMap<String, String>) -> Self {
        Self {
            url: url.into(),
            metadata,
        }
    }
}

/// Authentication requirement returned when a resolver detects auth is needed.
#[derive(Debug, Clone)]
pub struct AuthRequirement {
    /// The domain requiring authentication.
    pub domain: String,
    /// Human-readable message about the auth requirement.
    pub message: String,
}

impl AuthRequirement {
    /// Creates a new authentication requirement.
    #[must_use]
    pub fn new(domain: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            domain: domain.into(),
            message: message.into(),
        }
    }
}

/// Result of a single resolver's attempt to resolve input.
#[derive(Debug, Clone)]
pub enum ResolveStep {
    /// Final downloadable URL found.
    Url(ResolvedUrl),
    /// Intermediate URL that needs further resolution.
    Redirect(String),
    /// Authentication is required to access the resource.
    NeedsAuth(AuthRequirement),
    /// This resolver cannot handle the input.
    Failed(ResolveError),
}

/// Context passed to resolvers during resolution.
#[derive(Debug)]
pub struct ResolveContext {
    /// Maximum number of redirect hops allowed.
    pub max_redirects: usize,
}

impl ResolveContext {
    /// Creates a new context with default settings.
    #[must_use]
    pub fn new() -> Self {
        Self { max_redirects: 10 }
    }
}

impl Default for ResolveContext {
    fn default() -> Self {
        Self::new()
    }
}

/// Trait that all resolvers must implement.
///
/// Resolvers transform input strings (URLs, DOIs, references) into downloadable URLs.
/// Each resolver declares what input types it handles and at what priority level.
///
/// # Object Safety
///
/// This trait uses `async_trait` to support dynamic dispatch via `Box<dyn Resolver>`.
/// Rust 2024 native async traits are not object-safe, so `async_trait` is required
/// for the registry pattern.
#[async_trait]
pub trait Resolver: Send + Sync {
    /// Returns the resolver's name (e.g., "direct", "crossref", "arxiv").
    fn name(&self) -> &str;

    /// Returns the resolver's priority level.
    fn priority(&self) -> ResolverPriority;

    /// Returns true if this resolver can handle the given input.
    fn can_handle(&self, input: &str, input_type: InputType) -> bool;

    /// Attempts to resolve the input into a downloadable URL.
    async fn resolve(&self, input: &str, ctx: &ResolveContext)
    -> Result<ResolveStep, ResolveError>;
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn test_resolver_priority_ordering() {
        assert!(ResolverPriority::Specialized < ResolverPriority::General);
        assert!(ResolverPriority::General < ResolverPriority::Fallback);
        assert!(ResolverPriority::Specialized < ResolverPriority::Fallback);
    }

    #[test]
    fn test_resolved_url_new() {
        let resolved = ResolvedUrl::new("https://example.com/paper.pdf");
        assert_eq!(resolved.url, "https://example.com/paper.pdf");
        assert!(resolved.metadata.is_empty());
    }

    #[test]
    fn test_resolved_url_with_metadata() {
        let mut metadata = HashMap::new();
        metadata.insert("title".to_string(), "Test Paper".to_string());
        let resolved = ResolvedUrl::with_metadata("https://example.com/paper.pdf", metadata);
        assert_eq!(resolved.url, "https://example.com/paper.pdf");
        assert_eq!(resolved.metadata.get("title").unwrap(), "Test Paper");
    }

    #[test]
    fn test_resolve_context_default() {
        let ctx = ResolveContext::default();
        assert_eq!(ctx.max_redirects, 10);
    }

    #[test]
    fn test_auth_requirement_new() {
        let req = AuthRequirement::new("example.com", "login required");
        assert_eq!(req.domain, "example.com");
        assert_eq!(req.message, "login required");
    }
}
