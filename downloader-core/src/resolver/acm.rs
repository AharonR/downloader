//! ACM Digital Library resolver for `dl.acm.org` URLs and `10.1145/*` DOI inputs.
//!
//! `dl.acm.org` is behind Cloudflare bot detection that blocks all programmatic
//! clients with HTTP 403. This resolver intercepts ACM inputs, queries the
//! Semantic Scholar API for open-access alternatives (e.g. arXiv preprints),
//! and returns the best available download URL.
//!
//! # Resolution strategy
//!
//! 1. If Semantic Scholar reports an `openAccessPdf` URL on a non-ACM `https://` domain,
//!    use that directly (typically an arXiv preprint).
//! 2. If Semantic Scholar has an `ArXiv` external ID but no usable OA PDF, construct
//!    the arXiv PDF URL directly.
//! 3. Otherwise, return the ACM PDF URL (`dl.acm.org/doi/pdf/{doi}`), which may
//!    require authentication (run `downloader auth capture` to capture browser cookies).

use async_trait::async_trait;
use reqwest::Client;
use url::Url;

use crate::parser::InputType;

use super::http_client::{build_resolver_http_client, standard_user_agent};
use super::semantic_scholar::{self, S2ResolveConfig, DEFAULT_S2_BASE_URL};
use super::utils::looks_like_doi;
use super::{ResolveContext, ResolveError, ResolveStep, Resolver, ResolverPriority};

// ==================== Constants ====================

const ACM_DOI_PREFIX: &str = "10.1145/";

// ==================== AcmResolver ====================

/// Resolves ACM Digital Library inputs via the Semantic Scholar open-access API.
pub struct AcmResolver {
    client: Client,
    s2_base_url: String,
}

impl AcmResolver {
    /// Creates a new `AcmResolver` using the production Semantic Scholar API.
    ///
    /// # Errors
    ///
    /// Returns [`ResolveError`] if HTTP client construction fails.
    pub fn new() -> Result<Self, ResolveError> {
        Self::build(DEFAULT_S2_BASE_URL.to_string())
    }

    /// Creates an `AcmResolver` with a custom Semantic Scholar API base URL.
    ///
    /// Intended for use in tests with a wiremock server.
    ///
    /// # Errors
    ///
    /// Returns [`ResolveError`] if HTTP client construction fails.
    pub fn with_base_url(s2_base_url: impl Into<String>) -> Result<Self, ResolveError> {
        Self::build(s2_base_url.into())
    }

    fn build(s2_base_url: String) -> Result<Self, ResolveError> {
        let user_agent = standard_user_agent("acm");
        let client = build_resolver_http_client("acm", user_agent, None)?;
        Ok(Self {
            client,
            s2_base_url,
        })
    }
}

impl std::fmt::Debug for AcmResolver {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AcmResolver")
            .field("s2_base_url", &self.s2_base_url)
            .finish_non_exhaustive()
    }
}

// ==================== Resolver trait ====================

#[async_trait]
impl Resolver for AcmResolver {
    fn name(&self) -> &'static str {
        "acm"
    }

    fn priority(&self) -> ResolverPriority {
        ResolverPriority::Specialized
    }

    fn can_handle(&self, input: &str, input_type: InputType) -> bool {
        match input_type {
            InputType::Doi => looks_like_doi(input, ACM_DOI_PREFIX),
            InputType::Url => extract_doi_from_acm_url(input).is_some(),
            _ => false,
        }
    }

    #[tracing::instrument(skip(self, _ctx), fields(resolver = "acm", input = %input))]
    async fn resolve(
        &self,
        input: &str,
        _ctx: &ResolveContext,
    ) -> Result<ResolveStep, ResolveError> {
        let doi = if looks_like_doi(input, ACM_DOI_PREFIX) {
            input.trim().to_string()
        } else {
            match extract_doi_from_acm_url(input) {
                Some(doi) => doi,
                None => {
                    return Ok(ResolveStep::Failed(ResolveError::resolution_failed(
                        input,
                        "Could not extract ACM DOI from input. \
                         Why: input is not a 10.1145/ DOI or dl.acm.org article URL. \
                         Fix: provide the DOI directly (e.g. 10.1145/...).",
                    )));
                }
            }
        };

        let fallback_url = format!("https://dl.acm.org/doi/pdf/{doi}");
        let config = S2ResolveConfig {
            fallback_pdf_url: &fallback_url,
            publisher_domains: &["dl.acm.org"],
            resolver_name: self.name(),
        };
        semantic_scholar::resolve_via_s2(&self.client, &self.s2_base_url, &doi, input, &config)
            .await
    }
}

// ==================== DOI extraction from ACM URLs ====================

/// Extracts the ACM DOI from an ACM Digital Library URL.
///
/// Handles URL patterns:
/// - `dl.acm.org/doi/10.1145/{suffix}`
/// - `dl.acm.org/doi/pdf/10.1145/{suffix}`
/// - `dl.acm.org/doi/abs/10.1145/{suffix}`
/// - `dl.acm.org/doi/fullHtml/10.1145/{suffix}`
/// - Any of the above with query strings (e.g. `?casa_token=...`)
///
/// Returns `None` for non-article ACM URLs (e.g. `dl.acm.org/conference/chi`).
fn extract_doi_from_acm_url(input: &str) -> Option<String> {
    let url = Url::parse(input).ok()?;
    let host = url.host_str()?.to_ascii_lowercase();

    if host != "dl.acm.org" {
        return None;
    }

    let path = url.path();
    let prefix = "10.1145/";
    let doi_start = path.find(prefix)?;
    let suffix_raw = &path[doi_start + prefix.len()..];

    // ACM DOI suffixes are two dot-separated numeric groups (e.g. "3460418.3479327").
    // Constrain to digits and dots only so trailing path segments like "/supplementary"
    // (which contain alphanumeric chars) are not accidentally captured.
    let suffix: String = suffix_raw
        .chars()
        .take_while(|c| c.is_ascii_digit() || *c == '.')
        .collect();
    let suffix = suffix.trim_end_matches('.');

    if suffix.is_empty() {
        return None;
    }

    let doi = format!("{prefix}{suffix}");
    if looks_like_doi(&doi, ACM_DOI_PREFIX) {
        Some(doi)
    } else {
        None
    }
}

// ==================== Tests ====================

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use crate::resolver::semantic_scholar::S2_FIELDS;
    use crate::test_support::socket_guard::start_mock_server_or_skip;
    use wiremock::matchers::{method, path, query_param};
    use wiremock::{Mock, ResponseTemplate};

    // ==================== can_handle ====================

    #[test]
    fn test_name_and_priority() {
        let resolver = AcmResolver::new().unwrap();
        assert_eq!(resolver.name(), "acm");
        assert_eq!(resolver.priority(), ResolverPriority::Specialized);
    }

    #[test]
    fn test_can_handle_acm_doi() {
        let resolver = AcmResolver::new().unwrap();
        assert!(resolver.can_handle("10.1145/3460418.3479327", InputType::Doi));
    }

    #[test]
    fn test_cannot_handle_non_acm_doi() {
        let resolver = AcmResolver::new().unwrap();
        assert!(!resolver.can_handle("10.3390/electronics13132567", InputType::Doi));
        assert!(!resolver.can_handle("10.1109/5.771073", InputType::Doi));
        assert!(!resolver.can_handle("10.1007/s10618-021-00787-x", InputType::Doi));
    }

    #[test]
    fn test_cannot_handle_reference_input() {
        let resolver = AcmResolver::new().unwrap();
        assert!(!resolver.can_handle("10.1145/3460418.3479327", InputType::Reference));
    }

    #[test]
    fn test_can_handle_acm_url() {
        let resolver = AcmResolver::new().unwrap();
        assert!(resolver.can_handle(
            "https://dl.acm.org/doi/10.1145/3460418.3479327",
            InputType::Url
        ));
    }

    #[test]
    fn test_can_handle_acm_url_with_pdf_prefix() {
        let resolver = AcmResolver::new().unwrap();
        assert!(resolver.can_handle(
            "https://dl.acm.org/doi/pdf/10.1145/3460418.3479327",
            InputType::Url
        ));
    }

    #[test]
    fn test_cannot_handle_non_doi_acm_url() {
        let resolver = AcmResolver::new().unwrap();
        assert!(!resolver.can_handle("https://dl.acm.org/conference/chi", InputType::Url));
        assert!(!resolver.can_handle("https://dl.acm.org/", InputType::Url));
    }

    // ==================== DOI extraction ====================

    #[test]
    fn test_extract_doi_from_acm_url_variants() {
        let doi = "10.1145/3460418.3479327";
        assert_eq!(
            extract_doi_from_acm_url(&format!("https://dl.acm.org/doi/{doi}")),
            Some(doi.to_string())
        );
        assert_eq!(
            extract_doi_from_acm_url(&format!("https://dl.acm.org/doi/pdf/{doi}")),
            Some(doi.to_string())
        );
        assert_eq!(
            extract_doi_from_acm_url(&format!("https://dl.acm.org/doi/abs/{doi}")),
            Some(doi.to_string())
        );
        assert_eq!(
            extract_doi_from_acm_url(&format!("https://dl.acm.org/doi/fullHtml/{doi}")),
            Some(doi.to_string())
        );
    }

    #[test]
    fn test_extract_doi_from_url_with_query_params() {
        // Institutional access tokens (casa_token) must be stripped
        let result = extract_doi_from_acm_url(
            "https://dl.acm.org/doi/10.1145/3460418.3479327?casa_token=abc123",
        );
        assert_eq!(result, Some("10.1145/3460418.3479327".to_string()));
    }

    #[test]
    fn test_extract_doi_returns_none_for_conference_url() {
        assert_eq!(
            extract_doi_from_acm_url("https://dl.acm.org/conference/chi"),
            None
        );
    }

    // ==================== Wiremock integration tests ====================

    #[tokio::test]
    async fn test_resolve_doi_with_oa_alternative() {
        let Some(mock_server) = start_mock_server_or_skip().await else {
            return;
        };

        Mock::given(method("GET"))
            .and(path("/graph/v1/paper/DOI:10.1145/3460418.3479327"))
            .and(query_param("fields", S2_FIELDS))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "title": "Smart Multimodal Interaction",
                "authors": [{"name": "Alice Smith"}, {"name": "Bob Jones"}],
                "year": 2021,
                "externalIds": {"ArXiv": "2108.04144"},
                "openAccessPdf": {"url": "https://arxiv.org/pdf/2108.04144"}
            })))
            .mount(&mock_server)
            .await;

        let resolver = AcmResolver::with_base_url(mock_server.uri()).unwrap();
        let ctx = ResolveContext::default();
        let result = resolver
            .resolve("10.1145/3460418.3479327", &ctx)
            .await
            .unwrap();

        match result {
            ResolveStep::Url(resolved) => {
                assert_eq!(resolved.url, "https://arxiv.org/pdf/2108.04144");
            }
            other => panic!("Expected Url, got: {other:?}"),
        }
    }

    #[tokio::test]
    async fn test_resolve_doi_with_arxiv_external_id() {
        let Some(mock_server) = start_mock_server_or_skip().await else {
            return;
        };

        Mock::given(method("GET"))
            .and(path("/graph/v1/paper/DOI:10.1145/3460418.3479327"))
            .and(query_param("fields", S2_FIELDS))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "title": "Smart Multimodal Interaction",
                "authors": [{"name": "Alice Smith"}],
                "year": 2021,
                "externalIds": {"ArXiv": "2108.04144"},
                "openAccessPdf": null
            })))
            .mount(&mock_server)
            .await;

        let resolver = AcmResolver::with_base_url(mock_server.uri()).unwrap();
        let ctx = ResolveContext::default();
        let result = resolver
            .resolve("10.1145/3460418.3479327", &ctx)
            .await
            .unwrap();

        match result {
            ResolveStep::Url(resolved) => {
                assert_eq!(resolved.url, "https://arxiv.org/pdf/2108.04144");
            }
            other => panic!("Expected Url, got: {other:?}"),
        }
    }

    #[tokio::test]
    async fn test_resolve_doi_no_alternative_oa_on_acm() {
        let Some(mock_server) = start_mock_server_or_skip().await else {
            return;
        };

        // OA PDF is on dl.acm.org — should be skipped; no ArXiv either
        Mock::given(method("GET"))
            .and(path("/graph/v1/paper/DOI:10.1145/3460418.3479327"))
            .and(query_param("fields", S2_FIELDS))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "title": "Some ACM Paper",
                "authors": [{"name": "Carol White"}],
                "year": 2021,
                "externalIds": {},
                "openAccessPdf": {"url": "https://dl.acm.org/doi/pdf/10.1145/3460418.3479327"}
            })))
            .mount(&mock_server)
            .await;

        let resolver = AcmResolver::with_base_url(mock_server.uri()).unwrap();
        let ctx = ResolveContext::default();
        let result = resolver
            .resolve("10.1145/3460418.3479327", &ctx)
            .await
            .unwrap();

        match result {
            ResolveStep::Url(resolved) => {
                assert_eq!(
                    resolved.url,
                    "https://dl.acm.org/doi/pdf/10.1145/3460418.3479327"
                );
            }
            other => panic!("Expected Url fallback, got: {other:?}"),
        }
    }

    #[tokio::test]
    async fn test_resolve_doi_not_in_s2() {
        let Some(mock_server) = start_mock_server_or_skip().await else {
            return;
        };

        Mock::given(method("GET"))
            .and(path("/graph/v1/paper/DOI:10.1145/3460418.3479327"))
            .and(query_param("fields", S2_FIELDS))
            .respond_with(ResponseTemplate::new(404))
            .mount(&mock_server)
            .await;

        let resolver = AcmResolver::with_base_url(mock_server.uri()).unwrap();
        let ctx = ResolveContext::default();
        let result = resolver
            .resolve("10.1145/3460418.3479327", &ctx)
            .await
            .unwrap();

        assert!(
            matches!(result, ResolveStep::Failed(_)),
            "Expected Failed for S2 404, got: {result:?}"
        );
    }

    #[tokio::test]
    async fn test_resolve_doi_rate_limited() {
        let Some(mock_server) = start_mock_server_or_skip().await else {
            return;
        };

        Mock::given(method("GET"))
            .and(path("/graph/v1/paper/DOI:10.1145/3460418.3479327"))
            .respond_with(ResponseTemplate::new(429))
            .mount(&mock_server)
            .await;

        let resolver = AcmResolver::with_base_url(mock_server.uri()).unwrap();
        let ctx = ResolveContext::default();
        let result = resolver
            .resolve("10.1145/3460418.3479327", &ctx)
            .await
            .unwrap();

        match result {
            ResolveStep::Failed(err) => {
                let msg = err.to_string();
                assert!(
                    msg.contains("rate limit"),
                    "Error message should mention rate limit, got: {msg}"
                );
            }
            other => panic!("Expected Failed for 429, got: {other:?}"),
        }
    }

    #[tokio::test]
    async fn test_resolve_doi_empty_s2_response() {
        let Some(mock_server) = start_mock_server_or_skip().await else {
            return;
        };

        // All nullable fields are null — should still produce ACM fallback URL
        Mock::given(method("GET"))
            .and(path("/graph/v1/paper/DOI:10.1145/3460418.3479327"))
            .and(query_param("fields", S2_FIELDS))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "title": null,
                "authors": null,
                "year": null,
                "externalIds": null,
                "openAccessPdf": null
            })))
            .mount(&mock_server)
            .await;

        let resolver = AcmResolver::with_base_url(mock_server.uri()).unwrap();
        let ctx = ResolveContext::default();
        let result = resolver
            .resolve("10.1145/3460418.3479327", &ctx)
            .await
            .unwrap();

        match result {
            ResolveStep::Url(resolved) => {
                assert_eq!(
                    resolved.url,
                    "https://dl.acm.org/doi/pdf/10.1145/3460418.3479327"
                );
                // Metadata should still have doi and source_url
                assert_eq!(
                    resolved.metadata.get("doi").map(String::as_str),
                    Some("10.1145/3460418.3479327")
                );
                assert!(resolved.metadata.get("title").is_none());
            }
            other => panic!("Expected Url fallback for null S2 response, got: {other:?}"),
        }
    }

    #[tokio::test]
    async fn test_resolve_doi_rejects_http_oa_url() {
        let Some(mock_server) = start_mock_server_or_skip().await else {
            return;
        };

        // OA PDF uses http:// — must be rejected; ArXiv ID is present as fallback
        Mock::given(method("GET"))
            .and(path("/graph/v1/paper/DOI:10.1145/3460418.3479327"))
            .and(query_param("fields", S2_FIELDS))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "title": "A Paper",
                "authors": [{"name": "Dan Brown"}],
                "year": 2021,
                "externalIds": {"ArXiv": "2108.04144"},
                "openAccessPdf": {"url": "http://arxiv.org/pdf/2108.04144"}
            })))
            .mount(&mock_server)
            .await;

        let resolver = AcmResolver::with_base_url(mock_server.uri()).unwrap();
        let ctx = ResolveContext::default();
        let result = resolver
            .resolve("10.1145/3460418.3479327", &ctx)
            .await
            .unwrap();

        match result {
            ResolveStep::Url(resolved) => {
                // http:// OA URL rejected; ArXiv ID used instead
                assert_eq!(resolved.url, "https://arxiv.org/pdf/2108.04144");
            }
            other => panic!("Expected Url from ArXiv fallback, got: {other:?}"),
        }
    }

    #[tokio::test]
    async fn test_resolve_url_success() {
        let Some(mock_server) = start_mock_server_or_skip().await else {
            return;
        };

        Mock::given(method("GET"))
            .and(path("/graph/v1/paper/DOI:10.1145/3460418.3479327"))
            .and(query_param("fields", S2_FIELDS))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "title": "Smart Multimodal Interaction",
                "authors": [{"name": "Alice Smith"}],
                "year": 2021,
                "externalIds": {"ArXiv": "2108.04144"},
                "openAccessPdf": {"url": "https://arxiv.org/pdf/2108.04144"}
            })))
            .mount(&mock_server)
            .await;

        let resolver = AcmResolver::with_base_url(mock_server.uri()).unwrap();
        let ctx = ResolveContext::default();
        let result = resolver
            .resolve("https://dl.acm.org/doi/10.1145/3460418.3479327", &ctx)
            .await
            .unwrap();

        match result {
            ResolveStep::Url(resolved) => {
                assert_eq!(resolved.url, "https://arxiv.org/pdf/2108.04144");
            }
            other => panic!("Expected Url, got: {other:?}"),
        }
    }

    #[tokio::test]
    async fn test_resolve_doi_metadata() {
        let Some(mock_server) = start_mock_server_or_skip().await else {
            return;
        };

        Mock::given(method("GET"))
            .and(path("/graph/v1/paper/DOI:10.1145/3460418.3479327"))
            .and(query_param("fields", S2_FIELDS))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "title": "Smart Multimodal Interaction",
                "authors": [{"name": "Alice Smith"}, {"name": "Bob Jones"}],
                "year": 2021,
                "externalIds": {},
                "openAccessPdf": null
            })))
            .mount(&mock_server)
            .await;

        let resolver = AcmResolver::with_base_url(mock_server.uri()).unwrap();
        let ctx = ResolveContext::default();
        let result = resolver
            .resolve("10.1145/3460418.3479327", &ctx)
            .await
            .unwrap();

        match result {
            ResolveStep::Url(resolved) => {
                let m = &resolved.metadata;
                assert_eq!(
                    m.get("title").map(String::as_str),
                    Some("Smart Multimodal Interaction")
                );
                assert_eq!(
                    m.get("authors").map(String::as_str),
                    Some("Alice Smith; Bob Jones")
                );
                assert_eq!(m.get("year").map(String::as_str), Some("2021"));
                assert_eq!(
                    m.get("doi").map(String::as_str),
                    Some("10.1145/3460418.3479327")
                );
                assert_eq!(
                    m.get("source_url").map(String::as_str),
                    Some("https://doi.org/10.1145/3460418.3479327")
                );
            }
            other => panic!("Expected Url, got: {other:?}"),
        }
    }

    #[tokio::test]
    async fn test_resolve_doi_no_oa_no_arxiv() {
        let Some(mock_server) = start_mock_server_or_skip().await else {
            return;
        };

        Mock::given(method("GET"))
            .and(path("/graph/v1/paper/DOI:10.1145/3460418.3479327"))
            .and(query_param("fields", S2_FIELDS))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "title": "Closed-Access Paper",
                "authors": [{"name": "Eve"}],
                "year": 2023,
                "externalIds": {},
                "openAccessPdf": null
            })))
            .mount(&mock_server)
            .await;

        let resolver = AcmResolver::with_base_url(mock_server.uri()).unwrap();
        let ctx = ResolveContext::default();
        let result = resolver
            .resolve("10.1145/3460418.3479327", &ctx)
            .await
            .unwrap();

        match result {
            ResolveStep::Url(resolved) => {
                assert_eq!(
                    resolved.url,
                    "https://dl.acm.org/doi/pdf/10.1145/3460418.3479327"
                );
            }
            other => panic!("Expected Url fallback, got: {other:?}"),
        }
    }
}
