//! arXiv resolver for normalizing article and DOI inputs into canonical PDF URLs.

use std::collections::HashMap;
use std::sync::LazyLock;

use async_trait::async_trait;
use regex::Regex;
use url::Url;

use crate::parser::InputType;

use super::utils::{canonical_host, compile_static_regex};
use super::{ResolveContext, ResolveError, ResolveStep, ResolvedUrl, Resolver, ResolverPriority};

const ARXIV_BASE_URL: &str = "https://arxiv.org";
const ARXIV_HOST: &str = "arxiv.org";
const DOI_HOST: &str = "doi.org";
const ARXIV_DOI_PREFIX: &str = "10.48550/";

static ARXIV_ID_RE: LazyLock<Regex> = LazyLock::new(|| {
    compile_static_regex(r"(?i)^(?:\d{4}\.\d{4,5}|[a-z\-]+(?:\.[a-z]{2})?/\d{7})(?:v\d+)?$")
});

/// Specialized resolver for arXiv URLs and DOI signals.
#[derive(Debug, Default)]
pub struct ArxivResolver;

impl ArxivResolver {
    /// Creates a new `ArxivResolver`.
    #[must_use]
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl Resolver for ArxivResolver {
    fn name(&self) -> &'static str {
        "arxiv"
    }

    fn priority(&self) -> ResolverPriority {
        ResolverPriority::Specialized
    }

    fn can_handle(&self, input: &str, input_type: InputType) -> bool {
        extract_arxiv_id(input, input_type).is_some()
    }

    #[tracing::instrument(skip(self, _ctx), fields(resolver = "arxiv", input = %input))]
    async fn resolve(
        &self,
        input: &str,
        _ctx: &ResolveContext,
    ) -> Result<ResolveStep, ResolveError> {
        let Some(arxiv_id) = extract_arxiv_id(input, InputType::Url)
            .or_else(|| extract_arxiv_id(input, InputType::Doi))
        else {
            return Ok(ResolveStep::Failed(ResolveError::resolution_failed(
                input,
                "Input is not a recognized arXiv URL or DOI pattern",
            )));
        };

        let canonical_pdf = format!("{ARXIV_BASE_URL}/pdf/{arxiv_id}.pdf");
        let mut metadata = HashMap::new();
        metadata.insert("doi".to_string(), format!("10.48550/arXiv.{arxiv_id}"));
        metadata.insert("source_url".to_string(), input.trim().to_string());

        Ok(ResolveStep::Url(ResolvedUrl::with_metadata(
            canonical_pdf,
            metadata,
        )))
    }
}

fn extract_arxiv_id(input: &str, input_type: InputType) -> Option<String> {
    match input_type {
        InputType::Doi => extract_from_doi(input),
        InputType::Url => extract_from_url(input),
        _ => None,
    }
}

fn extract_from_doi(input: &str) -> Option<String> {
    let trimmed = input.trim();
    let lower = trimmed.to_ascii_lowercase();
    if !lower.starts_with(ARXIV_DOI_PREFIX) {
        return None;
    }

    let suffix = &trimmed[ARXIV_DOI_PREFIX.len()..];
    let suffix_lower = suffix.to_ascii_lowercase();
    let id_candidate = if suffix_lower.starts_with("arxiv.") {
        &suffix["arxiv.".len()..]
    } else {
        suffix
    };

    normalize_arxiv_id(id_candidate)
}

fn extract_from_url(input: &str) -> Option<String> {
    let parsed = Url::parse(input).ok()?;
    let host = canonical_host(parsed.host_str()?);
    let path = parsed.path().trim();

    if host == ARXIV_HOST {
        if let Some(id) = path.strip_prefix("/abs/") {
            return normalize_arxiv_id(id);
        }
        if let Some(id) = path.strip_prefix("/pdf/") {
            return normalize_arxiv_id(strip_pdf_suffix(id));
        }
        return None;
    }

    if host == DOI_HOST {
        return extract_from_doi(path.trim_start_matches('/'));
    }

    None
}

fn strip_pdf_suffix(value: &str) -> &str {
    value.strip_suffix(".pdf").unwrap_or(value)
}

fn normalize_arxiv_id(candidate: &str) -> Option<String> {
    let trimmed = candidate.trim().trim_matches('/');
    if ARXIV_ID_RE.is_match(trimmed) {
        Some(trimmed.to_string())
    } else {
        None
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn test_arxiv_name_and_priority() {
        let resolver = ArxivResolver::new();
        assert_eq!(resolver.name(), "arxiv");
        assert_eq!(resolver.priority(), ResolverPriority::Specialized);
    }

    #[test]
    fn test_arxiv_can_handle_abs_pdf_and_doi_patterns() {
        let resolver = ArxivResolver::new();
        assert!(resolver.can_handle("https://arxiv.org/abs/2301.01234v2", InputType::Url));
        assert!(resolver.can_handle("https://arxiv.org/pdf/2301.01234.pdf", InputType::Url));
        assert!(resolver.can_handle("https://doi.org/10.48550/arXiv.2301.01234", InputType::Url));
        assert!(resolver.can_handle("10.48550/arXiv.2301.01234", InputType::Doi));
        assert!(!resolver.can_handle("10.1109/5.771073", InputType::Doi));
    }

    #[tokio::test]
    async fn test_arxiv_resolve_normalizes_to_canonical_pdf() {
        let resolver = ArxivResolver::new();
        let ctx = ResolveContext::default();
        let step = resolver
            .resolve("https://arxiv.org/abs/2301.01234v3", &ctx)
            .await
            .unwrap();

        match step {
            ResolveStep::Url(resolved) => {
                assert_eq!(resolved.url, "https://arxiv.org/pdf/2301.01234v3.pdf");
                assert_eq!(
                    resolved.metadata.get("doi").unwrap(),
                    "10.48550/arXiv.2301.01234v3"
                );
            }
            other => panic!("expected Url, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn test_arxiv_resolve_returns_failed_for_malformed_signal() {
        let resolver = ArxivResolver::new();
        let ctx = ResolveContext::default();
        let step = resolver.resolve("10.48550/not-arxiv", &ctx).await.unwrap();
        assert!(matches!(step, ResolveStep::Failed(_)));
    }
}
