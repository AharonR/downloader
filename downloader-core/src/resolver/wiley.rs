//! Wiley Online Library resolver for `onlinelibrary.wiley.com` URLs and
//! `10.1002/*` / `10.1111/*` DOI inputs.
//!
//! Wiley's platform returns HTTP 403 for all programmatic PDF access. This
//! resolver intercepts Wiley inputs, queries the Semantic Scholar API for
//! open-access alternatives (e.g. arXiv preprints), and returns the best
//! available download URL.
//!
//! # Resolution strategy
//!
//! 1. If Semantic Scholar reports an `openAccessPdf` URL on a non-Wiley `https://`
//!    domain, use that directly (typically an arXiv preprint).
//! 2. If Semantic Scholar has an `ArXiv` external ID but no usable OA PDF, construct
//!    the arXiv PDF URL directly.
//! 3. Otherwise, return the Wiley PDF URL (`onlinelibrary.wiley.com/doi/pdf/{doi}`),
//!    which may require authentication (run `downloader auth capture`).
//!
//! # Notes
//!
//! - Semantic Scholar coverage varies by journal type. Clinical and medical Wiley
//!   journals (e.g. `cre2`, `adj`) have lower OA coverage than CS-focused venues.
//!   Falling back to the Wiley PDF URL + auth capture is expected behaviour, not
//!   a bug.
//! - `WILEY_DOI_PREFIXES` covers the two main prefixes: `10.1002/` (main Wiley)
//!   and `10.1111/` (Wiley-Blackwell). Additional acquired-publisher prefixes
//!   (e.g. `10.1046/`, `10.1113/`) can be appended here as encountered — the
//!   array-based design supports this without API changes.
//! - Semantic Scholar is shared with the ACM resolver. Both resolvers send
//!   independent requests; S2 rate limits are generous (~100 req/5min unauthenticated)
//!   for typical batch sizes, but may trigger on large mixed-publisher batches.

use async_trait::async_trait;
use reqwest::Client;
use url::Url;

use crate::parser::InputType;

use super::http_client::{build_resolver_http_client, standard_user_agent};
use super::semantic_scholar::{self, DEFAULT_S2_BASE_URL, S2ResolveConfig};
use super::utils::looks_like_doi_any;
use super::{ResolveContext, ResolveError, ResolveStep, Resolver, ResolverPriority};

// ==================== Constants ====================

/// Wiley DOI prefixes.
/// - `10.1002/` — main Wiley prefix
/// - `10.1111/` — Wiley-Blackwell (acquired from Blackwell Publishing)
const WILEY_DOI_PREFIXES: &[&str] = &["10.1002/", "10.1111/"];

/// Wiley publisher domains to reject when evaluating open-access PDF URLs.
/// OA PDFs on these domains (or their subdomains) are still paywalled.
const WILEY_PUBLISHER_DOMAINS: &[&str] = &["onlinelibrary.wiley.com", "wiley.com"];

// ==================== WileyResolver ====================

/// Resolves Wiley Online Library inputs via the Semantic Scholar open-access API.
pub struct WileyResolver {
    client: Client,
    s2_base_url: String,
}

impl WileyResolver {
    /// Creates a new `WileyResolver` using the production Semantic Scholar API.
    ///
    /// # Errors
    ///
    /// Returns [`ResolveError`] if HTTP client construction fails.
    pub fn new() -> Result<Self, ResolveError> {
        Self::build(DEFAULT_S2_BASE_URL.to_string())
    }

    /// Creates a `WileyResolver` with a custom Semantic Scholar API base URL.
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
        let user_agent = standard_user_agent("wiley");
        let client = build_resolver_http_client("wiley", user_agent, None)?;
        Ok(Self {
            client,
            s2_base_url,
        })
    }
}

impl std::fmt::Debug for WileyResolver {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("WileyResolver")
            .field("s2_base_url", &self.s2_base_url)
            .finish_non_exhaustive()
    }
}

// ==================== Resolver trait ====================

#[async_trait]
impl Resolver for WileyResolver {
    fn name(&self) -> &'static str {
        "wiley"
    }

    fn priority(&self) -> ResolverPriority {
        ResolverPriority::Specialized
    }

    fn can_handle(&self, input: &str, input_type: InputType) -> bool {
        match input_type {
            InputType::Doi => looks_like_doi_any(input, WILEY_DOI_PREFIXES),
            InputType::Url => extract_doi_from_wiley_url(input).is_some(),
            _ => false,
        }
    }

    #[tracing::instrument(skip(self, _ctx), fields(resolver = "wiley", input = %input))]
    async fn resolve(
        &self,
        input: &str,
        _ctx: &ResolveContext,
    ) -> Result<ResolveStep, ResolveError> {
        let doi = if looks_like_doi_any(input, WILEY_DOI_PREFIXES) {
            input.trim().to_string()
        } else {
            match extract_doi_from_wiley_url(input) {
                Some(doi) => doi,
                None => {
                    return Ok(ResolveStep::Failed(ResolveError::resolution_failed(
                        input,
                        "Could not extract Wiley DOI from input. \
                         Why: input is not a 10.1002/ or 10.1111/ DOI or \
                         onlinelibrary.wiley.com article URL. \
                         Fix: provide the DOI directly.",
                    )));
                }
            }
        };

        let fallback_url = format!("https://onlinelibrary.wiley.com/doi/pdf/{doi}");
        let config = S2ResolveConfig {
            fallback_pdf_url: &fallback_url,
            publisher_domains: WILEY_PUBLISHER_DOMAINS,
            resolver_name: self.name(),
        };
        semantic_scholar::resolve_via_s2(&self.client, &self.s2_base_url, &doi, input, &config)
            .await
    }
}

// ==================== DOI extraction from Wiley URLs ====================

/// Extracts a Wiley DOI from an `onlinelibrary.wiley.com` URL.
///
/// Handles URL patterns:
/// - `onlinelibrary.wiley.com/doi/10.1002/{suffix}`
/// - `onlinelibrary.wiley.com/doi/pdf/10.1002/{suffix}`
/// - `onlinelibrary.wiley.com/doi/abs/10.1002/{suffix}`
/// - `onlinelibrary.wiley.com/doi/full/10.1002/{suffix}`
/// - `onlinelibrary.wiley.com/doi/epdf/10.1002/{suffix}`
/// - Any of the above with `10.1111/` prefix
/// - Any of the above with query strings (e.g. `?casa_token=...`)
///
/// Returns `None` for non-article Wiley URLs (e.g. `onlinelibrary.wiley.com/journal/...`).
///
/// # DOI suffix character set
///
/// Wiley DOI suffixes contain alphanumerics, dots, hyphens, underscores, and
/// parentheses (e.g. `adma.202104055`, `(SICI)1097-4636`). Note: ancient SICI-format
/// DOIs also contain `:`, `<`, `>`, `;` which are NOT in the capture allowlist; these
/// characters cause truncation. SICI DOIs (pre-2000) are extremely rare in practice
/// and the truncated result is intentionally documented here.
///
/// # Percent-encoding limitation
///
/// This function operates on the raw (percent-encoded) path from [`Url::path`].
/// If a caller provides a URL where `(` has been encoded as `%28` (e.g. from
/// certain clipboard tools), the `%` character is not in the allowlist and DOI
/// extraction will return `None`. In practice, browsers and Wiley's own links do
/// not percent-encode `(` or `)` in path segments, so this is an edge case.
fn extract_doi_from_wiley_url(input: &str) -> Option<String> {
    let url = Url::parse(input).ok()?;
    let host = url.host_str()?.to_ascii_lowercase();

    if host != "onlinelibrary.wiley.com" {
        return None;
    }

    let path = url.path();

    for prefix in WILEY_DOI_PREFIXES {
        let Some(prefix_start) = path.find(prefix) else {
            continue;
        };
        let suffix_raw = &path[prefix_start + prefix.len()..];

        // Capture suffix using an allowlist. '/' is intentionally excluded: Wiley
        // DOI suffixes do not contain slashes, so excluding '/' prevents capturing
        // trailing path segments like "/full" or "/references".
        // Known limitation: SICI-format DOIs with ':', '<', '>', ';' will be truncated
        // at those characters. See the doc comment above for details.
        let suffix: String = suffix_raw
            .chars()
            .take_while(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '-' | '_' | '(' | ')'))
            .collect();
        let suffix = suffix.trim_end_matches('.');

        if suffix.is_empty() {
            continue;
        }

        return Some(format!("{prefix}{suffix}"));
    }

    None
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

    // ==================== name and priority ====================

    #[test]
    fn test_name_and_priority() {
        let resolver = WileyResolver::new().unwrap();
        assert_eq!(resolver.name(), "wiley");
        assert_eq!(resolver.priority(), ResolverPriority::Specialized);
    }

    // ==================== can_handle ====================

    #[test]
    fn test_can_handle_wiley_doi_10_1002() {
        let resolver = WileyResolver::new().unwrap();
        assert!(resolver.can_handle("10.1002/adma.202104055", InputType::Doi));
    }

    #[test]
    fn test_can_handle_wiley_doi_10_1111() {
        let resolver = WileyResolver::new().unwrap();
        assert!(resolver.can_handle("10.1111/jcmm.16278", InputType::Doi));
    }

    #[test]
    fn test_cannot_handle_non_wiley_doi() {
        let resolver = WileyResolver::new().unwrap();
        assert!(!resolver.can_handle("10.1145/3460418.3479327", InputType::Doi));
        assert!(!resolver.can_handle("10.1109/5.771073", InputType::Doi));
        assert!(!resolver.can_handle("10.1007/s10618-021-00787-x", InputType::Doi));
        assert!(!resolver.can_handle("10.3390/electronics13132567", InputType::Doi));
    }

    #[test]
    fn test_cannot_handle_reference_input() {
        let resolver = WileyResolver::new().unwrap();
        assert!(!resolver.can_handle("10.1002/adma.202104055", InputType::Reference));
    }

    #[test]
    fn test_can_handle_wiley_url_base_doi_path() {
        let resolver = WileyResolver::new().unwrap();
        assert!(resolver.can_handle(
            "https://onlinelibrary.wiley.com/doi/10.1002/adma.202104055",
            InputType::Url
        ));
    }

    #[test]
    fn test_can_handle_wiley_url_pdf() {
        let resolver = WileyResolver::new().unwrap();
        assert!(resolver.can_handle(
            "https://onlinelibrary.wiley.com/doi/pdf/10.1002/adma.202104055",
            InputType::Url
        ));
    }

    #[test]
    fn test_can_handle_wiley_url_abs() {
        let resolver = WileyResolver::new().unwrap();
        assert!(resolver.can_handle(
            "https://onlinelibrary.wiley.com/doi/abs/10.1002/adma.202104055",
            InputType::Url
        ));
    }

    #[test]
    fn test_can_handle_wiley_url_full() {
        let resolver = WileyResolver::new().unwrap();
        assert!(resolver.can_handle(
            "https://onlinelibrary.wiley.com/doi/full/10.1002/adma.202104055",
            InputType::Url
        ));
    }

    #[test]
    fn test_can_handle_wiley_url_epdf() {
        let resolver = WileyResolver::new().unwrap();
        assert!(resolver.can_handle(
            "https://onlinelibrary.wiley.com/doi/epdf/10.1002/adma.202104055",
            InputType::Url
        ));
    }

    #[test]
    fn test_can_handle_wiley_url_http_scheme() {
        // http:// Wiley URLs are accepted — scheme is not checked by extract_doi_from_wiley_url
        let resolver = WileyResolver::new().unwrap();
        assert!(resolver.can_handle(
            "http://onlinelibrary.wiley.com/doi/10.1002/adma.202104055",
            InputType::Url
        ));
    }

    #[test]
    fn test_cannot_handle_non_wiley_url() {
        let resolver = WileyResolver::new().unwrap();
        assert!(!resolver.can_handle(
            "https://dl.acm.org/doi/10.1145/3460418.3479327",
            InputType::Url
        ));
        assert!(!resolver.can_handle(
            "https://ieeexplore.ieee.org/document/7492312",
            InputType::Url
        ));
    }

    // ==================== DOI extraction ====================

    #[test]
    fn test_extract_doi_from_wiley_url_variants() {
        let doi = "10.1002/adma.202104055";
        assert_eq!(
            extract_doi_from_wiley_url(&format!("https://onlinelibrary.wiley.com/doi/{doi}")),
            Some(doi.to_string())
        );
        assert_eq!(
            extract_doi_from_wiley_url(&format!("https://onlinelibrary.wiley.com/doi/pdf/{doi}")),
            Some(doi.to_string())
        );
        assert_eq!(
            extract_doi_from_wiley_url(&format!("https://onlinelibrary.wiley.com/doi/abs/{doi}")),
            Some(doi.to_string())
        );
        assert_eq!(
            extract_doi_from_wiley_url(&format!("https://onlinelibrary.wiley.com/doi/full/{doi}")),
            Some(doi.to_string())
        );
        assert_eq!(
            extract_doi_from_wiley_url(&format!("https://onlinelibrary.wiley.com/doi/epdf/{doi}")),
            Some(doi.to_string())
        );
    }

    #[test]
    fn test_extract_doi_from_wiley_url_10_1111() {
        let doi = "10.1111/adj.13057";
        assert_eq!(
            extract_doi_from_wiley_url(&format!("https://onlinelibrary.wiley.com/doi/pdf/{doi}")),
            Some(doi.to_string())
        );
    }

    #[test]
    fn test_extract_doi_with_query_params() {
        // Institutional access tokens must be stripped
        let result = extract_doi_from_wiley_url(
            "https://onlinelibrary.wiley.com/doi/10.1002/adma.202104055?casa_token=abc123",
        );
        assert_eq!(result, Some("10.1002/adma.202104055".to_string()));
    }

    #[test]
    fn test_extract_doi_returns_none_for_non_article_url() {
        assert_eq!(
            extract_doi_from_wiley_url("https://onlinelibrary.wiley.com/journal/15214095"),
            None
        );
        assert_eq!(
            extract_doi_from_wiley_url("https://onlinelibrary.wiley.com/"),
            None
        );
    }

    #[test]
    fn test_extract_doi_with_parenthesized_suffix() {
        // Modern Wiley DOIs with parentheses in the suffix (e.g. disease subtype codes)
        let result = extract_doi_from_wiley_url(
            "https://onlinelibrary.wiley.com/doi/10.1002/(SICI)1097-4636",
        );
        // The suffix "(SICI)1097-4636" is captured: (, S, I, C, I, ), 1, 0, 9, 7, -, 4, 6, 3, 6
        // Note: ancient full SICI DOIs containing ':', '<', '>', ';' beyond this point
        // would be truncated at those characters — documented known limitation.
        assert_eq!(result, Some("10.1002/(SICI)1097-4636".to_string()));
    }

    // ==================== Wiremock integration tests ====================

    #[tokio::test]
    async fn test_resolve_doi_with_oa_alternative() {
        let Some(mock_server) = start_mock_server_or_skip().await else {
            return;
        };

        Mock::given(method("GET"))
            .and(path("/graph/v1/paper/DOI:10.1002/adma.202104055"))
            .and(query_param("fields", S2_FIELDS))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "title": "Advanced Materials Study",
                "authors": [{"name": "Alice Smith"}, {"name": "Bob Jones"}],
                "year": 2021,
                "externalIds": {"ArXiv": "2108.04144"},
                "openAccessPdf": {"url": "https://arxiv.org/pdf/2108.04144"}
            })))
            .mount(&mock_server)
            .await;

        let resolver = WileyResolver::with_base_url(mock_server.uri()).unwrap();
        let ctx = ResolveContext::default();
        let result = resolver
            .resolve("10.1002/adma.202104055", &ctx)
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
            .and(path("/graph/v1/paper/DOI:10.1002/adma.202104055"))
            .and(query_param("fields", S2_FIELDS))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "title": "Advanced Materials Study",
                "authors": [{"name": "Alice Smith"}],
                "year": 2021,
                "externalIds": {"ArXiv": "2108.04144"},
                "openAccessPdf": null
            })))
            .mount(&mock_server)
            .await;

        let resolver = WileyResolver::with_base_url(mock_server.uri()).unwrap();
        let ctx = ResolveContext::default();
        let result = resolver
            .resolve("10.1002/adma.202104055", &ctx)
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
    async fn test_resolve_doi_no_oa_falls_back_to_wiley() {
        let Some(mock_server) = start_mock_server_or_skip().await else {
            return;
        };

        Mock::given(method("GET"))
            .and(path("/graph/v1/paper/DOI:10.1002/adma.202104055"))
            .and(query_param("fields", S2_FIELDS))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "title": "Paywalled Paper",
                "authors": [{"name": "Carol White"}],
                "year": 2022,
                "externalIds": {},
                "openAccessPdf": null
            })))
            .mount(&mock_server)
            .await;

        let resolver = WileyResolver::with_base_url(mock_server.uri()).unwrap();
        let ctx = ResolveContext::default();
        let result = resolver
            .resolve("10.1002/adma.202104055", &ctx)
            .await
            .unwrap();

        match result {
            ResolveStep::Url(resolved) => {
                assert_eq!(
                    resolved.url,
                    "https://onlinelibrary.wiley.com/doi/pdf/10.1002/adma.202104055"
                );
            }
            other => panic!("Expected Url fallback, got: {other:?}"),
        }
    }

    #[tokio::test]
    async fn test_resolve_doi_oa_on_wiley_rejected() {
        let Some(mock_server) = start_mock_server_or_skip().await else {
            return;
        };

        // OA PDF is on onlinelibrary.wiley.com — should be skipped; no ArXiv either
        Mock::given(method("GET"))
            .and(path("/graph/v1/paper/DOI:10.1002/adma.202104055"))
            .and(query_param("fields", S2_FIELDS))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "title": "Some Wiley Paper",
                "authors": [{"name": "Dan Brown"}],
                "year": 2022,
                "externalIds": {},
                "openAccessPdf": {
                    "url": "https://onlinelibrary.wiley.com/doi/pdf/10.1002/adma.202104055"
                }
            })))
            .mount(&mock_server)
            .await;

        let resolver = WileyResolver::with_base_url(mock_server.uri()).unwrap();
        let ctx = ResolveContext::default();
        let result = resolver
            .resolve("10.1002/adma.202104055", &ctx)
            .await
            .unwrap();

        match result {
            ResolveStep::Url(resolved) => {
                assert_eq!(
                    resolved.url,
                    "https://onlinelibrary.wiley.com/doi/pdf/10.1002/adma.202104055"
                );
            }
            other => panic!("Expected Url fallback after Wiley OA rejection, got: {other:?}"),
        }
    }

    #[tokio::test]
    async fn test_resolve_doi_not_in_s2() {
        let Some(mock_server) = start_mock_server_or_skip().await else {
            return;
        };

        Mock::given(method("GET"))
            .and(path("/graph/v1/paper/DOI:10.1002/adma.202104055"))
            .and(query_param("fields", S2_FIELDS))
            .respond_with(ResponseTemplate::new(404))
            .mount(&mock_server)
            .await;

        let resolver = WileyResolver::with_base_url(mock_server.uri()).unwrap();
        let ctx = ResolveContext::default();
        let result = resolver
            .resolve("10.1002/adma.202104055", &ctx)
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
            .and(path("/graph/v1/paper/DOI:10.1002/adma.202104055"))
            .respond_with(ResponseTemplate::new(429))
            .mount(&mock_server)
            .await;

        let resolver = WileyResolver::with_base_url(mock_server.uri()).unwrap();
        let ctx = ResolveContext::default();
        let result = resolver
            .resolve("10.1002/adma.202104055", &ctx)
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
    async fn test_resolve_doi_metadata() {
        let Some(mock_server) = start_mock_server_or_skip().await else {
            return;
        };

        Mock::given(method("GET"))
            .and(path("/graph/v1/paper/DOI:10.1002/adma.202104055"))
            .and(query_param("fields", S2_FIELDS))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "title": "Advanced Materials Study",
                "authors": [{"name": "Alice Smith"}, {"name": "Bob Jones"}],
                "year": 2021,
                "externalIds": {},
                "openAccessPdf": null
            })))
            .mount(&mock_server)
            .await;

        let resolver = WileyResolver::with_base_url(mock_server.uri()).unwrap();
        let ctx = ResolveContext::default();
        let result = resolver
            .resolve("10.1002/adma.202104055", &ctx)
            .await
            .unwrap();

        match result {
            ResolveStep::Url(resolved) => {
                let m = &resolved.metadata;
                assert_eq!(
                    m.get("title").map(String::as_str),
                    Some("Advanced Materials Study")
                );
                assert_eq!(
                    m.get("authors").map(String::as_str),
                    Some("Alice Smith; Bob Jones")
                );
                assert_eq!(m.get("year").map(String::as_str), Some("2021"));
                assert_eq!(
                    m.get("doi").map(String::as_str),
                    Some("10.1002/adma.202104055")
                );
                assert_eq!(
                    m.get("source_url").map(String::as_str),
                    Some("https://doi.org/10.1002/adma.202104055")
                );
            }
            other => panic!("Expected Url, got: {other:?}"),
        }
    }

    #[tokio::test]
    async fn test_resolve_url_success() {
        let Some(mock_server) = start_mock_server_or_skip().await else {
            return;
        };

        Mock::given(method("GET"))
            .and(path("/graph/v1/paper/DOI:10.1002/adma.202104055"))
            .and(query_param("fields", S2_FIELDS))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "title": "Advanced Materials Study",
                "authors": [{"name": "Alice Smith"}],
                "year": 2021,
                "externalIds": {"ArXiv": "2108.04144"},
                "openAccessPdf": {"url": "https://arxiv.org/pdf/2108.04144"}
            })))
            .mount(&mock_server)
            .await;

        let resolver = WileyResolver::with_base_url(mock_server.uri()).unwrap();
        let ctx = ResolveContext::default();
        let result = resolver
            .resolve(
                "https://onlinelibrary.wiley.com/doi/10.1002/adma.202104055",
                &ctx,
            )
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
    async fn test_resolve_doi_10_1111_prefix() {
        let Some(mock_server) = start_mock_server_or_skip().await else {
            return;
        };

        Mock::given(method("GET"))
            .and(path("/graph/v1/paper/DOI:10.1111/adj.13057"))
            .and(query_param("fields", S2_FIELDS))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "title": "Dental Journal Paper",
                "authors": [{"name": "Eve"}],
                "year": 2023,
                "externalIds": {"ArXiv": "2301.12345"},
                "openAccessPdf": null
            })))
            .mount(&mock_server)
            .await;

        let resolver = WileyResolver::with_base_url(mock_server.uri()).unwrap();
        let ctx = ResolveContext::default();
        let result = resolver.resolve("10.1111/adj.13057", &ctx).await.unwrap();

        match result {
            ResolveStep::Url(resolved) => {
                assert_eq!(resolved.url, "https://arxiv.org/pdf/2301.12345");
            }
            other => panic!("Expected Url, got: {other:?}"),
        }
    }

    #[tokio::test]
    async fn test_resolve_doi_empty_s2_response() {
        let Some(mock_server) = start_mock_server_or_skip().await else {
            return;
        };

        // All nullable fields are null — must still produce Wiley fallback URL with doi/source_url metadata
        Mock::given(method("GET"))
            .and(path("/graph/v1/paper/DOI:10.1002/adma.202104055"))
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

        let resolver = WileyResolver::with_base_url(mock_server.uri()).unwrap();
        let ctx = ResolveContext::default();
        let result = resolver
            .resolve("10.1002/adma.202104055", &ctx)
            .await
            .unwrap();

        match result {
            ResolveStep::Url(resolved) => {
                assert_eq!(
                    resolved.url,
                    "https://onlinelibrary.wiley.com/doi/pdf/10.1002/adma.202104055"
                );
                assert_eq!(
                    resolved.metadata.get("doi").map(String::as_str),
                    Some("10.1002/adma.202104055")
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
            .and(path("/graph/v1/paper/DOI:10.1002/adma.202104055"))
            .and(query_param("fields", S2_FIELDS))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "title": "A Paper",
                "authors": [{"name": "Alice"}],
                "year": 2021,
                "externalIds": {"ArXiv": "2108.04144"},
                "openAccessPdf": {"url": "http://arxiv.org/pdf/2108.04144"}
            })))
            .mount(&mock_server)
            .await;

        let resolver = WileyResolver::with_base_url(mock_server.uri()).unwrap();
        let ctx = ResolveContext::default();
        let result = resolver
            .resolve("10.1002/adma.202104055", &ctx)
            .await
            .unwrap();

        match result {
            ResolveStep::Url(resolved) => {
                // http:// OA URL rejected; ArXiv external ID used instead
                assert_eq!(resolved.url, "https://arxiv.org/pdf/2108.04144");
            }
            other => {
                panic!("Expected Url from ArXiv fallback after http:// rejection, got: {other:?}")
            }
        }
    }
}
