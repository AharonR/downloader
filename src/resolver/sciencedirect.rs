//! `ScienceDirect` resolver for authenticated article-to-PDF URL resolution.
//!
//! This resolver handles common `ScienceDirect` article URL forms plus Elsevier
//! DOIs (`10.1016/...`). It loads the article HTML, extracts the PDF URL and
//! citation metadata, and surfaces auth-expired hints when login pages are
//! returned.

use std::collections::HashMap;
use std::sync::Arc;
use std::sync::LazyLock;

use async_trait::async_trait;
use regex::Regex;
use reqwest::Client;
use reqwest::cookie::Jar;
use reqwest::header::ACCEPT;
use tracing::{debug, warn};
use url::Url;

use crate::parser::InputType;

use super::http_client::{build_resolver_http_client, standard_user_agent};
use super::utils::{
    absolutize_url, auth_requirement, canonical_host, compile_static_regex, extract_year_from_str,
    hosts_match, is_auth_required_status, looks_like_doi, parse_host_or_fallback,
};
use super::{ResolveContext, ResolveError, ResolveStep, ResolvedUrl, Resolver, ResolverPriority};

const DEFAULT_BASE_URL: &str = "https://www.sciencedirect.com";
const DEFAULT_DOI_BASE_URL: &str = "https://doi.org";
const SCIENCE_DIRECT_DOI_PREFIX: &str = "10.1016/";
const ADDITIONAL_ELSEVIER_HOSTS: &[&str] = &["linkinghub.elsevier.com"];

static META_TAG_RE: LazyLock<Regex> =
    LazyLock::new(|| compile_static_regex(r"(?is)<meta\s+[^>]*>"));
static META_ATTR_RE: LazyLock<Regex> = LazyLock::new(|| {
    compile_static_regex(r#"([a-zA-Z_:][-a-zA-Z0-9_:.]*)\s*=\s*(?:"([^"]*)"|'([^']*)')"#)
});
static JSON_PDF_URL_RE: LazyLock<Regex> = LazyLock::new(|| {
    compile_static_regex(r#""(?:pdfUrl|pdfDownloadUrl|linkToPdf)"\s*:\s*"([^"]+)""#)
});
static PII_PATH_RE: LazyLock<Regex> = LazyLock::new(|| {
    compile_static_regex(r"(?i)/(?:science/article/(?:abs/|am/)?pii|pii)/([A-Z0-9]{8,32})")
});

/// A site-specific resolver for `ScienceDirect`.
///
/// The resolver can use a cookie jar so authenticated institutional sessions are
/// applied while loading article pages.
pub struct ScienceDirectResolver {
    client: Client,
    base_url: String,
    base_host: String,
    doi_base_url: String,
    doi_base_host: String,
}

impl ScienceDirectResolver {
    /// Create a resolver with default ScienceDirect/DOI endpoints.
    ///
    /// # Errors
    ///
    /// Returns `ResolveError` if the HTTP client cannot be constructed.
    #[tracing::instrument(skip(cookie_jar), fields(resolver = "sciencedirect"))]
    pub fn new(cookie_jar: Option<Arc<Jar>>) -> Result<Self, ResolveError> {
        Self::with_base_urls(cookie_jar, DEFAULT_BASE_URL, DEFAULT_DOI_BASE_URL)
    }

    /// Create a resolver with custom endpoints (used by integration tests).
    ///
    /// # Errors
    ///
    /// Returns `ResolveError` if the HTTP client cannot be constructed.
    #[tracing::instrument(
        skip(cookie_jar, base_url, doi_base_url),
        fields(resolver = "sciencedirect")
    )]
    pub fn with_base_urls(
        cookie_jar: Option<Arc<Jar>>,
        base_url: impl Into<String>,
        doi_base_url: impl Into<String>,
    ) -> Result<Self, ResolveError> {
        let base_url = base_url.into();
        let doi_base_url = doi_base_url.into();

        Ok(Self {
            client: build_client(cookie_jar)?,
            base_host: parse_host_or_fallback(&base_url),
            doi_base_host: parse_host_or_fallback(&doi_base_url),
            base_url,
            doi_base_url,
        })
    }
}

impl std::fmt::Debug for ScienceDirectResolver {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ScienceDirectResolver")
            .field("base_url", &self.base_url)
            .field("doi_base_url", &self.doi_base_url)
            .finish_non_exhaustive()
    }
}

#[async_trait]
impl Resolver for ScienceDirectResolver {
    fn name(&self) -> &'static str {
        "sciencedirect"
    }

    fn priority(&self) -> ResolverPriority {
        ResolverPriority::Specialized
    }

    #[tracing::instrument(level = "trace", skip(self))]
    fn can_handle(&self, input: &str, input_type: InputType) -> bool {
        match input_type {
            InputType::Url => {
                let Ok(url) = Url::parse(input) else {
                    return false;
                };
                let Some(host) = url.host_str() else {
                    return false;
                };

                if hosts_match(host, &self.base_host) {
                    // Let direct download URLs go straight to fallback resolver.
                    // This avoids downloading large PDF bodies during "resolution".
                    if is_probably_direct_pdf_path(url.path()) {
                        return false;
                    }

                    return is_probable_article_path(url.path());
                }

                hosts_match(host, &self.doi_base_host)
                    && looks_like_doi(
                        url.path().trim_start_matches('/'),
                        SCIENCE_DIRECT_DOI_PREFIX,
                    )
            }
            InputType::Doi => looks_like_doi(input, SCIENCE_DIRECT_DOI_PREFIX),
            _ => false,
        }
    }

    #[tracing::instrument(skip(self, _ctx), fields(resolver = "sciencedirect", input = %input))]
    async fn resolve(
        &self,
        input: &str,
        _ctx: &ResolveContext,
    ) -> Result<ResolveStep, ResolveError> {
        let request_url = normalize_input_url(input, &self.doi_base_url);

        if let Ok(parsed_url) = Url::parse(&request_url)
            && hosts_match(parsed_url.host_str().unwrap_or_default(), &self.base_host)
            && is_probably_direct_pdf_path(parsed_url.path())
        {
            debug!(
                url = %request_url,
                "URL already appears to be a direct ScienceDirect PDF endpoint"
            );
            return Ok(ResolveStep::Url(ResolvedUrl::new(request_url)));
        }

        debug!(url = %request_url, "Fetching ScienceDirect page for resolution");

        let response = match self
            .client
            .get(&request_url)
            .header(
                ACCEPT,
                "text/html,application/xhtml+xml,application/xml;q=0.9,*/*;q=0.8",
            )
            .send()
            .await
        {
            Ok(resp) => resp,
            Err(error) => {
                warn!(error = %error, "ScienceDirect request failed");
                return Ok(ResolveStep::Failed(ResolveError::resolution_failed(
                    input,
                    "Cannot reach ScienceDirect/DOI endpoint. Check network and try again.",
                )));
            }
        };

        let status = response.status();
        let final_url = response.url().clone();
        let final_host = final_url.host_str().unwrap_or_default().to_string();

        if is_auth_required_status(status.as_u16()) {
            return Ok(ResolveStep::NeedsAuth(auth_requirement(
                &final_host,
                "sciencedirect.com",
                "ScienceDirect returned an authorization response. Your session may be expired. Refresh cookies with `downloader auth capture --save-cookies` and retry.",
            )));
        }

        if !status.is_success() {
            return Ok(ResolveStep::Failed(ResolveError::resolution_failed(
                input,
                &format!("ScienceDirect returned HTTP {}", status.as_u16()),
            )));
        }

        if !is_accepted_final_host(&final_host, &self.base_host) {
            return Ok(ResolveStep::Failed(ResolveError::resolution_failed(
                input,
                "Resolved page is not hosted on ScienceDirect",
            )));
        }

        let html = match response.text().await {
            Ok(text) => text,
            Err(error) => {
                warn!(error = %error, "Failed to read ScienceDirect response body");
                return Ok(ResolveStep::Failed(ResolveError::resolution_failed(
                    input,
                    "Unable to parse ScienceDirect response body",
                )));
            }
        };

        if is_auth_page(&html, &final_url) {
            return Ok(ResolveStep::NeedsAuth(auth_requirement(
                &final_host,
                "sciencedirect.com",
                "ScienceDirect returned a login page. Session appears expired. Refresh cookies with `downloader auth capture --save-cookies` and retry.",
            )));
        }

        let meta_tags = collect_meta_tags(&html);
        let pdf_url = resolve_pdf_url(&meta_tags, &html, &final_url, &self.base_url);
        let Some(pdf_url) = pdf_url else {
            return Ok(ResolveStep::Failed(ResolveError::resolution_failed(
                input,
                "Could not identify a ScienceDirect PDF URL from the article page",
            )));
        };

        let mut metadata = extract_metadata(&meta_tags);
        metadata.insert("source_url".to_string(), final_url.as_str().to_string());
        if looks_like_doi(input, SCIENCE_DIRECT_DOI_PREFIX) {
            metadata
                .entry("doi".to_string())
                .or_insert_with(|| input.trim().to_string());
        }
        if let Some(pii) = extract_pii_from_text(final_url.as_str()) {
            metadata.entry("pii".to_string()).or_insert(pii);
        }

        Ok(ResolveStep::Url(ResolvedUrl::with_metadata(
            pdf_url, metadata,
        )))
    }
}

fn build_client(cookie_jar: Option<Arc<Jar>>) -> Result<Client, ResolveError> {
    build_resolver_http_client(
        "sciencedirect",
        standard_user_agent("sciencedirect"),
        cookie_jar,
    )
}

fn is_accepted_final_host(host: &str, base_host: &str) -> bool {
    if hosts_match(host, base_host) {
        return true;
    }
    let canonical = canonical_host(host);
    ADDITIONAL_ELSEVIER_HOSTS
        .iter()
        .any(|known| canonical == *known)
}

fn is_probable_article_path(path: &str) -> bool {
    path.starts_with("/science/article/") || extract_pii_from_text(path).is_some()
}

fn is_probably_direct_pdf_path(path: &str) -> bool {
    let has_pdf_extension = std::path::Path::new(path)
        .extension()
        .and_then(|ext| ext.to_str())
        .is_some_and(|ext| ext.eq_ignore_ascii_case("pdf"));

    let lower = path.to_ascii_lowercase();
    has_pdf_extension
        || lower.contains("/pdfft")
        || lower.ends_with("/pdf")
        || lower.contains("/downloadpdf")
}

fn normalize_input_url(input: &str, doi_base_url: &str) -> String {
    let trimmed = input.trim();
    if looks_like_doi(trimmed, SCIENCE_DIRECT_DOI_PREFIX) {
        format!("{}/{}", doi_base_url.trim_end_matches('/'), trimmed)
    } else {
        trimmed.to_string()
    }
}

#[derive(Debug, Clone)]
struct MetaTag {
    name: String,
    content: String,
}

fn collect_meta_tags(html: &str) -> Vec<MetaTag> {
    let mut tags = Vec::new();

    for tag_match in META_TAG_RE.find_iter(html) {
        let mut tag_name: Option<String> = None;
        let mut content: Option<String> = None;

        for attr in META_ATTR_RE.captures_iter(tag_match.as_str()) {
            let key = attr
                .get(1)
                .map_or("", |m| m.as_str())
                .trim()
                .to_ascii_lowercase();
            let value = attr
                .get(2)
                .or_else(|| attr.get(3))
                .map_or("", |m| m.as_str())
                .trim()
                .to_string();

            if value.is_empty() {
                continue;
            }

            if key == "name" || key == "property" {
                tag_name = Some(value.to_ascii_lowercase());
            } else if key == "content" {
                content = Some(value);
            }
        }

        if let (Some(name), Some(content)) = (tag_name, content) {
            tags.push(MetaTag { name, content });
        }
    }

    tags
}

fn resolve_pdf_url(
    meta_tags: &[MetaTag],
    html: &str,
    final_url: &Url,
    base_url: &str,
) -> Option<String> {
    first_meta_value(meta_tags, &["citation_pdf_url", "pdf_url"])
        .or_else(|| extract_pdf_url_from_json(html))
        .or_else(|| {
            extract_pii_from_text(final_url.as_str()).map(|pii| {
                format!(
                    "{}/science/article/pii/{pii}/pdfft?isDTMRedir=true&download=true",
                    base_url.trim_end_matches('/')
                )
            })
        })
        .and_then(|value| absolutize_url(&value, final_url))
}

fn extract_pdf_url_from_json(html: &str) -> Option<String> {
    JSON_PDF_URL_RE
        .captures(html)
        .and_then(|captures| captures.get(1).map(|m| decode_json_url_field(m.as_str())))
}

fn decode_json_url_field(value: &str) -> String {
    value.replace(r"\u002F", "/").replace(r"\/", "/")
}

fn first_meta_value(meta_tags: &[MetaTag], keys: &[&str]) -> Option<String> {
    meta_tags.iter().find_map(|tag| {
        keys.iter()
            .any(|key| tag.name.eq_ignore_ascii_case(key))
            .then(|| html_unescape_basic(&tag.content))
    })
}

fn all_meta_values(meta_tags: &[MetaTag], keys: &[&str]) -> Vec<String> {
    let mut values = Vec::new();
    for tag in meta_tags {
        if keys.iter().any(|key| tag.name.eq_ignore_ascii_case(key)) {
            let value = html_unescape_basic(&tag.content);
            if !value.is_empty() && !values.contains(&value) {
                values.push(value);
            }
        }
    }
    values
}

fn extract_metadata(meta_tags: &[MetaTag]) -> HashMap<String, String> {
    let mut metadata = HashMap::new();

    if let Some(title) = first_meta_value(meta_tags, &["citation_title", "dc.title"]) {
        metadata.insert("title".to_string(), title);
    }

    let authors = all_meta_values(meta_tags, &["citation_author"]);
    if !authors.is_empty() {
        metadata.insert("authors".to_string(), authors.join("; "));
    }

    if let Some(doi) = first_meta_value(meta_tags, &["citation_doi", "dc.identifier"]) {
        metadata.insert("doi".to_string(), doi);
    }

    if let Some(journal) = first_meta_value(meta_tags, &["citation_journal_title"]) {
        metadata.insert("journal".to_string(), journal);
    }

    if let Some(pub_date) = first_meta_value(meta_tags, &["citation_publication_date"])
        && let Some(year) = extract_year_from_str(&pub_date)
    {
        metadata.insert("year".to_string(), year);
    }

    metadata
}

fn extract_pii_from_text(value: &str) -> Option<String> {
    PII_PATH_RE
        .captures(value)
        .and_then(|captures| captures.get(1).map(|m| m.as_str().to_string()))
}

fn is_auth_page(html: &str, final_url: &Url) -> bool {
    if final_url.path().contains("/user/login") {
        return true;
    }

    let normalized = html.to_ascii_lowercase();

    if normalized.contains("id.elsevier.com") {
        return true;
    }

    let markers = [
        "sign in",
        "institutional access",
        "access through your institution",
        "single sign-on",
        "shibboleth",
    ];
    let marker_hits = markers
        .iter()
        .filter(|marker| normalized.contains(**marker))
        .count();

    marker_hits >= 3
}

fn html_unescape_basic(value: &str) -> String {
    value
        .replace("&amp;", "&")
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&ndash;", "\u{2013}")
        .replace("&mdash;", "\u{2014}")
        .replace("&nbsp;", "\u{00a0}")
        .replace("&#8211;", "\u{2013}")
        .replace("&#8212;", "\u{2014}")
        .replace("&#160;", "\u{00a0}")
        .trim()
        .to_string()
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn test_sciencedirect_resolver_can_handle_common_url_patterns() {
        let resolver = ScienceDirectResolver::new(None).unwrap();
        assert!(resolver.can_handle(
            "https://www.sciencedirect.com/science/article/pii/S0167739X18313560",
            InputType::Url
        ));
        assert!(resolver.can_handle(
            "https://www.sciencedirect.com/science/article/abs/pii/S0167739X18313560",
            InputType::Url
        ));
        assert!(!resolver.can_handle("https://example.com/article", InputType::Url));
    }

    #[test]
    fn test_sciencedirect_resolver_does_not_handle_direct_pdf_paths() {
        let resolver = ScienceDirectResolver::new(None).unwrap();
        assert!(!resolver.can_handle(
            "https://www.sciencedirect.com/science/article/pii/S0167739X18313560/pdfft?isDTMRedir=true&download=true",
            InputType::Url
        ));
        assert!(!resolver.can_handle(
            "https://www.sciencedirect.com/science/article/pii/S0167739X18313560/pdfft.pdf",
            InputType::Url
        ));
    }

    #[test]
    fn test_sciencedirect_resolver_can_handle_elsevier_doi() {
        let resolver = ScienceDirectResolver::new(None).unwrap();
        assert!(resolver.can_handle("10.1016/j.future.2018.10.001", InputType::Doi));
        assert!(!resolver.can_handle("10.1145/9999999.9999999", InputType::Doi));
    }

    #[test]
    fn test_extract_pii_from_supported_paths() {
        assert_eq!(
            extract_pii_from_text(
                "https://www.sciencedirect.com/science/article/pii/S0167739X18313560"
            )
            .as_deref(),
            Some("S0167739X18313560")
        );
        assert_eq!(
            extract_pii_from_text(
                "https://www.sciencedirect.com/science/article/abs/pii/S0925231223001111"
            )
            .as_deref(),
            Some("S0925231223001111")
        );
        assert!(extract_pii_from_text("https://example.com/no-pii").is_none());
    }

    #[test]
    fn test_collect_meta_tags_extracts_name_content_pairs() {
        let html = r#"
            <meta name="citation_title" content="Test Title">
            <meta property="citation_author" content="Alice Researcher">
            <meta content="10.1016/test" name="citation_doi">
        "#;
        let tags = collect_meta_tags(html);
        assert_eq!(tags.len(), 3);
        assert_eq!(
            first_meta_value(&tags, &["citation_title"]).unwrap(),
            "Test Title"
        );
        assert_eq!(
            first_meta_value(&tags, &["citation_doi"]).unwrap(),
            "10.1016/test"
        );
    }

    #[test]
    fn test_is_auth_page_detects_login_markers() {
        let url = Url::parse("https://www.sciencedirect.com/science/article/pii/S0167739X18313560")
            .unwrap();
        let html = "<html><body>id.elsevier.com Sign in</body></html>";
        assert!(is_auth_page(html, &url));
    }

    #[test]
    fn test_resolve_pdf_url_prefers_meta_tag() {
        let html = r#"
            <meta name="citation_pdf_url" content="/science/article/pii/S0167739X18313560/pdfft?isDTMRedir=true&download=true">
        "#;
        let tags = collect_meta_tags(html);
        let final_url =
            Url::parse("https://www.sciencedirect.com/science/article/pii/S0167739X18313560")
                .unwrap();
        let resolved = resolve_pdf_url(&tags, html, &final_url, DEFAULT_BASE_URL).unwrap();
        assert!(resolved.contains("pdfft"));
    }

    #[test]
    fn test_html_unescape_basic_handles_common_entities() {
        assert_eq!(html_unescape_basic("&amp;"), "&");
        assert_eq!(html_unescape_basic("&lt;test&gt;"), "<test>");
        assert_eq!(html_unescape_basic("&quot;quoted&quot;"), "\"quoted\"");
        assert_eq!(html_unescape_basic("it&#39;s"), "it's");
        assert_eq!(html_unescape_basic("no entities here"), "no entities here");
    }

    #[test]
    fn test_html_unescape_basic_handles_typographic_entities() {
        assert_eq!(html_unescape_basic("a&ndash;b"), "a\u{2013}b");
        assert_eq!(html_unescape_basic("a&mdash;b"), "a\u{2014}b");
        assert_eq!(html_unescape_basic("a&nbsp;b"), "a\u{00a0}b");
        assert_eq!(html_unescape_basic("a&#8211;b"), "a\u{2013}b");
        assert_eq!(html_unescape_basic("a&#8212;b"), "a\u{2014}b");
        assert_eq!(html_unescape_basic("a&#160;b"), "a\u{00a0}b");
    }

    #[test]
    fn test_is_auth_page_requires_three_text_markers() {
        let url = Url::parse("https://www.sciencedirect.com/science/article/pii/S0167739X18313560")
            .unwrap();
        // Two markers should NOT trigger auth detection (reduced false-positive risk)
        let html_two = "<html><body>Sign in for institutional access</body></html>";
        assert!(!is_auth_page(html_two, &url));
        // Three markers should trigger auth detection
        let html_three =
            "<html><body>Sign in for institutional access via single sign-on</body></html>";
        assert!(is_auth_page(html_three, &url));
    }

    #[test]
    fn test_collect_meta_tags_handles_single_quoted_attributes() {
        let html = "<meta name='citation_title' content='Single Quoted Title'>";
        let tags = collect_meta_tags(html);
        assert_eq!(tags.len(), 1);
        assert_eq!(
            first_meta_value(&tags, &["citation_title"]).unwrap(),
            "Single Quoted Title"
        );
    }

    #[test]
    fn test_is_accepted_final_host_matches_base_and_elsevier() {
        assert!(is_accepted_final_host(
            "www.sciencedirect.com",
            "sciencedirect.com"
        ));
        assert!(is_accepted_final_host(
            "linkinghub.elsevier.com",
            "sciencedirect.com"
        ));
        assert!(!is_accepted_final_host("example.com", "sciencedirect.com"));
    }

    // ==================== Regression Tests for Code Review Issues ====================

    #[test]
    fn regression_build_client_returns_result_not_panic() {
        // M1: Verify build_client returns Result instead of panicking
        // This ensures library code doesn't panic on HTTP client construction errors
        let result = build_client(None);
        assert!(
            result.is_ok(),
            "build_client should return Ok(Client) for valid construction"
        );
    }

    #[test]
    fn regression_constructor_propagates_build_errors() {
        // M1: Verify ScienceDirectResolver::new returns Result
        // Previously used panic!() which violates library code guidelines
        let result = ScienceDirectResolver::new(None);
        assert!(
            result.is_ok(),
            "ScienceDirectResolver::new should return Result, not panic"
        );
    }

    #[test]
    fn regression_html_unescape_handles_academic_typography() {
        // M2: Verify academic paper metadata with em-dash, en-dash, nbsp is unescaped correctly
        // Common in academic titles like "Theory—Practice Gap" or "A Study – An Analysis"
        let title_with_mdash = "Machine Learning&mdash;A Comprehensive Review";
        assert_eq!(
            html_unescape_basic(title_with_mdash),
            "Machine Learning\u{2014}A Comprehensive Review"
        );

        let title_with_ndash = "COVID-19 &ndash; Impact on Healthcare";
        assert_eq!(
            html_unescape_basic(title_with_ndash),
            "COVID-19 \u{2013} Impact on Healthcare"
        );

        let title_with_nbsp = "Quantum&nbsp;Computing Applications";
        assert_eq!(
            html_unescape_basic(title_with_nbsp),
            "Quantum\u{00a0}Computing Applications"
        );

        // Test numeric entity forms
        let title_numeric = "Deep Learning&#8212;Past&#8211;Present&#160;Future";
        assert_eq!(
            html_unescape_basic(title_numeric),
            "Deep Learning\u{2014}Past\u{2013}Present\u{00a0}Future"
        );
    }

    #[test]
    fn regression_auth_page_threshold_prevents_false_positives() {
        // M4: Verify threshold of 3 markers prevents false positives
        // Papers ABOUT authentication/login should not be flagged as auth pages
        let url = Url::parse("https://www.sciencedirect.com/science/article/pii/TEST123").unwrap();

        // Paper about "Single Sign-On Systems" - 2 markers, should NOT flag
        let paper_about_auth =
            "<html><body><h1>Single Sign-On Authentication Systems</h1></body></html>";
        assert!(
            !is_auth_page(paper_about_auth, &url),
            "Papers about authentication topics should not trigger auth detection with only 2 markers"
        );

        // Real auth page with 3+ markers - should flag
        // Contains: "sign in" + "institutional access" + "single sign-on" = 3 markers
        let real_auth_page = "<html><body>Please sign in to gain institutional access. Use your single sign-on credentials.</body></html>";
        assert!(
            is_auth_page(real_auth_page, &url),
            "Real auth pages with 3+ markers should be detected"
        );
    }

    #[test]
    fn regression_linkinghub_elsevier_accepted_as_valid_host() {
        // M5: Verify linkinghub.elsevier.com is accepted for DOI redirects
        // Some Elsevier DOIs redirect through linkinghub before reaching ScienceDirect
        assert!(
            is_accepted_final_host("linkinghub.elsevier.com", "sciencedirect.com"),
            "linkinghub.elsevier.com should be accepted as valid Elsevier redirect host"
        );

        assert!(
            is_accepted_final_host("www.linkinghub.elsevier.com", "sciencedirect.com"),
            "www.linkinghub.elsevier.com should be accepted (with www prefix)"
        );
    }

    #[test]
    fn regression_html_unescape_handles_mixed_entities() {
        // M2: Verify complex titles with multiple entity types are handled correctly
        let complex_title =
            "&quot;AI &amp; ML&mdash;Theory &ndash; Practice&quot; by Author&#160;Name";
        let expected = "\"AI & ML\u{2014}Theory \u{2013} Practice\" by Author\u{00a0}Name";
        assert_eq!(html_unescape_basic(complex_title), expected);
    }
}
