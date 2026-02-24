//! Direct URL resolver - passthrough for plain URLs.
//!
//! The [`DirectResolver`] is the simplest resolver implementation.
//! It accepts plain URLs (`InputType::Url`) and passes them through
//! unchanged as downloadable URLs. It serves as the fallback resolver
//! with the lowest priority.

use async_trait::async_trait;

use crate::parser::InputType;

use super::{ResolveContext, ResolveError, ResolveStep, ResolvedUrl, Resolver, ResolverPriority};

/// A resolver that passes URLs through unchanged.
///
/// This is the fallback resolver that handles plain HTTP/HTTPS URLs
/// by returning them directly without modification. It serves as:
/// - A reference implementation for future resolver authors
/// - A fallback ensuring plain URLs always work
/// - A test vehicle for the registry and resolution loop
#[derive(Debug)]
pub struct DirectResolver;

impl DirectResolver {
    /// Creates a new `DirectResolver`.
    #[must_use]
    pub fn new() -> Self {
        Self
    }
}

impl Default for DirectResolver {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Resolver for DirectResolver {
    fn name(&self) -> &'static str {
        "direct"
    }

    fn priority(&self) -> ResolverPriority {
        ResolverPriority::Fallback
    }

    fn can_handle(&self, _input: &str, input_type: InputType) -> bool {
        input_type == InputType::Url
    }

    #[tracing::instrument(skip(self, _ctx), fields(resolver = "direct"))]
    async fn resolve(
        &self,
        input: &str,
        _ctx: &ResolveContext,
    ) -> Result<ResolveStep, ResolveError> {
        Ok(ResolveStep::Url(ResolvedUrl::new(input)))
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn test_direct_resolver_name() {
        let resolver = DirectResolver::new();
        assert_eq!(resolver.name(), "direct");
    }

    #[test]
    fn test_direct_resolver_priority() {
        let resolver = DirectResolver::new();
        assert_eq!(resolver.priority(), ResolverPriority::Fallback);
    }

    #[test]
    fn test_direct_resolver_can_handle_url() {
        let resolver = DirectResolver::new();
        assert!(resolver.can_handle("https://example.com", InputType::Url));
    }

    #[test]
    fn test_direct_resolver_cannot_handle_doi() {
        let resolver = DirectResolver::new();
        assert!(!resolver.can_handle("10.1234/test", InputType::Doi));
    }

    #[test]
    fn test_direct_resolver_cannot_handle_reference() {
        let resolver = DirectResolver::new();
        assert!(!resolver.can_handle("Smith 2024", InputType::Reference));
    }

    #[tokio::test]
    async fn test_direct_resolver_resolve_returns_url() {
        let resolver = DirectResolver::new();
        let ctx = ResolveContext::default();
        let result = resolver
            .resolve("https://example.com/paper.pdf", &ctx)
            .await;
        assert!(result.is_ok());
        let step = result.unwrap();
        assert!(matches!(step, ResolveStep::Url(_)));
    }

    #[tokio::test]
    async fn test_direct_resolver_resolve_preserves_url() {
        let resolver = DirectResolver::new();
        let ctx = ResolveContext::default();
        let step = resolver
            .resolve("https://example.com/paper.pdf", &ctx)
            .await
            .unwrap();
        if let ResolveStep::Url(resolved) = step {
            assert_eq!(resolved.url, "https://example.com/paper.pdf");
        } else {
            panic!("Expected ResolveStep::Url");
        }
    }
}
