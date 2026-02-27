//! Error types for resolver operations.
//!
//! This module defines structured errors for URL resolution,
//! following the What/Why/Fix pattern used across the project.

use thiserror::Error;

/// Errors that can occur during URL resolution.
#[derive(Debug, Clone, Error)]
pub enum ResolveError {
    /// No registered resolver can handle the input
    #[error("no resolver found for input '{input}': {reason}\n  Suggestion: {suggestion}")]
    NoResolver {
        /// The input that no resolver could handle
        input: String,
        /// Why no resolver matched
        reason: String,
        /// How to fix the issue
        suggestion: String,
    },

    /// Too many redirects during resolution
    #[error(
        "too many redirects ({count}) resolving '{input}'\n  Suggestion: Check for circular redirects or simplify the URL"
    )]
    TooManyRedirects {
        /// The original input being resolved
        input: String,
        /// Number of redirects encountered
        count: usize,
    },

    /// Authentication required to access the resource
    #[error("authentication required for '{domain}': {message}\n  Suggestion: {suggestion}")]
    AuthRequired {
        /// The domain requiring authentication
        domain: String,
        /// Human-readable auth requirement message
        message: String,
        /// How to provide authentication
        suggestion: String,
    },

    /// A specific resolver failed to resolve the input
    #[error("resolution failed for '{input}': {reason}\n  Suggestion: {suggestion}")]
    ResolutionFailed {
        /// The input that failed resolution
        input: String,
        /// Why resolution failed
        reason: String,
        /// How to fix the issue
        suggestion: String,
    },

    /// All applicable resolvers failed
    #[error(
        "all resolvers failed for '{input}': tried {tried_count} resolver(s)\n  Suggestion: Check the input format or try a different URL"
    )]
    AllResolversFailed {
        /// The input that all resolvers failed on
        input: String,
        /// Number of resolvers that were tried
        tried_count: usize,
    },
}

impl ResolveError {
    /// Creates a `NoResolver` error for input with no matching resolver.
    #[must_use]
    pub fn no_resolver(input: &str) -> Self {
        Self::NoResolver {
            input: input.to_string(),
            reason: "no registered resolver can handle this input".to_string(),
            suggestion: "Check the input format or register an appropriate resolver".to_string(),
        }
    }

    /// Creates a `TooManyRedirects` error.
    #[must_use]
    pub fn too_many_redirects(input: &str, count: usize) -> Self {
        Self::TooManyRedirects {
            input: input.to_string(),
            count,
        }
    }

    /// Creates an `AuthRequired` error.
    #[must_use]
    pub fn auth_required(domain: &str, message: &str) -> Self {
        Self::AuthRequired {
            domain: domain.to_string(),
            message: message.to_string(),
            suggestion: "Provide authentication credentials for this domain".to_string(),
        }
    }

    /// Creates a `ResolutionFailed` error.
    #[must_use]
    pub fn resolution_failed(input: &str, reason: &str) -> Self {
        Self::ResolutionFailed {
            input: input.to_string(),
            reason: reason.to_string(),
            suggestion: "Check the input and try again".to_string(),
        }
    }

    /// Creates an `AllResolversFailed` error.
    #[must_use]
    pub fn all_failed(input: &str, tried_count: usize) -> Self {
        Self::AllResolversFailed {
            input: input.to_string(),
            tried_count,
        }
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn test_resolve_error_no_resolver_message() {
        let err = ResolveError::no_resolver("10.1234/test");
        let msg = err.to_string();
        assert!(msg.contains("10.1234/test"), "should contain input");
        assert!(msg.contains("no resolver"), "should mention no resolver");
        assert!(msg.contains("Suggestion"), "should have suggestion");
    }

    #[test]
    fn test_resolve_error_too_many_redirects_message() {
        let err = ResolveError::too_many_redirects("https://example.com", 11);
        let msg = err.to_string();
        assert!(msg.contains("11"), "should contain redirect count");
        assert!(msg.contains("example.com"), "should contain input");
        assert!(
            msg.contains("circular"),
            "suggestion should mention circular"
        );
    }

    #[test]
    fn test_resolve_error_all_failed_message() {
        let err = ResolveError::all_failed("https://example.com", 3);
        let msg = err.to_string();
        assert!(msg.contains("3 resolver(s)"), "should contain tried count");
        assert!(msg.contains("example.com"), "should contain input");
    }

    #[test]
    fn test_resolve_error_auth_required_message() {
        let err = ResolveError::auth_required("sciencedirect.com", "subscription required");
        let msg = err.to_string();
        assert!(msg.contains("sciencedirect.com"), "should contain domain");
        assert!(
            msg.contains("subscription required"),
            "should contain message"
        );
        assert!(
            msg.contains("credentials"),
            "suggestion should mention credentials"
        );
    }

    #[test]
    fn test_resolve_error_resolution_failed_message() {
        let err = ResolveError::resolution_failed("10.1234/test", "DOI not found");
        let msg = err.to_string();
        assert!(msg.contains("10.1234/test"), "should contain input");
        assert!(msg.contains("DOI not found"), "should contain reason");
    }

    #[test]
    fn test_resolve_error_clone() {
        let err = ResolveError::no_resolver("test-input");
        let cloned = err.clone();
        assert_eq!(err.to_string(), cloned.to_string());
    }
}
