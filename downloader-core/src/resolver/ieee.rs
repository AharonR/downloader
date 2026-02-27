//! IEEE Xplore resolver for document URLs and `10.1109/*` DOI signals.

use std::collections::HashMap;
use std::sync::Arc;
use std::sync::LazyLock;

use async_trait::async_trait;
use regex::Regex;
use reqwest::Client;
use reqwest::cookie::Jar;
use reqwest::header::ACCEPT;
use url::Url;

use crate::parser::InputType;

use super::http_client::{build_resolver_http_client, standard_user_agent};
use super::utils::{
    CITATION_PDF_RE, absolutize_url, auth_requirement, compile_static_regex, extract_meta_value,
    extract_year_from_str, hosts_match, is_auth_required_status, looks_like_doi,
    parse_host_or_fallback,
};
use super::{ResolveContext, ResolveError, ResolveStep, ResolvedUrl, Resolver, ResolverPriority};

const DEFAULT_BASE_URL: &str = "https://ieeexplore.ieee.org";
const DEFAULT_DOI_BASE_URL: &str = "https://doi.org";
const IEEE_DOI_PREFIX: &str = "10.1109/";

static DOCUMENT_ID_RE: LazyLock<Regex> = LazyLock::new(|| compile_static_regex(r"/document/(\d+)"));
static STAMP_URL_RE: LazyLock<Regex> =
    LazyLock::new(|| compile_static_regex(r#"(?i)(/stamp/stamp\.jsp\?[^"']*arnumber=\d+[^"']*)"#));
static TITLE_RE: LazyLock<Regex> = LazyLock::new(|| {
    compile_static_regex(
        r#"(?is)<meta\s+[^>]*(?:name|property)\s*=\s*["']citation_title["'][^>]*content\s*=\s*["']([^"']+)["']"#,
    )
});
static DOI_RE: LazyLock<Regex> = LazyLock::new(|| {
    compile_static_regex(
        r#"(?is)<meta\s+[^>]*(?:name|property)\s*=\s*["']citation_doi["'][^>]*content\s*=\s*["']([^"']+)["']"#,
    )
});
static YEAR_RE: LazyLock<Regex> = LazyLock::new(|| {
    compile_static_regex(
        r#"(?is)<meta\s+[^>]*(?:name|property)\s*=\s*["']citation_publication_date["'][^>]*content\s*=\s*["']([^"']+)["']"#,
    )
});

/// Specialized resolver for IEEE URLs and DOI patterns.
pub struct IeeeResolver {
    client: Client,
    base_url: String,
    doi_base_url: String,
    base_host: String,
    doi_base_host: String,
}

impl IeeeResolver {
    /// Creates a resolver with default IEEE and DOI hosts.
    ///
    /// # Errors
    ///
    /// Returns [`ResolveError`] if client construction fails.
    pub fn new(cookie_jar: Option<Arc<Jar>>) -> Result<Self, ResolveError> {
        Self::with_base_urls(cookie_jar, DEFAULT_BASE_URL, DEFAULT_DOI_BASE_URL)
    }

    /// Creates a resolver with custom hosts (for tests).
    ///
    /// # Errors
    ///
    /// Returns [`ResolveError`] if client construction fails.
    pub fn with_base_urls(
        cookie_jar: Option<Arc<Jar>>,
        base_url: impl Into<String>,
        doi_base_url: impl Into<String>,
    ) -> Result<Self, ResolveError> {
        let base_url = base_url.into();
        let doi_base_url = doi_base_url.into();

        Ok(Self {
            client: build_resolver_http_client("ieee", standard_user_agent("ieee"), cookie_jar)?,
            base_host: parse_host_or_fallback(&base_url),
            doi_base_host: parse_host_or_fallback(&doi_base_url),
            base_url,
            doi_base_url,
        })
    }
}

impl std::fmt::Debug for IeeeResolver {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("IeeeResolver")
            .field("base_url", &self.base_url)
            .field("doi_base_url", &self.doi_base_url)
            .finish_non_exhaustive()
    }
}

#[async_trait]
impl Resolver for IeeeResolver {
    fn name(&self) -> &'static str {
        "ieee"
    }

    fn priority(&self) -> ResolverPriority {
        ResolverPriority::Specialized
    }

    fn can_handle(&self, input: &str, input_type: InputType) -> bool {
        match input_type {
            InputType::Doi => looks_like_doi(input, IEEE_DOI_PREFIX),
            InputType::Url => {
                let Ok(url) = Url::parse(input) else {
                    return false;
                };
                let Some(host) = url.host_str() else {
                    return false;
                };
                if hosts_match(host, &self.base_host) {
                    return path_looks_like_ieee_resource(url.path()) || has_arnumber_query(&url);
                }
                hosts_match(host, &self.doi_base_host)
                    && looks_like_doi(url.path().trim_start_matches('/'), IEEE_DOI_PREFIX)
            }
            _ => false,
        }
    }

    #[tracing::instrument(skip(self, _ctx), fields(resolver = "ieee", input = %input))]
    async fn resolve(
        &self,
        input: &str,
        _ctx: &ResolveContext,
    ) -> Result<ResolveStep, ResolveError> {
        let request_url = normalize_input_url(input, &self.doi_base_url);
        if is_direct_stamp_url(&request_url) {
            return Ok(ResolveStep::Url(ResolvedUrl::new(request_url)));
        }

        let Ok(response) = self
            .client
            .get(&request_url)
            .header(
                ACCEPT,
                "text/html,application/xhtml+xml,application/xml;q=0.9,*/*;q=0.8",
            )
            .send()
            .await
        else {
            return Ok(ResolveStep::Failed(ResolveError::resolution_failed(
                input,
                "Unable to reach IEEE Xplore or DOI endpoint",
            )));
        };

        let status = response.status();
        if is_auth_required_status(status.as_u16()) {
            return Ok(ResolveStep::NeedsAuth(auth_requirement(
                "ieeexplore.ieee.org",
                "ieeexplore.ieee.org",
                "IEEE returned an authorization response. Provide authenticated cookies (`--cookies` or `auth capture`) and retry.",
            )));
        }
        if !status.is_success() {
            return Ok(ResolveStep::Failed(ResolveError::resolution_failed(
                input,
                &format!("IEEE returned HTTP {}", status.as_u16()),
            )));
        }

        let final_url = response.url().clone();
        let Ok(html) = response.text().await else {
            return Ok(ResolveStep::Failed(ResolveError::resolution_failed(
                input,
                "IEEE response body could not be parsed",
            )));
        };

        if is_auth_page(&html, &final_url) {
            return Ok(ResolveStep::NeedsAuth(auth_requirement(
                final_url.host_str().unwrap_or(""),
                "ieeexplore.ieee.org",
                "IEEE page appears to be paywalled or requires sign-in. Provide authenticated cookies (`--cookies` or `auth capture`) and retry.",
            )));
        }

        let arnumber = extract_document_id(final_url.path()).or_else(|| extract_arnumber(&html));
        let pdf_url = extract_pdf_url(&html, &final_url).or_else(|| {
            arnumber.as_ref().map(|id| {
                format!(
                    "{}/stamp/stamp.jsp?tp=&arnumber={id}",
                    self.base_url.trim_end_matches('/')
                )
            })
        });

        let Some(pdf_url) = pdf_url else {
            return Ok(ResolveStep::Failed(ResolveError::resolution_failed(
                input,
                "No IEEE PDF target could be identified from the document page",
            )));
        };

        let mut metadata = HashMap::new();
        metadata.insert("source_url".to_string(), final_url.as_str().to_string());
        if let Some(id) = arnumber {
            metadata.insert("ieee_arnumber".to_string(), id);
        }
        if let Some(title) = extract_meta_value(&html, &TITLE_RE) {
            metadata.insert("title".to_string(), title);
        }
        if let Some(doi) = extract_meta_value(&html, &DOI_RE).or_else(|| extract_input_doi(input)) {
            metadata.insert("doi".to_string(), doi);
        }
        if let Some(raw_date) = extract_meta_value(&html, &YEAR_RE)
            && let Some(year) = extract_year_from_str(&raw_date)
        {
            metadata.insert("year".to_string(), year);
        }

        Ok(ResolveStep::Url(ResolvedUrl::with_metadata(
            pdf_url, metadata,
        )))
    }
}

fn normalize_input_url(input: &str, doi_base_url: &str) -> String {
    let trimmed = input.trim();
    if looks_like_doi(trimmed, IEEE_DOI_PREFIX) {
        format!("{}/{}", doi_base_url.trim_end_matches('/'), trimmed)
    } else {
        trimmed.to_string()
    }
}

fn path_looks_like_ieee_resource(path: &str) -> bool {
    path.contains("/document/") || path.contains("/stamp/stamp.jsp")
}

fn has_arnumber_query(url: &Url) -> bool {
    url.query()
        .is_some_and(|query| query.to_ascii_lowercase().contains("arnumber="))
}

fn is_direct_stamp_url(value: &str) -> bool {
    Url::parse(value)
        .ok()
        .is_some_and(|url| path_looks_like_ieee_resource(url.path()) && has_arnumber_query(&url))
}

fn extract_document_id(path: &str) -> Option<String> {
    DOCUMENT_ID_RE
        .captures(path)
        .and_then(|caps| caps.get(1).map(|m| m.as_str().to_string()))
}

fn extract_arnumber(html: &str) -> Option<String> {
    STAMP_URL_RE.captures(html).and_then(|caps| {
        caps.get(1)
            .and_then(|m| Url::parse(&format!("https://ieeexplore.ieee.org{}", m.as_str())).ok())
            .and_then(|url| {
                url.query_pairs()
                    .find(|(k, _)| k.eq_ignore_ascii_case("arnumber"))
                    .map(|(_, v)| v.to_string())
            })
    })
}

fn extract_pdf_url(html: &str, base_url: &Url) -> Option<String> {
    extract_meta_value(html, &CITATION_PDF_RE)
        .or_else(|| {
            STAMP_URL_RE
                .captures(html)
                .and_then(|caps| caps.get(1).map(|m| m.as_str().to_string()))
        })
        .and_then(|value| absolutize_url(&value, base_url))
}

fn extract_input_doi(input: &str) -> Option<String> {
    let trimmed = input.trim();
    if looks_like_doi(trimmed, IEEE_DOI_PREFIX) {
        return Some(trimmed.to_string());
    }
    Url::parse(trimmed)
        .ok()
        .map(|url| url.path().trim_start_matches('/').to_string())
        .filter(|candidate| looks_like_doi(candidate, IEEE_DOI_PREFIX))
}

fn is_auth_page(html: &str, final_url: &Url) -> bool {
    let path = final_url.path().to_ascii_lowercase();
    if path.contains("/login") || path.contains("/servlet/login") {
        return true;
    }

    let normalized = html.to_ascii_lowercase();
    let markers = [
        "sign in",
        "institutional sign in",
        "access through your institution",
        "purchase pdf",
        "subscribe to ieee xplore",
    ];

    let marker_hits = markers
        .iter()
        .filter(|marker| normalized.contains(**marker))
        .count();

    marker_hits >= 2
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn test_ieee_can_handle_document_urls_and_doi_signals() {
        let resolver = IeeeResolver::new(None).unwrap();
        assert!(resolver.can_handle(
            "https://ieeexplore.ieee.org/document/1234567/",
            InputType::Url
        ));
        assert!(resolver.can_handle("https://doi.org/10.1109/5.771073", InputType::Url));
        assert!(resolver.can_handle("10.1109/5.771073", InputType::Doi));
        assert!(!resolver.can_handle("10.1007/s00134-020-06294-x", InputType::Doi));
    }

    #[test]
    fn test_extract_document_id_and_stamp_url() {
        assert_eq!(
            extract_document_id("/document/7654321/").as_deref(),
            Some("7654321")
        );
        assert!(is_direct_stamp_url(
            "https://ieeexplore.ieee.org/stamp/stamp.jsp?tp=&arnumber=7654321"
        ));
    }

    #[test]
    fn test_auth_page_detection() {
        let url = Url::parse("https://ieeexplore.ieee.org/document/1234567/").unwrap();
        assert!(is_auth_page(
            "<html>Sign in for access through your institution</html>",
            &url
        ));
        assert!(!is_auth_page("<html>Regular abstract page</html>", &url));
    }
}
