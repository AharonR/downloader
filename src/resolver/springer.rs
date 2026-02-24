//! Springer resolver for `link.springer.com` URLs and `10.1007/*` DOI patterns.

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
    absolutize_url, auth_requirement, extract_meta_value, extract_year_from_str,
    is_auth_required_status, CITATION_PDF_RE, compile_static_regex, hosts_match, looks_like_doi,
    parse_host_or_fallback,
};
use super::{ResolveContext, ResolveError, ResolveStep, ResolvedUrl, Resolver, ResolverPriority};

const DEFAULT_BASE_URL: &str = "https://link.springer.com";
const DEFAULT_DOI_BASE_URL: &str = "https://doi.org";
const SPRINGER_DOI_PREFIX: &str = "10.1007/";

static CONTENT_PDF_LINK_RE: LazyLock<Regex> = LazyLock::new(|| {
    compile_static_regex(r#"(?is)href\s*=\s*["']([^"']*/content/pdf/[^"']+\.pdf[^"']*)["']"#)
});
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

/// Specialized resolver for Springer inputs.
pub struct SpringerResolver {
    client: Client,
    base_url: String,
    doi_base_url: String,
    base_host: String,
    doi_base_host: String,
}

impl SpringerResolver {
    /// Creates resolver with default Springer and DOI endpoints.
    ///
    /// # Errors
    ///
    /// Returns [`ResolveError`] when HTTP client construction fails.
    pub fn new(cookie_jar: Option<Arc<Jar>>) -> Result<Self, ResolveError> {
        Self::with_base_urls(cookie_jar, DEFAULT_BASE_URL, DEFAULT_DOI_BASE_URL)
    }

    /// Creates resolver with custom base URLs (for tests).
    ///
    /// # Errors
    ///
    /// Returns [`ResolveError`] when HTTP client construction fails.
    pub fn with_base_urls(
        cookie_jar: Option<Arc<Jar>>,
        base_url: impl Into<String>,
        doi_base_url: impl Into<String>,
    ) -> Result<Self, ResolveError> {
        let base_url = base_url.into();
        let doi_base_url = doi_base_url.into();

        Ok(Self {
            client: build_resolver_http_client(
                "springer",
                standard_user_agent("springer"),
                cookie_jar,
            )?,
            base_host: parse_host_or_fallback(&base_url),
            doi_base_host: parse_host_or_fallback(&doi_base_url),
            base_url,
            doi_base_url,
        })
    }
}

impl std::fmt::Debug for SpringerResolver {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SpringerResolver")
            .field("base_url", &self.base_url)
            .field("doi_base_url", &self.doi_base_url)
            .finish_non_exhaustive()
    }
}

#[async_trait]
impl Resolver for SpringerResolver {
    fn name(&self) -> &'static str {
        "springer"
    }

    fn priority(&self) -> ResolverPriority {
        ResolverPriority::Specialized
    }

    fn can_handle(&self, input: &str, input_type: InputType) -> bool {
        match input_type {
            InputType::Doi => looks_like_doi(input, SPRINGER_DOI_PREFIX),
            InputType::Url => {
                let Ok(url) = Url::parse(input) else {
                    return false;
                };
                let Some(host) = url.host_str() else {
                    return false;
                };

                if hosts_match(host, &self.base_host) {
                    return is_springer_resource_path(url.path());
                }

                hosts_match(host, &self.doi_base_host)
                    && looks_like_doi(url.path().trim_start_matches('/'), SPRINGER_DOI_PREFIX)
            }
            _ => false,
        }
    }

    #[tracing::instrument(skip(self, _ctx), fields(resolver = "springer", input = %input))]
    async fn resolve(
        &self,
        input: &str,
        _ctx: &ResolveContext,
    ) -> Result<ResolveStep, ResolveError> {
        let request_url = normalize_input_url(input, &self.base_url, &self.doi_base_url);

        if looks_like_direct_pdf_url(&request_url) {
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
                "Unable to reach Springer page for PDF resolution",
            )));
        };

        let status = response.status();
        if is_auth_required_status(status.as_u16()) {
            return Ok(ResolveStep::NeedsAuth(auth_requirement(
                "link.springer.com",
                "link.springer.com",
                "Springer returned an authorization response. Retry with authenticated session cookies from your institution.",
            )));
        }
        if !status.is_success() {
            return Ok(ResolveStep::Failed(ResolveError::resolution_failed(
                input,
                &format!("Springer returned HTTP {}", status.as_u16()),
            )));
        }

        let final_url = response.url().clone();
        let Ok(html) = response.text().await else {
            return Ok(ResolveStep::Failed(ResolveError::resolution_failed(
                input,
                "Springer response body could not be parsed",
            )));
        };

        let doi = extract_meta_value(&html, &DOI_RE)
            .or_else(|| extract_input_doi(input))
            .or_else(|| extract_doi_from_path(final_url.path()));

        let explicit_pdf_url = extract_pdf_url(&html, &final_url);
        if explicit_pdf_url.is_none() && is_auth_or_paywall_page(&html) {
            return Ok(ResolveStep::NeedsAuth(auth_requirement(
                final_url.host_str().unwrap_or(""),
                "link.springer.com",
                "Springer page appears to require subscription access. Retry with authenticated session cookies from your institution.",
            )));
        }

        let pdf_url = explicit_pdf_url.or_else(|| {
            doi.as_ref().map(|value| {
                format!(
                    "{}/content/pdf/{value}.pdf",
                    self.base_url.trim_end_matches('/')
                )
            })
        });

        let Some(pdf_url) = pdf_url else {
            return Ok(ResolveStep::Failed(ResolveError::resolution_failed(
                input,
                "No Springer PDF link could be extracted from article metadata",
            )));
        };

        let mut metadata = HashMap::new();
        metadata.insert("source_url".to_string(), final_url.to_string());
        if let Some(value) = doi {
            metadata.insert("doi".to_string(), value);
        }
        if let Some(title) = extract_meta_value(&html, &TITLE_RE) {
            metadata.insert("title".to_string(), title);
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

fn normalize_input_url(input: &str, base_url: &str, doi_base_url: &str) -> String {
    let trimmed = input.trim();
    if looks_like_doi(trimmed, SPRINGER_DOI_PREFIX) {
        return format!("{}/article/{}", base_url.trim_end_matches('/'), trimmed);
    }

    let Ok(url) = Url::parse(trimmed) else {
        return trimmed.to_string();
    };

    if hosts_match(
        url.host_str().unwrap_or_default(),
        parse_host_or_fallback(doi_base_url).as_str(),
    ) {
        let candidate = url.path().trim_start_matches('/');
        if looks_like_doi(candidate, SPRINGER_DOI_PREFIX) {
            return format!("{}/article/{}", base_url.trim_end_matches('/'), candidate);
        }
    }

    trimmed.to_string()
}

fn is_springer_resource_path(path: &str) -> bool {
    path.starts_with("/article/")
        || path.starts_with("/chapter/")
        || path.starts_with("/content/pdf/")
}

fn looks_like_direct_pdf_url(value: &str) -> bool {
    Url::parse(value).ok().is_some_and(|url| {
        url.path().to_ascii_lowercase().starts_with("/content/pdf/")
            && url.path().to_ascii_lowercase().contains(".pdf")
    })
}

fn extract_pdf_url(html: &str, base_url: &Url) -> Option<String> {
    extract_meta_value(html, &CITATION_PDF_RE)
        .or_else(|| {
            CONTENT_PDF_LINK_RE
                .captures(html)
                .and_then(|caps| caps.get(1).map(|m| m.as_str().trim().to_string()))
        })
        .and_then(|value| absolutize_url(&value, base_url))
}

fn extract_input_doi(input: &str) -> Option<String> {
    let trimmed = input.trim();
    if looks_like_doi(trimmed, SPRINGER_DOI_PREFIX) {
        return Some(trimmed.to_string());
    }

    Url::parse(trimmed)
        .ok()
        .map(|url| url.path().trim_start_matches('/').to_string())
        .filter(|candidate| looks_like_doi(candidate, SPRINGER_DOI_PREFIX))
}

fn extract_doi_from_path(path: &str) -> Option<String> {
    let value = path
        .strip_prefix("/article/")
        .or_else(|| path.strip_prefix("/chapter/"))
        .or_else(|| path.strip_prefix("/content/pdf/"))
        .map(str::to_string)?;

    let value = value.trim_end_matches(".pdf");
    looks_like_doi(value, SPRINGER_DOI_PREFIX).then(|| value.to_string())
}

fn is_auth_or_paywall_page(html: &str) -> bool {
    let normalized = html.to_ascii_lowercase();
    let markers = [
        "buy article",
        "purchase pdf",
        "access through your institution",
        "log in via an institution",
        "subscribe to this journal",
    ];

    let hits = markers
        .iter()
        .filter(|marker| normalized.contains(**marker))
        .count();

    hits >= 2
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn test_springer_can_handle_urls_and_doi() {
        let resolver = SpringerResolver::new(None).unwrap();
        assert!(resolver.can_handle(
            "https://link.springer.com/article/10.1007/s00134-020-06294-x",
            InputType::Url
        ));
        assert!(resolver.can_handle(
            "https://link.springer.com/content/pdf/10.1007/s00134-020-06294-x.pdf",
            InputType::Url
        ));
        assert!(resolver.can_handle("10.1007/s00134-020-06294-x", InputType::Doi));
        assert!(!resolver.can_handle("10.1109/5.771073", InputType::Doi));
    }

    #[test]
    fn test_extract_doi_from_known_path_forms() {
        assert_eq!(
            extract_doi_from_path("/article/10.1007/s00134-020-06294-x").unwrap(),
            "10.1007/s00134-020-06294-x"
        );
        assert_eq!(
            extract_doi_from_path("/content/pdf/10.1007/s00134-020-06294-x.pdf").unwrap(),
            "10.1007/s00134-020-06294-x"
        );
    }

    #[test]
    fn test_paywall_detection_requires_multiple_markers() {
        assert!(is_auth_or_paywall_page(
            "Buy article with access through your institution"
        ));
        assert!(!is_auth_or_paywall_page("Single marker buy article"));
    }
}
