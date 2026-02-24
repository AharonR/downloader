//! Crossref DOI resolver - resolves DOIs to downloadable URLs via the Crossref API.
//!
//! The [`CrossrefResolver`] calls the Crossref REST API to look up metadata for DOIs
//! and extract PDF URLs from the response. When no PDF link is available, it redirects
//! to the `doi.org` URL for the `DirectResolver` to handle.

use async_trait::async_trait;
use reqwest::Client;
use serde::Deserialize;
use std::collections::HashMap;
use tracing::{debug, warn};

use crate::parser::InputType;

use super::http_client::{build_resolver_http_client, standard_user_agent};
use super::{ResolveContext, ResolveError, ResolveStep, ResolvedUrl, Resolver, ResolverPriority};

/// Default Crossref API base URL.
const DEFAULT_BASE_URL: &str = "https://api.crossref.org";

// ==================== Crossref API Response Types ====================

/// Top-level Crossref API response.
#[derive(Debug, Deserialize)]
pub(crate) struct CrossrefResponse {
    #[allow(dead_code)] // Deserialized for completeness; may be used for validation later
    pub status: String,
    pub message: CrossrefMessage,
}

/// The `message` field from a Crossref works response.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub(crate) struct CrossrefMessage {
    pub title: Option<Vec<String>>,
    pub author: Option<Vec<CrossrefAuthor>>,
    pub link: Option<Vec<CrossrefLink>>,
    pub published: Option<CrossrefDate>,
    pub published_print: Option<CrossrefDate>,
    pub published_online: Option<CrossrefDate>,
}

/// An author entry from the Crossref response.
#[derive(Debug, Deserialize)]
pub(crate) struct CrossrefAuthor {
    pub given: Option<String>,
    pub family: Option<String>,
}

/// A resource link from the Crossref response.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub(crate) struct CrossrefLink {
    /// The URL field is uppercase in the Crossref response.
    #[serde(rename = "URL")]
    pub url: String,
    pub content_type: Option<String>,
    #[allow(dead_code)] // Deserialized for Debug output; useful for troubleshooting API responses
    pub content_version: Option<String>,
    pub intended_application: Option<String>,
}

/// A date entry from the Crossref response.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub(crate) struct CrossrefDate {
    pub date_parts: Option<Vec<Vec<Option<i32>>>>,
}

// ==================== CrossrefResolver ====================

/// Resolves DOIs to downloadable URLs via the Crossref REST API.
///
/// The resolver queries `https://api.crossref.org/works/{doi}` and extracts
/// PDF URLs from the `message.link` array. When no PDF link is found, it
/// redirects to `https://doi.org/{doi}` for fallback handling.
///
/// # Polite Pool
///
/// All requests include a `mailto` query parameter to access Crossref's
/// polite pool, which provides higher rate limits (10 req/s vs 5 req/s).
pub struct CrossrefResolver {
    client: Client,
    base_url: String,
    mailto: String,
}

impl CrossrefResolver {
    /// Creates a new `CrossrefResolver` configured for the Crossref polite pool.
    ///
    /// # Arguments
    ///
    /// * `mailto` - Contact email for Crossref polite pool access
    ///
    /// # Errors
    ///
    /// Returns [`ResolveError`] if HTTP client construction fails.
    #[tracing::instrument(skip_all, fields(mailto))]
    pub fn new(mailto: impl Into<String>) -> Result<Self, ResolveError> {
        Self::build(mailto.into(), DEFAULT_BASE_URL.to_string())
    }

    /// Creates a `CrossrefResolver` with a custom base URL (for testing with wiremock).
    ///
    /// # Errors
    ///
    /// Returns [`ResolveError`] if HTTP client construction fails.
    #[tracing::instrument(skip_all, fields(mailto, base_url))]
    pub fn with_base_url(
        mailto: impl Into<String>,
        base_url: impl Into<String>,
    ) -> Result<Self, ResolveError> {
        Self::build(mailto.into(), base_url.into())
    }

    fn build(mailto: String, base_url: String) -> Result<Self, ResolveError> {
        if mailto.chars().any(|c| c == '\n' || c == '\r' || c == '\0') {
            return Err(ResolveError::resolution_failed(
                &mailto,
                "mailto contains invalid control characters",
            ));
        }
        let user_agent = standard_user_agent("crossref");
        let client = build_resolver_http_client("crossref", user_agent, None)?;

        Ok(Self {
            client,
            base_url,
            mailto,
        })
    }
}

impl std::fmt::Debug for CrossrefResolver {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CrossrefResolver")
            .field("base_url", &self.base_url)
            .field("mailto", &self.mailto)
            .finish_non_exhaustive()
    }
}

#[async_trait]
impl Resolver for CrossrefResolver {
    fn name(&self) -> &'static str {
        "crossref"
    }

    fn priority(&self) -> ResolverPriority {
        ResolverPriority::General
    }

    fn can_handle(&self, _input: &str, input_type: InputType) -> bool {
        input_type == InputType::Doi
    }

    #[tracing::instrument(skip(self, _ctx), fields(resolver = "crossref", doi = %input))]
    async fn resolve(
        &self,
        input: &str,
        _ctx: &ResolveContext,
    ) -> Result<ResolveStep, ResolveError> {
        let encoded_doi = urlencoding::encode(input);
        let encoded_mailto = urlencoding::encode(&self.mailto);
        let url = format!(
            "{}/works/{}?mailto={}",
            self.base_url, encoded_doi, encoded_mailto
        );

        debug!(api_url = %url, "Calling Crossref API");

        let response = match self.client.get(&url).send().await {
            Ok(resp) => resp,
            Err(e) => {
                warn!(error = %e, "Crossref API request failed");
                return Ok(ResolveStep::Failed(ResolveError::resolution_failed(
                    input,
                    "Cannot reach Crossref API. Check your network connection.",
                )));
            }
        };

        // Log rate limit headers
        if let Some(limit) = response.headers().get("x-rate-limit-limit") {
            debug!(rate_limit = ?limit, "Crossref rate limit");
        }
        if let Some(interval) = response.headers().get("x-rate-limit-interval") {
            debug!(rate_interval = ?interval, "Crossref rate interval");
        }

        let status = response.status();
        if !status.is_success() {
            let reason = match status.as_u16() {
                404 => "DOI not found in Crossref database".to_string(),
                429 => "Crossref rate limit exceeded. Try again in a few seconds.".to_string(),
                s if s >= 500 => "Crossref API unavailable. Try again later.".to_string(),
                s => format!("Crossref API returned HTTP {s}"),
            };
            debug!(status = status.as_u16(), %reason, "Crossref API error");
            return Ok(ResolveStep::Failed(ResolveError::resolution_failed(
                input, &reason,
            )));
        }

        let body = match response.json::<CrossrefResponse>().await {
            Ok(parsed) => parsed,
            Err(e) => {
                warn!(error = %e, "Failed to parse Crossref response JSON");
                return Ok(ResolveStep::Failed(ResolveError::resolution_failed(
                    input,
                    "Unexpected Crossref API response format",
                )));
            }
        };

        if !body.status.eq_ignore_ascii_case("ok") {
            warn!(status = %body.status, "Crossref response status was not ok");
            return Ok(ResolveStep::Failed(ResolveError::resolution_failed(
                input,
                "Unexpected Crossref response status",
            )));
        }

        let metadata = extract_metadata(&body.message, input);
        let links = body.message.link.as_deref().unwrap_or(&[]);

        if let Some(pdf_url) = extract_pdf_url(links) {
            debug!(pdf_url = %pdf_url, "Found PDF URL in Crossref response");
            Ok(ResolveStep::Url(ResolvedUrl::with_metadata(
                pdf_url, metadata,
            )))
        } else {
            let doi_url = format!("https://doi.org/{input}");
            debug!(redirect_url = %doi_url, "No PDF link found, redirecting to doi.org");
            Ok(ResolveStep::Redirect(doi_url))
        }
    }
}

// ==================== Extraction Helpers ====================

/// Extracts the best PDF URL from Crossref link entries.
///
/// Priority:
/// 1. Links with `content-type: "application/pdf"`
/// 2. Links with `intended-application: "similarity-checking"` or `"text-mining"`
fn extract_pdf_url(links: &[CrossrefLink]) -> Option<String> {
    // First pass: look for explicit PDF content-type
    for link in links {
        if let Some(ct) = &link.content_type {
            if is_pdf_content_type(ct) {
                return Some(link.url.clone());
            }
        }
    }

    // Second pass: look for text-mining or similarity-checking links
    for link in links {
        if let Some(app) = &link.intended_application {
            if is_fallback_application(app) {
                return Some(link.url.clone());
            }
        }
    }

    None
}

fn is_pdf_content_type(content_type: &str) -> bool {
    content_type
        .split(';')
        .next()
        .map(str::trim)
        .is_some_and(|mime| mime.eq_ignore_ascii_case("application/pdf"))
}

fn is_fallback_application(intended_application: &str) -> bool {
    intended_application.eq_ignore_ascii_case("text-mining")
        || intended_application.eq_ignore_ascii_case("similarity-checking")
}

/// Extracts metadata from a Crossref message into a `HashMap`.
fn extract_metadata(message: &CrossrefMessage, doi: &str) -> HashMap<String, String> {
    let mut metadata = HashMap::new();

    metadata.insert("doi".to_string(), doi.to_string());

    // Title
    if let Some(titles) = &message.title {
        if let Some(title) = titles.first() {
            metadata.insert("title".to_string(), title.clone());
        }
    }

    // Authors
    if let Some(authors) = &message.author {
        let formatted: Vec<String> = authors
            .iter()
            .map(|a| match (&a.family, &a.given) {
                (Some(f), Some(g)) => format!("{f}, {g}"),
                (Some(f), None) => f.clone(),
                (None, Some(g)) => g.clone(),
                (None, None) => String::new(),
            })
            .filter(|s| !s.is_empty())
            .collect();
        if !formatted.is_empty() {
            metadata.insert("authors".to_string(), formatted.join("; "));
        }
    }

    // Year - try published, then published-print, then published-online
    let year = extract_year(message.published.as_ref())
        .or_else(|| extract_year(message.published_print.as_ref()))
        .or_else(|| extract_year(message.published_online.as_ref()));
    if let Some(y) = year {
        metadata.insert("year".to_string(), y.to_string());
    }

    metadata
}

/// Extracts the year from a Crossref date field.
fn extract_year(date: Option<&CrossrefDate>) -> Option<i32> {
    date.and_then(|d| d.date_parts.as_ref())
        .and_then(|parts| parts.first())
        .and_then(|inner| inner.first())
        .copied()
        .flatten()
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use crate::test_support::socket_guard::start_mock_server_or_skip;
    use wiremock::matchers::{header, method, path, path_regex, query_param};
    use wiremock::{Mock, ResponseTemplate};

    // ==================== Serde Deserialization Tests ====================

    #[test]
    fn test_crossref_response_deserialize_full() {
        let json = serde_json::json!({
            "status": "ok",
            "message": {
                "title": ["A Test Paper"],
                "author": [
                    {"given": "John", "family": "Smith"},
                    {"given": "Jane", "family": "Doe"}
                ],
                "link": [{
                    "URL": "https://publisher.com/paper.pdf",
                    "content-type": "application/pdf",
                    "content-version": "vor",
                    "intended-application": "text-mining"
                }],
                "published": {"date-parts": [[2024, 6, 15]]},
                "published-print": {"date-parts": [[2024, 7, 1]]},
                "published-online": {"date-parts": [[2024, 5, 30]]}
            }
        });

        let resp: CrossrefResponse = serde_json::from_value(json).unwrap();
        assert_eq!(resp.status, "ok");
        assert_eq!(resp.message.title.unwrap()[0], "A Test Paper");
        assert_eq!(resp.message.author.unwrap().len(), 2);
        assert_eq!(
            resp.message.link.unwrap()[0].url,
            "https://publisher.com/paper.pdf"
        );
    }

    #[test]
    fn test_crossref_response_deserialize_minimal() {
        let json = serde_json::json!({
            "status": "ok",
            "message": {}
        });

        let resp: CrossrefResponse = serde_json::from_value(json).unwrap();
        assert_eq!(resp.status, "ok");
        assert!(resp.message.title.is_none());
        assert!(resp.message.author.is_none());
        assert!(resp.message.link.is_none());
        assert!(resp.message.published.is_none());
    }

    #[test]
    fn test_crossref_link_deserialize_with_uppercase_url() {
        let json = serde_json::json!({
            "URL": "https://example.com/paper.pdf",
            "content-type": "application/pdf"
        });

        let link: CrossrefLink = serde_json::from_value(json).unwrap();
        assert_eq!(link.url, "https://example.com/paper.pdf");
        assert_eq!(link.content_type.unwrap(), "application/pdf");
    }

    #[test]
    fn test_crossref_author_deserialize_missing_given() {
        let json = serde_json::json!({"family": "Consortium"});

        let author: CrossrefAuthor = serde_json::from_value(json).unwrap();
        assert_eq!(author.family.unwrap(), "Consortium");
        assert!(author.given.is_none());
    }

    #[test]
    fn test_crossref_date_deserialize_partial() {
        let json = serde_json::json!({"date-parts": [[2024]]});

        let date: CrossrefDate = serde_json::from_value(json).unwrap();
        let parts = date.date_parts.unwrap();
        assert_eq!(parts[0][0], Some(2024));
    }

    // ==================== PDF URL Extraction Tests ====================

    #[test]
    fn test_extract_pdf_url_prefers_pdf_content_type() {
        let links = vec![
            CrossrefLink {
                url: "https://example.com/xml".to_string(),
                content_type: Some("text/xml".to_string()),
                content_version: None,
                intended_application: Some("text-mining".to_string()),
            },
            CrossrefLink {
                url: "https://example.com/paper.pdf".to_string(),
                content_type: Some("application/pdf".to_string()),
                content_version: None,
                intended_application: None,
            },
        ];
        assert_eq!(
            extract_pdf_url(&links),
            Some("https://example.com/paper.pdf".to_string())
        );
    }

    #[test]
    fn test_extract_pdf_url_fallback_text_mining() {
        let links = vec![CrossrefLink {
            url: "https://example.com/fulltext".to_string(),
            content_type: Some("text/html".to_string()),
            content_version: None,
            intended_application: Some("text-mining".to_string()),
        }];
        assert_eq!(
            extract_pdf_url(&links),
            Some("https://example.com/fulltext".to_string())
        );
    }

    #[test]
    fn test_extract_pdf_url_empty_links_returns_none() {
        assert_eq!(extract_pdf_url(&[]), None);
    }

    #[test]
    fn test_extract_pdf_url_no_matching_links_returns_none() {
        let links = vec![CrossrefLink {
            url: "https://example.com/something".to_string(),
            content_type: Some("text/html".to_string()),
            content_version: None,
            intended_application: Some("unspecified".to_string()),
        }];
        assert_eq!(extract_pdf_url(&links), None);
    }

    #[test]
    fn test_extract_pdf_url_content_type_case_insensitive_with_params() {
        let links = vec![CrossrefLink {
            url: "https://example.com/paper.pdf".to_string(),
            content_type: Some("Application/PDF; charset=utf-8".to_string()),
            content_version: None,
            intended_application: None,
        }];
        assert_eq!(
            extract_pdf_url(&links),
            Some("https://example.com/paper.pdf".to_string())
        );
    }

    #[test]
    fn test_extract_pdf_url_fallback_application_case_insensitive() {
        let links = vec![CrossrefLink {
            url: "https://example.com/fulltext".to_string(),
            content_type: Some("text/html".to_string()),
            content_version: None,
            intended_application: Some("Similarity-Checking".to_string()),
        }];
        assert_eq!(
            extract_pdf_url(&links),
            Some("https://example.com/fulltext".to_string())
        );
    }

    // ==================== Metadata Extraction Tests ====================

    #[test]
    fn test_extract_metadata_full() {
        let message = CrossrefMessage {
            title: Some(vec!["Test Paper Title".to_string()]),
            author: Some(vec![
                CrossrefAuthor {
                    given: Some("John".to_string()),
                    family: Some("Smith".to_string()),
                },
                CrossrefAuthor {
                    given: Some("Jane".to_string()),
                    family: Some("Doe".to_string()),
                },
            ]),
            link: None,
            published: Some(CrossrefDate {
                date_parts: Some(vec![vec![Some(2024), Some(6), Some(15)]]),
            }),
            published_print: None,
            published_online: None,
        };

        let meta = extract_metadata(&message, "10.1234/test");
        assert_eq!(meta.get("title").unwrap(), "Test Paper Title");
        assert_eq!(meta.get("authors").unwrap(), "Smith, John; Doe, Jane");
        assert_eq!(meta.get("year").unwrap(), "2024");
        assert_eq!(meta.get("doi").unwrap(), "10.1234/test");
    }

    #[test]
    fn test_extract_metadata_missing_title() {
        let message = CrossrefMessage {
            title: None,
            author: None,
            link: None,
            published: None,
            published_print: None,
            published_online: None,
        };

        let meta = extract_metadata(&message, "10.1234/test");
        assert!(!meta.contains_key("title"));
        assert!(!meta.contains_key("authors"));
        assert!(!meta.contains_key("year"));
        assert_eq!(meta.get("doi").unwrap(), "10.1234/test");
    }

    #[test]
    fn test_extract_metadata_multiple_authors() {
        let message = CrossrefMessage {
            title: None,
            author: Some(vec![
                CrossrefAuthor {
                    given: Some("A".to_string()),
                    family: Some("First".to_string()),
                },
                CrossrefAuthor {
                    given: None,
                    family: Some("Consortium".to_string()),
                },
                CrossrefAuthor {
                    given: Some("C".to_string()),
                    family: Some("Third".to_string()),
                },
            ]),
            link: None,
            published: None,
            published_print: None,
            published_online: None,
        };

        let meta = extract_metadata(&message, "10.1234/test");
        assert_eq!(
            meta.get("authors").unwrap(),
            "First, A; Consortium; Third, C"
        );
    }

    #[test]
    fn test_extract_metadata_year_from_published_print() {
        let message = CrossrefMessage {
            title: None,
            author: None,
            link: None,
            published: None,
            published_print: Some(CrossrefDate {
                date_parts: Some(vec![vec![Some(2023)]]),
            }),
            published_online: None,
        };

        let meta = extract_metadata(&message, "10.1234/test");
        assert_eq!(meta.get("year").unwrap(), "2023");
    }

    #[test]
    fn test_extract_metadata_year_from_published_online() {
        let message = CrossrefMessage {
            title: None,
            author: None,
            link: None,
            published: None,
            published_print: None,
            published_online: Some(CrossrefDate {
                date_parts: Some(vec![vec![Some(2022)]]),
            }),
        };

        let meta = extract_metadata(&message, "10.1234/test");
        assert_eq!(meta.get("year").unwrap(), "2022");
    }

    #[test]
    fn test_extract_metadata_no_date() {
        let message = CrossrefMessage {
            title: None,
            author: None,
            link: None,
            published: None,
            published_print: None,
            published_online: None,
        };

        let meta = extract_metadata(&message, "10.1234/test");
        assert!(!meta.contains_key("year"));
    }

    // ==================== Resolver Trait Tests ====================

    #[test]
    fn test_crossref_resolver_name() {
        let resolver = CrossrefResolver::new("test@example.com").unwrap();
        assert_eq!(resolver.name(), "crossref");
    }

    #[test]
    fn test_crossref_resolver_priority() {
        let resolver = CrossrefResolver::new("test@example.com").unwrap();
        assert_eq!(resolver.priority(), ResolverPriority::General);
    }

    #[test]
    fn test_crossref_resolver_can_handle_doi() {
        let resolver = CrossrefResolver::new("test@example.com").unwrap();
        assert!(resolver.can_handle("10.1234/test", InputType::Doi));
    }

    #[test]
    fn test_crossref_resolver_cannot_handle_url() {
        let resolver = CrossrefResolver::new("test@example.com").unwrap();
        assert!(!resolver.can_handle("https://example.com", InputType::Url));
    }

    #[test]
    fn regression_crossref_constructor_rejects_invalid_mailto_header_value() {
        let result = CrossrefResolver::new("invalid\nmailto@example.com");
        assert!(
            result.is_err(),
            "constructor should fail for newline-containing mailto values"
        );
    }

    #[test]
    fn regression_crossref_with_base_url_rejects_invalid_mailto_header_value() {
        let result = CrossrefResolver::with_base_url(
            "invalid\rmailto@example.com",
            "https://api.crossref.org",
        );
        assert!(
            result.is_err(),
            "with_base_url should fail for control characters in mailto"
        );
    }

    // ==================== Resolver Integration Tests (wiremock) ====================

    fn crossref_success_json() -> serde_json::Value {
        serde_json::json!({
            "status": "ok",
            "message": {
                "title": ["A Great Paper"],
                "author": [{"given": "John", "family": "Smith"}],
                "link": [{
                    "URL": "https://publisher.com/paper.pdf",
                    "content-type": "application/pdf",
                    "content-version": "vor",
                    "intended-application": "text-mining"
                }],
                "published": {"date-parts": [[2024, 6, 15]]}
            }
        })
    }

    fn crossref_no_pdf_json() -> serde_json::Value {
        serde_json::json!({
            "status": "ok",
            "message": {
                "title": ["Paper Without PDF Link"],
                "author": [{"given": "Jane", "family": "Doe"}],
                "published": {"date-parts": [[2023]]}
            }
        })
    }

    #[tokio::test]
    async fn test_crossref_resolver_resolve_success_with_pdf() {
        let Some(mock_server) = start_mock_server_or_skip().await else {
            return;
        };

        Mock::given(method("GET"))
            .and(path_regex(r"/works/10\..+"))
            .respond_with(ResponseTemplate::new(200).set_body_json(crossref_success_json()))
            .mount(&mock_server)
            .await;

        let resolver =
            CrossrefResolver::with_base_url("test@example.com", mock_server.uri()).unwrap();
        let ctx = ResolveContext::default();
        let result = resolver.resolve("10.1234/test", &ctx).await.unwrap();

        match result {
            ResolveStep::Url(resolved) => {
                assert_eq!(resolved.url, "https://publisher.com/paper.pdf");
                assert_eq!(resolved.metadata.get("title").unwrap(), "A Great Paper");
                assert_eq!(resolved.metadata.get("authors").unwrap(), "Smith, John");
                assert_eq!(resolved.metadata.get("year").unwrap(), "2024");
                assert_eq!(resolved.metadata.get("doi").unwrap(), "10.1234/test");
            }
            other => panic!("Expected ResolveStep::Url, got: {other:?}"),
        }
    }

    #[tokio::test]
    async fn test_crossref_resolver_resolve_no_pdf_redirects() {
        let Some(mock_server) = start_mock_server_or_skip().await else {
            return;
        };

        Mock::given(method("GET"))
            .and(path_regex(r"/works/10\..+"))
            .respond_with(ResponseTemplate::new(200).set_body_json(crossref_no_pdf_json()))
            .mount(&mock_server)
            .await;

        let resolver =
            CrossrefResolver::with_base_url("test@example.com", mock_server.uri()).unwrap();
        let ctx = ResolveContext::default();
        let result = resolver.resolve("10.5678/no-pdf", &ctx).await.unwrap();

        match result {
            ResolveStep::Redirect(url) => {
                assert_eq!(url, "https://doi.org/10.5678/no-pdf");
            }
            other => panic!("Expected ResolveStep::Redirect, got: {other:?}"),
        }
    }

    #[tokio::test]
    async fn test_crossref_resolver_resolve_404_fails() {
        let Some(mock_server) = start_mock_server_or_skip().await else {
            return;
        };

        Mock::given(method("GET"))
            .and(path_regex(r"/works/10\..+"))
            .respond_with(ResponseTemplate::new(404))
            .mount(&mock_server)
            .await;

        let resolver =
            CrossrefResolver::with_base_url("test@example.com", mock_server.uri()).unwrap();
        let ctx = ResolveContext::default();
        let result = resolver.resolve("10.9999/invalid", &ctx).await.unwrap();

        match result {
            ResolveStep::Failed(err) => {
                let msg = err.to_string();
                assert!(
                    msg.contains("not found"),
                    "Error should mention 'not found': {msg}"
                );
            }
            other => panic!("Expected ResolveStep::Failed, got: {other:?}"),
        }
    }

    #[tokio::test]
    async fn test_crossref_resolver_resolve_429_fails() {
        let Some(mock_server) = start_mock_server_or_skip().await else {
            return;
        };

        Mock::given(method("GET"))
            .and(path_regex(r"/works/10\..+"))
            .respond_with(ResponseTemplate::new(429))
            .mount(&mock_server)
            .await;

        let resolver =
            CrossrefResolver::with_base_url("test@example.com", mock_server.uri()).unwrap();
        let ctx = ResolveContext::default();
        let result = resolver.resolve("10.1234/test", &ctx).await.unwrap();

        match result {
            ResolveStep::Failed(err) => {
                let msg = err.to_string();
                assert!(
                    msg.contains("rate limit"),
                    "Error should mention 'rate limit': {msg}"
                );
            }
            other => panic!("Expected ResolveStep::Failed, got: {other:?}"),
        }
    }

    #[tokio::test]
    async fn test_crossref_resolver_resolve_500_fails() {
        let Some(mock_server) = start_mock_server_or_skip().await else {
            return;
        };

        Mock::given(method("GET"))
            .and(path_regex(r"/works/10\..+"))
            .respond_with(ResponseTemplate::new(503))
            .mount(&mock_server)
            .await;

        let resolver =
            CrossrefResolver::with_base_url("test@example.com", mock_server.uri()).unwrap();
        let ctx = ResolveContext::default();
        let result = resolver.resolve("10.1234/test", &ctx).await.unwrap();

        match result {
            ResolveStep::Failed(err) => {
                let msg = err.to_string();
                assert!(
                    msg.contains("unavailable"),
                    "Error should mention 'unavailable': {msg}"
                );
            }
            other => panic!("Expected ResolveStep::Failed, got: {other:?}"),
        }
    }

    #[tokio::test]
    async fn test_crossref_resolver_resolve_includes_metadata() {
        let Some(mock_server) = start_mock_server_or_skip().await else {
            return;
        };

        Mock::given(method("GET"))
            .and(path_regex(r"/works/10\..+"))
            .respond_with(ResponseTemplate::new(200).set_body_json(crossref_success_json()))
            .mount(&mock_server)
            .await;

        let resolver =
            CrossrefResolver::with_base_url("test@example.com", mock_server.uri()).unwrap();
        let ctx = ResolveContext::default();
        let result = resolver.resolve("10.1234/test", &ctx).await.unwrap();

        if let ResolveStep::Url(resolved) = result {
            assert!(resolved.metadata.contains_key("title"));
            assert!(resolved.metadata.contains_key("authors"));
            assert!(resolved.metadata.contains_key("year"));
            assert!(resolved.metadata.contains_key("doi"));
        } else {
            panic!("Expected ResolveStep::Url");
        }
    }

    #[tokio::test]
    async fn test_crossref_resolver_resolve_malformed_json_fails() {
        let Some(mock_server) = start_mock_server_or_skip().await else {
            return;
        };

        Mock::given(method("GET"))
            .and(path_regex(r"/works/10\..+"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_string(r#"{"invalid": "not crossref format"}"#)
                    .insert_header("content-type", "application/json"),
            )
            .mount(&mock_server)
            .await;

        let resolver =
            CrossrefResolver::with_base_url("test@example.com", mock_server.uri()).unwrap();
        let ctx = ResolveContext::default();
        let result = resolver.resolve("10.1234/test", &ctx).await.unwrap();

        match result {
            ResolveStep::Failed(err) => {
                let msg = err.to_string();
                assert!(
                    msg.contains("Unexpected") || msg.contains("response format"),
                    "Error should mention unexpected format: {msg}"
                );
            }
            other => panic!("Expected ResolveStep::Failed, got: {other:?}"),
        }
    }

    #[tokio::test]
    async fn test_crossref_resolver_sends_mailto_param() {
        let Some(mock_server) = start_mock_server_or_skip().await else {
            return;
        };

        Mock::given(method("GET"))
            .and(path_regex(r"/works/10\..+"))
            .and(query_param("mailto", "test@example.com"))
            .respond_with(ResponseTemplate::new(200).set_body_json(crossref_success_json()))
            .mount(&mock_server)
            .await;

        let resolver =
            CrossrefResolver::with_base_url("test@example.com", mock_server.uri()).unwrap();
        let ctx = ResolveContext::default();

        // If mailto param is missing, wiremock won't match and will return 404
        let result = resolver.resolve("10.1234/test", &ctx).await.unwrap();
        assert!(
            matches!(result, ResolveStep::Url(_)),
            "Should succeed when mailto param is present"
        );
    }

    #[tokio::test]
    async fn test_crossref_resolver_sends_url_encoded_doi_path() {
        let Some(mock_server) = start_mock_server_or_skip().await else {
            return;
        };

        Mock::given(method("GET"))
            .and(path("/works/10.1234%2Ftest.encoded"))
            .respond_with(ResponseTemplate::new(200).set_body_json(crossref_success_json()))
            .mount(&mock_server)
            .await;

        let resolver =
            CrossrefResolver::with_base_url("test@example.com", mock_server.uri()).unwrap();
        let ctx = ResolveContext::default();
        let result = resolver
            .resolve("10.1234/test.encoded", &ctx)
            .await
            .unwrap();
        assert!(
            matches!(result, ResolveStep::Url(_)),
            "Should succeed with URL-encoded DOI path"
        );
    }

    #[tokio::test]
    async fn test_crossref_resolver_sets_shared_user_agent() {
        let Some(mock_server) = start_mock_server_or_skip().await else {
            return;
        };
        let expected_user_agent = standard_user_agent("crossref");
        assert!(
            !expected_user_agent.contains("mailto:"),
            "resolver UA must not contain mailto (polite pool uses query param)"
        );
        assert!(
            expected_user_agent.contains("downloader/"),
            "UA must identify the tool"
        );

        Mock::given(method("GET"))
            .and(path_regex(r"/works/10\..+"))
            .and(header("user-agent", expected_user_agent))
            .respond_with(ResponseTemplate::new(200).set_body_json(crossref_success_json()))
            .mount(&mock_server)
            .await;

        let resolver =
            CrossrefResolver::with_base_url("test@example.com", mock_server.uri()).unwrap();
        let ctx = ResolveContext::default();
        let result = resolver.resolve("10.1234/test", &ctx).await.unwrap();
        assert!(
            matches!(result, ResolveStep::Url(_)),
            "Should send shared User-Agent header"
        );
    }

    #[tokio::test]
    async fn test_crossref_resolver_non_ok_status_fails() {
        let Some(mock_server) = start_mock_server_or_skip().await else {
            return;
        };

        Mock::given(method("GET"))
            .and(path_regex(r"/works/10\..+"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "status": "error",
                "message": {}
            })))
            .mount(&mock_server)
            .await;

        let resolver =
            CrossrefResolver::with_base_url("test@example.com", mock_server.uri()).unwrap();
        let ctx = ResolveContext::default();
        let result = resolver.resolve("10.1234/test", &ctx).await.unwrap();

        match result {
            ResolveStep::Failed(err) => {
                let msg = err.to_string();
                assert!(
                    msg.contains("Unexpected Crossref response status"),
                    "Error should mention unexpected status: {msg}"
                );
            }
            other => panic!("Expected ResolveStep::Failed, got: {other:?}"),
        }
    }
}
