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

use std::collections::HashMap;

use async_trait::async_trait;
use reqwest::Client;
use serde::Deserialize;
use tracing::{debug, warn};
use url::Url;

use crate::parser::InputType;

use super::http_client::{build_resolver_http_client, standard_user_agent};
use super::utils::looks_like_doi;
use super::{ResolveContext, ResolveError, ResolveStep, ResolvedUrl, Resolver, ResolverPriority};

// ==================== Constants ====================

const DEFAULT_S2_BASE_URL: &str = "https://api.semanticscholar.org";
const ACM_DOI_PREFIX: &str = "10.1145/";
const S2_FIELDS: &str = "externalIds,openAccessPdf,title,authors,year";

// ==================== Semantic Scholar Response Types ====================

#[derive(Debug, Deserialize)]
struct S2PaperResponse {
    title: Option<String>,
    authors: Option<Vec<S2Author>>,
    year: Option<i32>,
    #[serde(rename = "externalIds")]
    external_ids: Option<S2ExternalIds>,
    #[serde(rename = "openAccessPdf")]
    open_access_pdf: Option<S2OpenAccessPdf>,
}

#[derive(Debug, Deserialize)]
struct S2Author {
    name: Option<String>,
}

#[derive(Debug, Deserialize)]
struct S2ExternalIds {
    #[serde(rename = "ArXiv")]
    arxiv: Option<String>,
}

#[derive(Debug, Deserialize)]
struct S2OpenAccessPdf {
    url: Option<String>,
}

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
        Ok(Self { client, s2_base_url })
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

        self.resolve_via_s2(&doi, input).await
    }
}

// ==================== Resolution logic ====================

impl AcmResolver {
    async fn resolve_via_s2(
        &self,
        doi: &str,
        original_input: &str,
    ) -> Result<ResolveStep, ResolveError> {
        // Semantic Scholar accepts raw (unencoded) DOI in the path.
        let url = format!(
            "{}/graph/v1/paper/DOI:{}?fields={}",
            self.s2_base_url, doi, S2_FIELDS
        );

        debug!(s2_url = %url, "Querying Semantic Scholar for ACM DOI");

        let response = match self.client.get(&url).send().await {
            Ok(resp) => resp,
            Err(e) => {
                warn!(error = %e, doi, "Semantic Scholar request failed");
                return Ok(ResolveStep::Failed(ResolveError::resolution_failed(
                    original_input,
                    "Cannot reach Semantic Scholar API. \
                     Why: network error or API unavailable. \
                     Fix: check your internet connection and retry.",
                )));
            }
        };

        let status = response.status();
        if !status.is_success() {
            let reason = s2_error_reason(status.as_u16());
            debug!(status = status.as_u16(), %reason, "Semantic Scholar API error");
            return Ok(ResolveStep::Failed(ResolveError::resolution_failed(
                original_input,
                &reason,
            )));
        }

        let paper: S2PaperResponse = match response.json().await {
            Ok(parsed) => parsed,
            Err(e) => {
                warn!(error = %e, doi, "Failed to parse Semantic Scholar response");
                return Ok(ResolveStep::body_parse_failed(original_input, "Semantic Scholar"));
            }
        };

        let pdf_url = select_best_pdf_url(&paper, doi);
        let metadata = build_metadata(&paper, doi);

        debug!(pdf_url = %pdf_url, "Resolved ACM paper to PDF URL");
        Ok(ResolveStep::Url(ResolvedUrl::with_metadata(pdf_url, metadata)))
    }
}

// ==================== URL selection ====================

/// Selects the best available PDF URL from a Semantic Scholar response.
///
/// Priority:
/// 1. `openAccessPdf.url` if it is `https://` scheme and NOT on `dl.acm.org`
/// 2. arXiv PDF constructed from `externalIds.ArXiv`
/// 3. ACM PDF URL fallback (`dl.acm.org/doi/pdf/{doi}`)
fn select_best_pdf_url(paper: &S2PaperResponse, doi: &str) -> String {
    // Priority 1: openAccessPdf on a non-ACM https:// domain
    if let Some(oa) = &paper.open_access_pdf {
        if let Some(oa_url) = &oa.url {
            if is_usable_oa_url(oa_url) {
                debug!(oa_url = %oa_url, "Using open-access PDF from Semantic Scholar");
                return oa_url.clone();
            }
            debug!("Semantic Scholar OA PDF skipped (ACM domain or non-HTTPS)");
        }
    }

    // Priority 2: arXiv external ID
    if let Some(ids) = &paper.external_ids {
        if let Some(arxiv_id) = &ids.arxiv {
            let arxiv_url = format!("https://arxiv.org/pdf/{arxiv_id}");
            debug!(arxiv_id = %arxiv_id, "Constructing arXiv PDF URL from external ID");
            return arxiv_url;
        }
    }

    // Fallback: ACM PDF URL (may require auth via `downloader auth capture`)
    let acm_url = format!("https://dl.acm.org/doi/pdf/{doi}");
    debug!("No open-access alternative found; falling back to ACM PDF URL");
    acm_url
}

/// Returns true if `url` is a usable open-access PDF: `https://` scheme and not on `dl.acm.org`.
fn is_usable_oa_url(url: &str) -> bool {
    if !url.starts_with("https://") {
        return false;
    }
    Url::parse(url)
        .ok()
        .and_then(|parsed| parsed.host_str().map(str::to_ascii_lowercase))
        .is_some_and(|host| host != "dl.acm.org" && !host.ends_with(".dl.acm.org"))
}

// ==================== Metadata ====================

fn build_metadata(paper: &S2PaperResponse, doi: &str) -> HashMap<String, String> {
    let mut metadata = HashMap::new();

    metadata.insert("doi".to_string(), doi.to_string());
    metadata.insert("source_url".to_string(), format!("https://doi.org/{doi}"));

    if let Some(title) = &paper.title {
        if !title.is_empty() {
            metadata.insert("title".to_string(), title.clone());
        }
    }

    if let Some(authors) = &paper.authors {
        let names: Vec<String> = authors
            .iter()
            .filter_map(|a| a.name.as_deref())
            .filter(|n| !n.is_empty())
            .map(str::to_string)
            .collect();
        if !names.is_empty() {
            metadata.insert("authors".to_string(), names.join("; "));
        }
    }

    if let Some(year) = paper.year {
        metadata.insert("year".to_string(), year.to_string());
    }

    metadata
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

// ==================== Error helpers ====================

fn s2_error_reason(status: u16) -> String {
    match status {
        404 => "Paper not found in Semantic Scholar. \
                Why: this DOI may not be indexed yet. \
                Fix: the resolver will fall through to Crossref."
            .to_string(),
        429 => "Semantic Scholar rate limit exceeded. \
                Why: too many requests in a short period. \
                Fix: wait and retry, or reduce batch size."
            .to_string(),
        s if s >= 500 => "Semantic Scholar API unavailable. \
                          Why: server error. \
                          Fix: wait and retry."
            .to_string(),
        s => format!(
            "Semantic Scholar API returned HTTP {s}. \
             Fix: check the DOI and retry."
        ),
    }
}

// ==================== Tests ====================

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
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
        assert!(!resolver.can_handle(
            "https://dl.acm.org/conference/chi",
            InputType::Url
        ));
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

    // ==================== is_usable_oa_url ====================

    #[test]
    fn test_is_usable_oa_url_accepts_arxiv_https() {
        assert!(is_usable_oa_url("https://arxiv.org/pdf/2108.04144"));
    }

    #[test]
    fn test_is_usable_oa_url_rejects_dl_acm_org() {
        assert!(!is_usable_oa_url(
            "https://dl.acm.org/doi/pdf/10.1145/3460418.3479327"
        ));
    }

    #[test]
    fn test_is_usable_oa_url_rejects_dl_acm_org_subdomain() {
        assert!(!is_usable_oa_url(
            "https://mirror.dl.acm.org/doi/pdf/10.1145/3460418.3479327"
        ));
    }

    #[test]
    fn test_is_usable_oa_url_rejects_http_scheme() {
        assert!(!is_usable_oa_url("http://arxiv.org/pdf/2108.04144"));
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
            .resolve(
                "https://dl.acm.org/doi/10.1145/3460418.3479327",
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
                assert_eq!(m.get("title").map(String::as_str), Some("Smart Multimodal Interaction"));
                assert_eq!(m.get("authors").map(String::as_str), Some("Alice Smith; Bob Jones"));
                assert_eq!(m.get("year").map(String::as_str), Some("2021"));
                assert_eq!(m.get("doi").map(String::as_str), Some("10.1145/3460418.3479327"));
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
