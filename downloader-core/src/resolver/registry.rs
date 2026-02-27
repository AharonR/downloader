//! Resolver registry with priority-ordered resolution loop.
//!
//! The [`ResolverRegistry`] manages a collection of resolvers and orchestrates
//! the resolution loop, including fallback chains and redirect handling.

use tracing::{debug, info, warn};

use crate::parser::InputType;

use super::{ResolveContext, ResolveError, ResolveStep, ResolvedUrl, Resolver};

/// A priority-ordered collection of resolvers with resolution loop.
///
/// The registry tries resolvers in priority order (Specialized first, then General,
/// then Fallback). Within the same priority level, resolvers are tried in
/// registration order.
pub struct ResolverRegistry {
    resolvers: Vec<Box<dyn Resolver>>,
}

impl ResolverRegistry {
    /// Creates an empty resolver registry.
    #[must_use]
    pub fn new() -> Self {
        Self {
            resolvers: Vec::new(),
        }
    }

    /// Registers a resolver with the registry.
    #[tracing::instrument(skip(self, resolver), fields(resolver_name))]
    pub fn register(&mut self, resolver: Box<dyn Resolver>) {
        tracing::Span::current().record("resolver_name", resolver.name());
        debug!(
            name = resolver.name(),
            priority = ?resolver.priority(),
            "Registering resolver"
        );
        self.resolvers.push(resolver);
    }

    /// Returns the number of registered resolvers.
    #[must_use]
    pub fn resolver_count(&self) -> usize {
        self.resolvers.len()
    }

    /// Returns true if no resolvers are registered.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.resolvers.is_empty()
    }

    /// Returns all resolvers that can handle the given input, sorted by priority.
    ///
    /// Resolvers are returned in priority order: Specialized first, then General,
    /// then Fallback. Within the same priority level, registration order is preserved.
    #[must_use]
    #[tracing::instrument(skip(self), fields(input_type = ?input_type))]
    pub fn find_handlers(&self, input: &str, input_type: InputType) -> Vec<&dyn Resolver> {
        let mut handlers: Vec<&dyn Resolver> = self
            .resolvers
            .iter()
            .filter(|r| r.can_handle(input, input_type))
            .map(AsRef::as_ref)
            .collect();
        handlers.sort_by_key(|r| r.priority());
        handlers
    }

    /// Resolves input to a final downloadable URL through the resolution loop.
    ///
    /// This method orchestrates the full resolution process:
    /// 1. Finds all applicable resolvers via `find_handlers()`
    /// 2. Tries each in priority order
    /// 3. On `ResolveStep::Url` → returns success
    /// 4. On `ResolveStep::Redirect` → follows redirect with new handlers
    /// 5. On `ResolveStep::NeedsAuth` → returns `AuthRequired` error
    /// 6. On `ResolveStep::Failed` → tries next resolver
    /// 7. Returns `AllResolversFailed` if no resolver succeeds
    ///
    /// # Errors
    ///
    /// Returns `ResolveError::NoResolver` if no registered resolver can handle the input.
    /// Returns `ResolveError::TooManyRedirects` if the redirect chain exceeds `ctx.max_redirects`.
    /// Returns `ResolveError::AuthRequired` if a resolver detects authentication is needed.
    /// Returns `ResolveError::AllResolversFailed` if all applicable resolvers fail.
    #[tracing::instrument(skip(self, ctx), fields(input_type = ?input_type))]
    pub async fn resolve_to_url(
        &self,
        input: &str,
        input_type: InputType,
        ctx: &ResolveContext,
    ) -> Result<ResolvedUrl, ResolveError> {
        let mut current_input = input.to_string();
        let mut current_type = input_type;
        let mut redirect_count: usize = 0;

        loop {
            let handlers = self.find_handlers(&current_input, current_type);

            if handlers.is_empty() {
                return Err(ResolveError::no_resolver(&current_input));
            }

            debug!(
                handler_count = handlers.len(),
                input = %current_input,
                "Found handlers for input"
            );

            let mut tried_count: usize = 0;
            let mut got_redirect = false;

            for handler in &handlers {
                tried_count += 1;
                debug!(
                    resolver = handler.name(),
                    input = %current_input,
                    "Trying resolver"
                );

                match handler.resolve(&current_input, ctx).await {
                    Ok(ResolveStep::Url(resolved)) => {
                        info!(
                            resolver = handler.name(),
                            url = %resolved.url,
                            "Resolution successful"
                        );
                        return Ok(resolved);
                    }
                    Ok(ResolveStep::Redirect(new_url)) => {
                        redirect_count += 1;
                        if redirect_count > ctx.max_redirects {
                            return Err(ResolveError::too_many_redirects(input, redirect_count));
                        }
                        debug!(
                            resolver = handler.name(),
                            from = %current_input,
                            to = %new_url,
                            redirect_count,
                            "Following redirect"
                        );
                        current_input = new_url;
                        current_type = InputType::Url;
                        got_redirect = true;
                        break;
                    }
                    Ok(ResolveStep::NeedsAuth(req)) => {
                        return Err(ResolveError::auth_required(&req.domain, &req.message));
                    }
                    Ok(ResolveStep::Failed(err)) => {
                        debug!(
                            resolver = handler.name(),
                            error = %err,
                            "Resolver failed, trying next"
                        );
                    }
                    Err(err) => {
                        warn!(
                            resolver = handler.name(),
                            error = %err,
                            "Resolver returned error"
                        );
                    }
                }
            }

            if got_redirect {
                // Continue the outer loop with new input
                continue;
            }

            // All handlers tried, none succeeded
            return Err(ResolveError::all_failed(input, tried_count));
        }
    }
}

impl std::fmt::Debug for ResolverRegistry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let names: Vec<&str> = self.resolvers.iter().map(|r| r.name()).collect();
        f.debug_struct("ResolverRegistry")
            .field("resolver_count", &self.resolvers.len())
            .field("resolvers", &names)
            .finish()
    }
}

impl Default for ResolverRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use crate::resolver::{
        AuthRequirement, ResolveContext, ResolveError, ResolveStep, ResolvedUrl, Resolver,
        ResolverPriority,
    };
    use async_trait::async_trait;

    // ==================== MockResolver for Testing ====================

    struct MockResolver {
        mock_name: &'static str,
        mock_priority: ResolverPriority,
        handles: Vec<InputType>,
        step: ResolveStep,
    }

    #[async_trait]
    impl Resolver for MockResolver {
        fn name(&self) -> &str {
            self.mock_name
        }

        fn priority(&self) -> ResolverPriority {
            self.mock_priority
        }

        fn can_handle(&self, _input: &str, input_type: InputType) -> bool {
            self.handles.contains(&input_type)
        }

        async fn resolve(
            &self,
            _input: &str,
            _ctx: &ResolveContext,
        ) -> Result<ResolveStep, ResolveError> {
            Ok(self.step.clone())
        }
    }

    fn mock_url_resolver(
        name: &'static str,
        priority: ResolverPriority,
        url: &str,
    ) -> MockResolver {
        MockResolver {
            mock_name: name,
            mock_priority: priority,
            handles: vec![InputType::Url],
            step: ResolveStep::Url(ResolvedUrl::new(url)),
        }
    }

    fn mock_failing_resolver(
        name: &'static str,
        priority: ResolverPriority,
        handles: Vec<InputType>,
    ) -> MockResolver {
        MockResolver {
            mock_name: name,
            mock_priority: priority,
            handles,
            step: ResolveStep::Failed(ResolveError::resolution_failed("test", "mock failure")),
        }
    }

    fn mock_redirect_resolver(
        name: &'static str,
        priority: ResolverPriority,
        redirect_to: &str,
    ) -> MockResolver {
        MockResolver {
            mock_name: name,
            mock_priority: priority,
            handles: vec![InputType::Doi],
            step: ResolveStep::Redirect(redirect_to.to_string()),
        }
    }

    // ==================== Registry Basic Tests ====================

    #[test]
    fn test_registry_new_is_empty() {
        let registry = ResolverRegistry::new();
        assert!(registry.is_empty());
        assert_eq!(registry.resolver_count(), 0);
    }

    #[test]
    fn test_registry_debug_shows_resolvers() {
        let mut registry = ResolverRegistry::new();
        registry.register(Box::new(mock_url_resolver(
            "test-resolver",
            ResolverPriority::Fallback,
            "https://example.com",
        )));
        let debug_str = format!("{registry:?}");
        assert!(
            debug_str.contains("test-resolver"),
            "Debug should show resolver names"
        );
        assert!(
            debug_str.contains("resolver_count: 1"),
            "Debug should show count"
        );
    }

    #[test]
    fn test_registry_register_adds_resolver() {
        let mut registry = ResolverRegistry::new();
        registry.register(Box::new(mock_url_resolver(
            "test",
            ResolverPriority::Fallback,
            "https://example.com",
        )));
        assert!(!registry.is_empty());
        assert_eq!(registry.resolver_count(), 1);
    }

    // ==================== find_handlers Tests ====================

    #[test]
    fn test_registry_find_handlers_returns_matching() {
        let mut registry = ResolverRegistry::new();
        registry.register(Box::new(mock_url_resolver(
            "url-handler",
            ResolverPriority::Fallback,
            "https://example.com",
        )));

        let handlers = registry.find_handlers("https://example.com", InputType::Url);
        assert_eq!(handlers.len(), 1);
        assert_eq!(handlers[0].name(), "url-handler");
    }

    #[test]
    fn test_registry_find_handlers_priority_order() {
        let mut registry = ResolverRegistry::new();

        // Register in reverse priority order
        registry.register(Box::new(mock_url_resolver(
            "fallback",
            ResolverPriority::Fallback,
            "https://fallback.com",
        )));
        registry.register(Box::new(mock_url_resolver(
            "specialized",
            ResolverPriority::Specialized,
            "https://specialized.com",
        )));
        registry.register(Box::new(mock_url_resolver(
            "general",
            ResolverPriority::General,
            "https://general.com",
        )));

        let handlers = registry.find_handlers("https://example.com", InputType::Url);
        assert_eq!(handlers.len(), 3);
        assert_eq!(handlers[0].name(), "specialized");
        assert_eq!(handlers[1].name(), "general");
        assert_eq!(handlers[2].name(), "fallback");
    }

    #[test]
    fn test_registry_find_handlers_empty_for_unknown() {
        let mut registry = ResolverRegistry::new();
        registry.register(Box::new(mock_url_resolver(
            "url-only",
            ResolverPriority::Fallback,
            "https://example.com",
        )));

        let handlers = registry.find_handlers("unknown", InputType::Unknown);
        assert!(handlers.is_empty());
    }

    // ==================== resolve_to_url Tests ====================

    #[tokio::test]
    async fn test_registry_resolve_to_url_direct() {
        let mut registry = ResolverRegistry::new();
        registry.register(Box::new(mock_url_resolver(
            "direct",
            ResolverPriority::Fallback,
            "https://example.com/paper.pdf",
        )));

        let ctx = ResolveContext::default();
        let result = registry
            .resolve_to_url("https://example.com/paper.pdf", InputType::Url, &ctx)
            .await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap().url, "https://example.com/paper.pdf");
    }

    #[tokio::test]
    async fn test_registry_resolve_to_url_no_resolver_error() {
        let registry = ResolverRegistry::new();
        let ctx = ResolveContext::default();
        let result = registry
            .resolve_to_url("unknown-input", InputType::Unknown, &ctx)
            .await;
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("no resolver"));
    }

    #[tokio::test]
    async fn test_registry_resolve_to_url_fallback_chain() {
        let mut registry = ResolverRegistry::new();

        // First resolver fails
        registry.register(Box::new(mock_failing_resolver(
            "failing",
            ResolverPriority::Specialized,
            vec![InputType::Url],
        )));
        // Second resolver succeeds
        registry.register(Box::new(mock_url_resolver(
            "fallback",
            ResolverPriority::Fallback,
            "https://example.com/resolved.pdf",
        )));

        let ctx = ResolveContext::default();
        let result = registry
            .resolve_to_url("https://example.com", InputType::Url, &ctx)
            .await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap().url, "https://example.com/resolved.pdf");
    }

    #[tokio::test]
    async fn test_registry_resolve_to_url_redirect() {
        let mut registry = ResolverRegistry::new();

        // DOI resolver redirects to URL
        registry.register(Box::new(mock_redirect_resolver(
            "doi-resolver",
            ResolverPriority::General,
            "https://example.com/paper.pdf",
        )));
        // URL resolver handles the redirect target
        registry.register(Box::new(mock_url_resolver(
            "url-handler",
            ResolverPriority::Fallback,
            "https://example.com/paper.pdf",
        )));

        let ctx = ResolveContext::default();
        let result = registry
            .resolve_to_url("10.1234/test", InputType::Doi, &ctx)
            .await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap().url, "https://example.com/paper.pdf");
    }

    #[tokio::test]
    async fn test_registry_resolve_to_url_too_many_redirects() {
        let mut registry = ResolverRegistry::new();

        // A resolver that always redirects (to itself via URL type)
        registry.register(Box::new(MockResolver {
            mock_name: "infinite-redirect",
            mock_priority: ResolverPriority::Fallback,
            handles: vec![InputType::Url, InputType::Doi],
            step: ResolveStep::Redirect("https://loop.com".to_string()),
        }));

        let mut ctx = ResolveContext::default();
        ctx.max_redirects = 3;

        let result = registry
            .resolve_to_url("https://loop.com", InputType::Url, &ctx)
            .await;
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("too many redirects"));
    }

    #[tokio::test]
    async fn test_registry_resolve_to_url_needs_auth() {
        let mut registry = ResolverRegistry::new();

        registry.register(Box::new(MockResolver {
            mock_name: "auth-required",
            mock_priority: ResolverPriority::Fallback,
            handles: vec![InputType::Url],
            step: ResolveStep::NeedsAuth(AuthRequirement::new(
                "sciencedirect.com",
                "subscription required",
            )),
        }));

        let ctx = ResolveContext::default();
        let result = registry
            .resolve_to_url("https://sciencedirect.com/paper", InputType::Url, &ctx)
            .await;
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("authentication required"));
        assert!(err.to_string().contains("sciencedirect.com"));
    }

    #[tokio::test]
    async fn test_registry_resolve_to_url_all_fail() {
        let mut registry = ResolverRegistry::new();

        registry.register(Box::new(mock_failing_resolver(
            "fail-1",
            ResolverPriority::Specialized,
            vec![InputType::Url],
        )));
        registry.register(Box::new(mock_failing_resolver(
            "fail-2",
            ResolverPriority::Fallback,
            vec![InputType::Url],
        )));

        let ctx = ResolveContext::default();
        let result = registry
            .resolve_to_url("https://example.com", InputType::Url, &ctx)
            .await;
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(
            err.to_string().contains("all resolvers failed"),
            "Expected 'all resolvers failed' in: {}",
            err
        );
        assert!(
            err.to_string().contains("2 resolver(s)"),
            "Expected '2 resolver(s)' in: {}",
            err
        );
    }
}
