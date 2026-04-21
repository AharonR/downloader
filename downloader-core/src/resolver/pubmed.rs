//! `PubMed` resolver for routing `PubMed` records to `PMC` full-text PDF URLs.

use std::collections::HashMap;
use std::sync::Arc;
use std::sync::LazyLock;

use async_trait::async_trait;
use regex::Regex;
use reqwest::Client;
use reqwest::cookie::Jar;
use reqwest::header::ACCEPT;
use url::Url;

use crate::parser::{BARE_PMC_ID_PATTERN, InputType};

use super::http_client::{build_resolver_http_client, standard_user_agent};
use super::utils::{
    CITATION_PDF_RE, absolutize_url, compile_static_regex, hosts_match, parse_host_or_fallback,
};
use super::{ResolveContext, ResolveError, ResolveStep, ResolvedUrl, Resolver, ResolverPriority};

const DEFAULT_PUBMED_BASE_URL: &str = "https://pubmed.ncbi.nlm.nih.gov";
const DEFAULT_PMC_BASE_URL: &str = "https://pmc.ncbi.nlm.nih.gov";

static PMCID_RE: LazyLock<Regex> = LazyLock::new(|| compile_static_regex(r"(?i)\b(PMC\d{4,})\b"));
/// Matches a bare PMC identifier as the whole input (e.g. "PMC1234567").
static BARE_PMCID_RE: LazyLock<Regex> = LazyLock::new(|| compile_static_regex(BARE_PMC_ID_PATTERN));
/// Matches the normalised PMID form emitted by the parser ("PMID:12345678").
static BARE_PMID_RE: LazyLock<Regex> =
    LazyLock::new(|| compile_static_regex(r"(?i)^PMID:(\d{1,9})$"));
static PDF_LINK_RE: LazyLock<Regex> = LazyLock::new(|| {
    compile_static_regex(r#"(?is)href\s*=\s*["']([^"']*(?:/pdf/[^"']*|\.pdf(?:\?[^"']*)?))["']"#)
});

/// Specialized resolver for `PubMed` and `PMC` URLs.
pub struct PubMedResolver {
    client: Client,
    pubmed_base_url: String,
    pmc_base_url: String,
    pubmed_host: String,
    pmc_host: String,
}

impl PubMedResolver {
    /// Creates a resolver with default PubMed/PMC endpoints.
    ///
    /// # Errors
    ///
    /// Returns [`ResolveError`] when HTTP client construction fails.
    pub fn new(cookie_jar: Option<Arc<Jar>>) -> Result<Self, ResolveError> {
        Self::with_base_urls(cookie_jar, DEFAULT_PUBMED_BASE_URL, DEFAULT_PMC_BASE_URL)
    }

    /// Creates a resolver with custom endpoints for tests.
    ///
    /// # Errors
    ///
    /// Returns [`ResolveError`] when HTTP client construction fails.
    pub fn with_base_urls(
        cookie_jar: Option<Arc<Jar>>,
        pubmed_base_url: impl Into<String>,
        pmc_base_url: impl Into<String>,
    ) -> Result<Self, ResolveError> {
        let pubmed_base_url = pubmed_base_url.into();
        let pmc_base_url = pmc_base_url.into();

        Ok(Self {
            client: build_resolver_http_client(
                "pubmed",
                standard_user_agent("pubmed"),
                cookie_jar,
            )?,
            pubmed_host: parse_host_or_fallback(&pubmed_base_url),
            pmc_host: parse_host_or_fallback(&pmc_base_url),
            pubmed_base_url,
            pmc_base_url,
        })
    }
}

impl std::fmt::Debug for PubMedResolver {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PubMedResolver")
            .field("pubmed_base_url", &self.pubmed_base_url)
            .field("pmc_base_url", &self.pmc_base_url)
            .finish_non_exhaustive()
    }
}

#[async_trait]
impl Resolver for PubMedResolver {
    fn name(&self) -> &'static str {
        "pubmed"
    }

    fn priority(&self) -> ResolverPriority {
        ResolverPriority::Specialized
    }

    fn can_handle(&self, input: &str, input_type: InputType) -> bool {
        // Accept bare PMC IDs ("PMC1234567") and PMID-prefixed inputs ("PMID:12345678")
        // that the parser emits as InputType::Unknown.
        if input_type == InputType::Unknown {
            let trimmed = input.trim();
            return BARE_PMCID_RE.is_match(trimmed) || BARE_PMID_RE.is_match(trimmed);
        }

        if input_type != InputType::Url {
            return false;
        }

        let Ok(url) = Url::parse(input) else {
            return false;
        };
        let Some(host) = url.host_str() else {
            return false;
        };

        if hosts_match(host, &self.pmc_host) && looks_like_pmc_path(url.path()) {
            return true;
        }

        hosts_match(host, &self.pubmed_host)
    }

    #[tracing::instrument(skip(self, _ctx), fields(resolver = "pubmed", input = %input))]
    async fn resolve(
        &self,
        input: &str,
        _ctx: &ResolveContext,
    ) -> Result<ResolveStep, ResolveError> {
        let trimmed = input.trim();

        // Bare PMC ID (e.g. "PMC1234567") → redirect to the PMC article page,
        // which the URL-based branch below then resolves to a PDF.
        if BARE_PMCID_RE.is_match(trimmed) {
            let pmcid = trimmed.to_ascii_uppercase();
            let redirect_url = format!(
                "{}/articles/{}/",
                self.pmc_base_url.trim_end_matches('/'),
                pmcid
            );
            return Ok(ResolveStep::Redirect(redirect_url));
        }

        // PMID with prefix (e.g. "PMID:12345678") → redirect to the PubMed record.
        if let Some(caps) = BARE_PMID_RE.captures(trimmed) {
            let pmid = &caps[1];
            let redirect_url = format!("{}/{}/", self.pubmed_base_url.trim_end_matches('/'), pmid);
            return Ok(ResolveStep::Redirect(redirect_url));
        }

        let Ok(parsed) = Url::parse(input) else {
            return Ok(ResolveStep::Failed(ResolveError::resolution_failed(
                input,
                "PubMed resolver expected a valid URL but the input could not be parsed",
            )));
        };

        let host = parsed.host_str().unwrap_or_default();
        if hosts_match(host, &self.pmc_host) && looks_like_pmc_path(parsed.path()) {
            return self.resolve_pmc_url(input, parsed).await;
        }

        if !hosts_match(host, &self.pubmed_host) {
            return Ok(ResolveStep::Failed(ResolveError::resolution_failed(
                input,
                "URL does not belong to PubMed or PMC",
            )));
        }

        let Ok(pubmed_response) = self
            .client
            .get(input)
            .header(
                ACCEPT,
                "text/html,application/xhtml+xml,application/xml;q=0.9,*/*;q=0.8",
            )
            .send()
            .await
        else {
            return Ok(ResolveStep::Failed(ResolveError::resolution_failed(
                input,
                "Unable to fetch PubMed page. Check network connectivity and retry.",
            )));
        };

        if !pubmed_response.status().is_success() {
            return Ok(ResolveStep::Failed(ResolveError::resolution_failed(
                input,
                &format!("PubMed returned HTTP {}", pubmed_response.status().as_u16()),
            )));
        }

        let final_url = pubmed_response.url().clone();
        let Ok(html) = pubmed_response.text().await else {
            return Ok(ResolveStep::Failed(ResolveError::resolution_failed(
                input,
                "PubMed page could not be read for PMC full-text discovery",
            )));
        };

        let Some(pmcid) = extract_pmcid(&html) else {
            return Ok(ResolveStep::Failed(ResolveError::resolution_failed(
                input,
                "PubMed entry does not expose an open-access PMC full-text link",
            )));
        };

        self.resolve_pmcid_from_pubmed(&pmcid, &final_url).await
    }
}

impl PubMedResolver {
    async fn resolve_pmc_url(&self, input: &str, parsed: Url) -> Result<ResolveStep, ResolveError> {
        if looks_like_direct_pdf_path(parsed.path()) {
            let mut metadata = HashMap::new();
            metadata.insert("source_url".to_string(), parsed.as_str().to_string());
            if let Some(pmcid) = extract_pmcid(parsed.as_str()) {
                metadata.insert("pmcid".to_string(), pmcid);
            }
            return Ok(ResolveStep::Url(ResolvedUrl::with_metadata(
                parsed.as_str(),
                metadata,
            )));
        }

        let pmcid = extract_pmcid(parsed.as_str()).or_else(|| extract_pmcid(parsed.path()));
        let Some(pmcid) = pmcid else {
            return Ok(ResolveStep::Failed(ResolveError::resolution_failed(
                input,
                "PMC URL did not contain a recognizable PMCID identifier",
            )));
        };

        self.resolve_pmcid_from_pubmed(&pmcid, &parsed).await
    }

    async fn resolve_pmcid_from_pubmed(
        &self,
        pmcid: &str,
        source_url: &Url,
    ) -> Result<ResolveStep, ResolveError> {
        let pmc_article = format!(
            "{}/articles/{}/",
            self.pmc_base_url.trim_end_matches('/'),
            pmcid
        );

        let Ok(response) = self
            .client
            .get(&pmc_article)
            .header(
                ACCEPT,
                "text/html,application/xhtml+xml,application/xml;q=0.9,*/*;q=0.8",
            )
            .send()
            .await
        else {
            return Ok(ResolveStep::Failed(ResolveError::resolution_failed(
                &pmc_article,
                "PMC full-text page could not be fetched for PDF extraction",
            )));
        };

        if !response.status().is_success() {
            return Ok(ResolveStep::Failed(ResolveError::resolution_failed(
                &pmc_article,
                &format!("PMC returned HTTP {}", response.status().as_u16()),
            )));
        }

        let final_url = response.url().clone();
        let Ok(html) = response.text().await else {
            return Ok(ResolveStep::body_parse_failed(&pmc_article, "PMC"));
        };

        let pdf_url = extract_pdf_url(&html, &final_url).or_else(|| {
            Some(format!(
                "{}/articles/{}/pdf/",
                self.pmc_base_url.trim_end_matches('/'),
                pmcid
            ))
        });

        let Some(pdf_url) = pdf_url else {
            return Ok(ResolveStep::Failed(ResolveError::resolution_failed(
                &pmc_article,
                "PMC article does not expose a downloadable PDF target",
            )));
        };

        let mut metadata = HashMap::new();
        metadata.insert("source_url".to_string(), source_url.as_str().to_string());
        metadata.insert("pmcid".to_string(), pmcid.to_string());
        if let Some(pmid) = extract_pmid(source_url) {
            metadata.insert("pmid".to_string(), pmid);
        }

        Ok(ResolveStep::Url(ResolvedUrl::with_metadata(
            pdf_url, metadata,
        )))
    }
}

fn extract_pmcid(value: &str) -> Option<String> {
    PMCID_RE
        .captures(value)
        .and_then(|caps| caps.get(1).map(|m| m.as_str().to_ascii_uppercase()))
}

fn extract_pdf_url(html: &str, base_url: &Url) -> Option<String> {
    CITATION_PDF_RE
        .captures(html)
        .and_then(|caps| caps.get(1).map(|m| m.as_str().trim().to_string()))
        .or_else(|| {
            PDF_LINK_RE
                .captures(html)
                .and_then(|caps| caps.get(1).map(|m| m.as_str().trim().to_string()))
        })
        .and_then(|value| absolutize_url(&value, base_url))
}

fn looks_like_direct_pdf_path(path: &str) -> bool {
    let lower = path.to_ascii_lowercase();
    let has_pdf_extension = std::path::Path::new(path)
        .extension()
        .and_then(|ext| ext.to_str())
        .is_some_and(|ext| ext.eq_ignore_ascii_case("pdf"));

    lower.contains("/pdf/") || has_pdf_extension
}

fn looks_like_pmc_path(path: &str) -> bool {
    path.to_ascii_lowercase().contains("/articles/pmc")
}

fn extract_pmid(url: &Url) -> Option<String> {
    url.path_segments()?
        .find(|segment| !segment.is_empty() && segment.chars().all(|c| c.is_ascii_digit()))
        .map(std::string::ToString::to_string)
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn test_pubmed_can_handle_pubmed_and_pmc_hosts() {
        let resolver = PubMedResolver::new(None).unwrap();
        assert!(resolver.can_handle("https://pubmed.ncbi.nlm.nih.gov/12345678/", InputType::Url));
        assert!(resolver.can_handle(
            "https://pmc.ncbi.nlm.nih.gov/articles/PMC1234567/",
            InputType::Url
        ));
        assert!(!resolver.can_handle("https://example.com/12345678", InputType::Url));
        assert!(!resolver.can_handle("10.48550/arXiv.2301.01234", InputType::Doi));
    }

    #[test]
    fn test_extract_pmcid_from_html_and_url_forms() {
        assert_eq!(
            extract_pmcid(r#"<a href="/articles/PMC1234567/">PMC1234567</a>"#).unwrap(),
            "PMC1234567"
        );
        assert_eq!(
            extract_pmcid("https://pmc.ncbi.nlm.nih.gov/articles/PMC7654321/pdf/").unwrap(),
            "PMC7654321"
        );
    }

    #[test]
    fn test_pubmed_can_handle_bare_pmc_id() {
        let resolver = PubMedResolver::new(None).unwrap();
        assert!(resolver.can_handle("PMC1234567", InputType::Unknown));
        assert!(resolver.can_handle("pmc9999999", InputType::Unknown));
        assert!(!resolver.can_handle("1234567", InputType::Unknown)); // bare number not accepted
        assert!(!resolver.can_handle("PMC123", InputType::Unknown)); // too short (< 4 digits)
    }

    #[test]
    fn test_pubmed_can_handle_pmid_with_prefix() {
        let resolver = PubMedResolver::new(None).unwrap();
        assert!(resolver.can_handle("PMID:12345678", InputType::Unknown));
        assert!(resolver.can_handle("pmid:99", InputType::Unknown));
        assert!(!resolver.can_handle("PMID:", InputType::Unknown)); // no digits
    }

    #[tokio::test]
    async fn test_pubmed_resolve_bare_pmc_id_redirects() {
        let resolver = PubMedResolver::with_base_urls(
            None,
            "https://pubmed.ncbi.nlm.nih.gov",
            "https://pmc.ncbi.nlm.nih.gov",
        )
        .unwrap();
        let ctx = ResolveContext::default();
        let step = resolver.resolve("PMC1234567", &ctx).await.unwrap();
        match step {
            ResolveStep::Redirect(url) => {
                assert_eq!(url, "https://pmc.ncbi.nlm.nih.gov/articles/PMC1234567/");
            }
            other => panic!("expected Redirect, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn test_pubmed_resolve_pmc_id_normalises_to_uppercase() {
        let resolver = PubMedResolver::with_base_urls(
            None,
            "https://pubmed.ncbi.nlm.nih.gov",
            "https://pmc.ncbi.nlm.nih.gov",
        )
        .unwrap();
        let ctx = ResolveContext::default();
        let step = resolver.resolve("pmc1234567", &ctx).await.unwrap();
        match step {
            ResolveStep::Redirect(url) => {
                assert!(
                    url.contains("PMC1234567"),
                    "PMCID must be uppercased in redirect URL"
                );
            }
            other => panic!("expected Redirect, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn test_pubmed_resolve_pmid_prefix_redirects() {
        let resolver = PubMedResolver::with_base_urls(
            None,
            "https://pubmed.ncbi.nlm.nih.gov",
            "https://pmc.ncbi.nlm.nih.gov",
        )
        .unwrap();
        let ctx = ResolveContext::default();
        let step = resolver.resolve("PMID:12345678", &ctx).await.unwrap();
        match step {
            ResolveStep::Redirect(url) => {
                assert_eq!(url, "https://pubmed.ncbi.nlm.nih.gov/12345678/");
            }
            other => panic!("expected Redirect, got {other:?}"),
        }
    }

    #[test]
    fn test_extract_pdf_url_prefers_citation_meta() {
        let html = r#"
            <meta name="citation_pdf_url" content="/articles/PMC1234567/pdf/main.pdf">
            <a href="/articles/PMC1234567/pdf/ignored.pdf">Download</a>
        "#;
        let base = Url::parse("https://pmc.ncbi.nlm.nih.gov/articles/PMC1234567/").unwrap();
        let pdf = extract_pdf_url(html, &base).unwrap();
        assert_eq!(
            pdf,
            "https://pmc.ncbi.nlm.nih.gov/articles/PMC1234567/pdf/main.pdf"
        );
    }
}
