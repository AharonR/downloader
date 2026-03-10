//! Oxford Academic resolver for article URLs and `10.1093/*` DOI inputs.

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
    absolutize_url, auth_requirement, compile_static_regex, extract_year_from_str, hosts_match,
    is_auth_required_status, looks_like_doi, parse_host_or_fallback,
};
use super::{ResolveContext, ResolveError, ResolveStep, ResolvedUrl, Resolver, ResolverPriority};

const DEFAULT_BASE_URL: &str = "https://academic.oup.com";
const DEFAULT_DOI_BASE_URL: &str = "https://doi.org";
const OXFORD_DOI_PREFIX: &str = "10.1093/";

static META_TAG_RE: LazyLock<Regex> =
    LazyLock::new(|| compile_static_regex(r"(?is)<meta\s+[^>]*>"));
static META_ATTR_RE: LazyLock<Regex> = LazyLock::new(|| {
    compile_static_regex(r#"([a-zA-Z_:][-a-zA-Z0-9_:.]*)\s*=\s*(?:"([^"]*)"|'([^']*)')"#)
});
static PDF_LINK_RE: LazyLock<Regex> = LazyLock::new(|| {
    compile_static_regex(
        r#"(?is)href\s*=\s*["']([^"']*(?:/article-pdf/|/advance-article-pdf/)[^"']*)["']"#,
    )
});

/// A site-specific resolver for Oxford Academic article pages.
pub struct OxfordAcademicResolver {
    client: Client,
    base_url: String,
    base_host: String,
    doi_base_url: String,
    doi_base_host: String,
}

impl OxfordAcademicResolver {
    /// Creates a resolver with default Oxford Academic and DOI endpoints.
    ///
    /// # Errors
    ///
    /// Returns [`ResolveError`] if the HTTP client cannot be constructed.
    #[tracing::instrument(skip(cookie_jar), fields(resolver = "oxford"))]
    pub fn new(cookie_jar: Option<Arc<Jar>>) -> Result<Self, ResolveError> {
        Self::with_base_urls(cookie_jar, DEFAULT_BASE_URL, DEFAULT_DOI_BASE_URL)
    }

    /// Creates a resolver with custom endpoints for tests.
    ///
    /// # Errors
    ///
    /// Returns [`ResolveError`] if the HTTP client cannot be constructed.
    #[tracing::instrument(skip(cookie_jar, base_url, doi_base_url), fields(resolver = "oxford"))]
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

impl std::fmt::Debug for OxfordAcademicResolver {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("OxfordAcademicResolver")
            .field("base_url", &self.base_url)
            .field("doi_base_url", &self.doi_base_url)
            .finish_non_exhaustive()
    }
}

#[async_trait]
impl Resolver for OxfordAcademicResolver {
    fn name(&self) -> &'static str {
        "oxford"
    }

    fn priority(&self) -> ResolverPriority {
        ResolverPriority::Specialized
    }

    #[tracing::instrument(level = "trace", skip(self))]
    fn can_handle(&self, input: &str, input_type: InputType) -> bool {
        match input_type {
            InputType::Doi => looks_like_doi(input, OXFORD_DOI_PREFIX),
            InputType::Url => {
                let Ok(url) = Url::parse(input) else {
                    return false;
                };
                let Some(host) = url.host_str() else {
                    return false;
                };

                if hosts_match(host, &self.base_host) {
                    return is_supported_article_path(url.path()) || is_direct_pdf_path(url.path());
                }

                hosts_match(host, &self.doi_base_host)
                    && looks_like_doi(url.path().trim_start_matches('/'), OXFORD_DOI_PREFIX)
            }
            _ => false,
        }
    }

    #[tracing::instrument(skip(self, _ctx), fields(resolver = "oxford", input = %input))]
    async fn resolve(
        &self,
        input: &str,
        _ctx: &ResolveContext,
    ) -> Result<ResolveStep, ResolveError> {
        let request_url = normalize_input_url(input, &self.doi_base_url);

        if let Ok(parsed_url) = Url::parse(&request_url)
            && hosts_match(parsed_url.host_str().unwrap_or_default(), &self.base_host)
            && is_direct_pdf_path(parsed_url.path())
        {
            debug!(
                url = %request_url,
                "URL already appears to be a direct Oxford Academic PDF endpoint"
            );
            return Ok(ResolveStep::Url(ResolvedUrl::new(request_url)));
        }

        debug!(url = %request_url, "Fetching Oxford Academic page for resolution");

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
                warn!(error = %error, "Oxford Academic request failed");
                return Ok(ResolveStep::Failed(ResolveError::resolution_failed(
                    input,
                    "Cannot reach Oxford Academic/DOI endpoint. Check network and try again.",
                )));
            }
        };

        let status = response.status();
        let final_url = response.url().clone();
        let final_host = final_url.host_str().unwrap_or_default().to_string();

        if is_auth_required_status(status.as_u16()) {
            return Ok(ResolveStep::NeedsAuth(auth_requirement(
                &final_host,
                "academic.oup.com",
                "Oxford Academic returned an authorization response. Refresh cookies with `downloader auth capture --save-cookies` and retry.",
            )));
        }

        if !status.is_success() {
            return Ok(ResolveStep::Failed(ResolveError::resolution_failed(
                input,
                &format!("Oxford Academic returned HTTP {}", status.as_u16()),
            )));
        }

        if !hosts_match(&final_host, &self.base_host) {
            return Ok(ResolveStep::Failed(ResolveError::resolution_failed(
                input,
                "Resolved page is not hosted on Oxford Academic",
            )));
        }

        let html = match response.text().await {
            Ok(text) => text,
            Err(error) => {
                warn!(error = %error, "Failed to read Oxford Academic response body");
                return Ok(ResolveStep::Failed(ResolveError::resolution_failed(
                    input,
                    "Unable to parse Oxford Academic response body",
                )));
            }
        };

        if is_auth_page(&html, &final_url) {
            return Ok(ResolveStep::NeedsAuth(auth_requirement(
                &final_host,
                "academic.oup.com",
                "Oxford Academic page appears to require subscription or sign-in. Refresh cookies with `downloader auth capture --save-cookies` and retry.",
            )));
        }

        let meta_tags = collect_meta_tags(&html);
        let Some(pdf_url) = resolve_pdf_url(&meta_tags, &html, &final_url) else {
            return Ok(ResolveStep::Failed(ResolveError::resolution_failed(
                input,
                "Could not identify an Oxford Academic PDF URL from the article page",
            )));
        };

        let mut metadata = extract_metadata(&meta_tags);
        metadata.insert("source_url".to_string(), final_url.as_str().to_string());
        if let Some(doi) = extract_input_doi(input) {
            metadata.entry("doi".to_string()).or_insert(doi);
        }

        Ok(ResolveStep::Url(ResolvedUrl::with_metadata(
            pdf_url, metadata,
        )))
    }
}

fn build_client(cookie_jar: Option<Arc<Jar>>) -> Result<Client, ResolveError> {
    build_resolver_http_client("oxford", standard_user_agent("oxford"), cookie_jar)
}

fn normalize_input_url(input: &str, doi_base_url: &str) -> String {
    let trimmed = input.trim();
    if looks_like_doi(trimmed, OXFORD_DOI_PREFIX) {
        format!("{}/{}", doi_base_url.trim_end_matches('/'), trimmed)
    } else {
        trimmed.to_string()
    }
}

fn is_supported_article_path(path: &str) -> bool {
    let lower = path.to_ascii_lowercase();
    lower.contains("/article/")
        || lower.contains("/article-abstract/")
        || lower.contains("/advance-article/")
        || lower.contains("/advance-article-abstract/")
}

fn is_direct_pdf_path(path: &str) -> bool {
    let lower = path.to_ascii_lowercase();
    lower.contains("/article-pdf/") || lower.contains("/advance-article-pdf/")
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

fn resolve_pdf_url(meta_tags: &[MetaTag], html: &str, final_url: &Url) -> Option<String> {
    first_meta_value(meta_tags, &["citation_pdf_url"])
        .or_else(|| extract_pdf_url_from_links(html))
        .and_then(|value| absolutize_url(&value, final_url))
}

fn extract_pdf_url_from_links(html: &str) -> Option<String> {
    PDF_LINK_RE
        .captures(html)
        .and_then(|caps| caps.get(1).map(|m| html_unescape_basic(m.as_str())))
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
        metadata.insert("doi".to_string(), normalize_doi_value(&doi));
    }

    if let Some(pub_date) = first_meta_value(meta_tags, &["citation_publication_date"])
        && let Some(year) = extract_year_from_str(&pub_date)
    {
        metadata.insert("year".to_string(), year);
    }

    metadata
}

fn normalize_doi_value(value: &str) -> String {
    let trimmed = value.trim();
    let lower = trimmed.to_ascii_lowercase();
    if lower.starts_with("doi:") {
        trimmed[4..].trim().to_string()
    } else {
        trimmed.to_string()
    }
}

fn extract_input_doi(input: &str) -> Option<String> {
    let trimmed = input.trim();
    if looks_like_doi(trimmed, OXFORD_DOI_PREFIX) {
        return Some(trimmed.to_string());
    }

    Url::parse(trimmed)
        .ok()
        .map(|url| url.path().trim_start_matches('/').to_string())
        .filter(|candidate| looks_like_doi(candidate, OXFORD_DOI_PREFIX))
}

fn is_auth_page(html: &str, final_url: &Url) -> bool {
    let path = final_url.path().to_ascii_lowercase();
    if path.contains("/login") || path.contains("/sign-in") {
        return true;
    }

    let normalized = html.to_ascii_lowercase();
    let exact_markers = [
        "you do not currently have access to this article",
        "this pdf is available to subscribers only",
    ];

    if exact_markers
        .iter()
        .any(|marker| normalized.contains(marker))
    {
        return true;
    }

    let markers = [
        "sign in",
        "access through your institution",
        "subscribe",
        "purchase",
        "institutional login",
        "available to subscribers only",
    ];
    let marker_hits = markers
        .iter()
        .filter(|marker| normalized.contains(**marker))
        .count();

    marker_hits >= 2
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
    fn test_oxford_resolver_can_handle_supported_inputs() {
        let resolver = OxfordAcademicResolver::new(None).unwrap();
        assert!(resolver.can_handle(
            "https://academic.oup.com/brain/article/doi/10.1093/brain/awab497/41589581",
            InputType::Url
        ));
        assert!(resolver.can_handle(
            "https://academic.oup.com/brain/article-abstract/doi/10.1093/brain/awab497/41589581",
            InputType::Url
        ));
        assert!(resolver.can_handle(
            "https://academic.oup.com/brain/advance-article/doi/10.1093/brain/awab497/41589581",
            InputType::Url
        ));
        assert!(resolver.can_handle(
            "https://academic.oup.com/brain/advance-article-pdf/doi/10.1093/brain/awab497/41589581/awab497.pdf",
            InputType::Url
        ));
        assert!(resolver.can_handle(
            "https://academic.oup.com/chemse/article-pdf/36/2/NP/854590/bjr004.pdf",
            InputType::Url
        ));
        assert!(resolver.can_handle("https://doi.org/10.1093/chemse/bjag003", InputType::Url));
        assert!(resolver.can_handle("10.1093/chemse/bjag003", InputType::Doi));
    }

    #[test]
    fn test_oxford_resolver_rejects_unrelated_inputs() {
        let resolver = OxfordAcademicResolver::new(None).unwrap();
        assert!(!resolver.can_handle("https://academic.oup.com/brain/issue/149/2", InputType::Url));
        assert!(!resolver.can_handle("https://academic.oup.com/search-results", InputType::Url));
        assert!(!resolver.can_handle("10.1016/j.future.2018.10.001", InputType::Doi));
    }

    #[test]
    fn test_normalize_input_url_from_doi() {
        assert_eq!(
            normalize_input_url("10.1093/chemse/bjag003", DEFAULT_DOI_BASE_URL),
            "https://doi.org/10.1093/chemse/bjag003"
        );
    }

    #[test]
    fn test_is_direct_pdf_path_matches_doi_and_issue_style_urls() {
        assert!(is_direct_pdf_path(
            "/brain/article-pdf/doi/10.1093/brain/awab497/41589581/awab497.pdf"
        ));
        assert!(is_direct_pdf_path(
            "/chemse/article-pdf/36/2/NP/854590/bjr004.pdf"
        ));
        assert!(is_direct_pdf_path(
            "/brain/advance-article-pdf/doi/10.1093/brain/awab497/41589581/awab497.pdf"
        ));
        assert!(!is_direct_pdf_path(
            "/brain/article/doi/10.1093/brain/awab497/41589581"
        ));
    }

    #[test]
    fn test_auth_page_detection_matches_high_signal_phrases() {
        let url = Url::parse("https://academic.oup.com/brain/article/doi/10.1093/x/y").unwrap();
        assert!(is_auth_page(
            "<html>You do not currently have access to this article.</html>",
            &url
        ));
        assert!(is_auth_page(
            "<html>This PDF is available to Subscribers Only</html>",
            &url
        ));
    }

    #[test]
    fn test_auth_page_detection_does_not_flag_get_access_alone() {
        let url = Url::parse("https://academic.oup.com/brain/article/doi/10.1093/x/y").unwrap();
        assert!(!is_auth_page(
            "<html><button>Get access</button></html>",
            &url
        ));
    }

    #[test]
    fn test_extract_metadata_returns_standard_fields() {
        let html = r#"
            <meta name="citation_title" content="Oxford Test Paper">
            <meta name="citation_author" content="Alice Researcher">
            <meta name="citation_author" content="Bob Scientist">
            <meta name="citation_doi" content="doi:10.1093/brain/awab497">
            <meta name="citation_publication_date" content="2025-01-08">
        "#;
        let meta_tags = collect_meta_tags(html);
        let metadata = extract_metadata(&meta_tags);

        assert_eq!(metadata.get("title").unwrap(), "Oxford Test Paper");
        assert_eq!(
            metadata.get("authors").unwrap(),
            "Alice Researcher; Bob Scientist"
        );
        assert_eq!(metadata.get("doi").unwrap(), "10.1093/brain/awab497");
        assert_eq!(metadata.get("year").unwrap(), "2025");
    }

    #[test]
    fn test_resolve_pdf_url_prefers_citation_meta() {
        let html = r#"
            <meta name="citation_pdf_url" content="/brain/article-pdf/doi/10.1093/brain/awab497/41589581/awab497.pdf">
            <a href="/brain/article-pdf/36/2/NP/854590/bjr004.pdf">Download PDF</a>
        "#;
        let meta_tags = collect_meta_tags(html);
        let final_url =
            Url::parse("https://academic.oup.com/brain/article/doi/10.1093/brain/awab497/1")
                .unwrap();

        let resolved = resolve_pdf_url(&meta_tags, html, &final_url).unwrap();
        assert!(resolved.contains("awab497.pdf"));
    }

    #[test]
    fn test_resolve_pdf_url_falls_back_to_pdf_link() {
        let html = r#"
            <a href="/brain/article-pdf/36/2/NP/854590/bjr004.pdf">Download PDF</a>
        "#;
        let meta_tags = collect_meta_tags(html);
        let final_url =
            Url::parse("https://academic.oup.com/brain/article/36/2/NP/854590").unwrap();

        assert_eq!(
            resolve_pdf_url(&meta_tags, html, &final_url).unwrap(),
            "https://academic.oup.com/brain/article-pdf/36/2/NP/854590/bjr004.pdf"
        );
    }
}
