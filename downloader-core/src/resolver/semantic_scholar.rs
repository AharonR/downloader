//! Shared Semantic Scholar API types and resolution helpers.
//!
//! Used by [`AcmResolver`](super::acm::AcmResolver) and
//! [`WileyResolver`](super::wiley::WileyResolver) to query open-access metadata
//! before falling back to publisher PDFs.

use std::collections::HashMap;

use reqwest::Client;
use serde::Deserialize;
use tracing::{debug, warn};
use url::Url;

use super::{ResolveError, ResolveStep, ResolvedUrl};

// ==================== Constants ====================

pub(crate) const DEFAULT_S2_BASE_URL: &str = "https://api.semanticscholar.org";
pub(crate) const S2_FIELDS: &str = "externalIds,openAccessPdf,title,authors,year";

// ==================== Config ====================

/// Configuration for a Semantic Scholar resolution call.
///
/// Groups resolver-specific parameters so callers don't need a 7-argument function.
pub(crate) struct S2ResolveConfig<'a> {
    /// Direct PDF URL to return when no open-access alternative is found.
    pub fallback_pdf_url: &'a str,
    /// Publisher domains to reject when evaluating open-access PDF URLs.
    /// OA PDFs hosted on these domains (or their subdomains) are skipped.
    pub publisher_domains: &'a [&'a str],
    /// Resolver name used in log messages.
    pub resolver_name: &'a str,
}

// ==================== Response Types ====================

#[derive(Debug, Deserialize)]
pub(crate) struct S2PaperResponse {
    pub title: Option<String>,
    pub authors: Option<Vec<S2Author>>,
    pub year: Option<i32>,
    #[serde(rename = "externalIds")]
    pub external_ids: Option<S2ExternalIds>,
    #[serde(rename = "openAccessPdf")]
    pub open_access_pdf: Option<S2OpenAccessPdf>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct S2Author {
    pub name: Option<String>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct S2ExternalIds {
    #[serde(rename = "ArXiv")]
    pub arxiv: Option<String>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct S2OpenAccessPdf {
    pub url: Option<String>,
}

// ==================== Resolution ====================

/// Queries Semantic Scholar and returns the best available [`ResolveStep`].
///
/// Priority:
/// 1. `openAccessPdf.url` if `https://` and NOT on a publisher domain
/// 2. arXiv PDF constructed from `externalIds.ArXiv`
/// 3. `config.fallback_pdf_url` (the publisher's PDF URL, which may require auth)
pub(crate) async fn resolve_via_s2(
    client: &Client,
    s2_base_url: &str,
    doi: &str,
    original_input: &str,
    config: &S2ResolveConfig<'_>,
) -> Result<ResolveStep, ResolveError> {
    let url = format!("{s2_base_url}/graph/v1/paper/DOI:{doi}?fields={S2_FIELDS}");

    debug!(s2_url = %url, resolver = config.resolver_name, "Querying Semantic Scholar");

    let response = match client.get(&url).send().await {
        Ok(resp) => resp,
        Err(e) => {
            warn!(error = %e, doi, resolver = config.resolver_name, "Semantic Scholar request failed");
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
        debug!(
            status = status.as_u16(),
            %reason,
            resolver = config.resolver_name,
            "Semantic Scholar API error"
        );
        return Ok(ResolveStep::Failed(ResolveError::resolution_failed(
            original_input,
            &reason,
        )));
    }

    let paper: S2PaperResponse = match response.json().await {
        Ok(parsed) => parsed,
        Err(e) => {
            warn!(
                error = %e,
                doi,
                resolver = config.resolver_name,
                "Failed to parse Semantic Scholar response"
            );
            return Ok(ResolveStep::body_parse_failed(original_input, "Semantic Scholar"));
        }
    };

    let pdf_url = select_best_pdf_url(&paper, config);
    let metadata = build_metadata(&paper, doi);

    debug!(pdf_url = %pdf_url, resolver = config.resolver_name, "Resolved paper to PDF URL");
    Ok(ResolveStep::Url(ResolvedUrl::with_metadata(pdf_url, metadata)))
}

// ==================== URL selection ====================

/// Selects the best available PDF URL from a Semantic Scholar response.
pub(crate) fn select_best_pdf_url(
    paper: &S2PaperResponse,
    config: &S2ResolveConfig<'_>,
) -> String {
    // Priority 1: openAccessPdf on a non-publisher https:// domain
    if let Some(oa) = &paper.open_access_pdf {
        if let Some(oa_url) = &oa.url {
            if is_usable_oa_url(oa_url, config.publisher_domains) {
                debug!(oa_url = %oa_url, "Using open-access PDF from Semantic Scholar");
                return oa_url.clone();
            }
            debug!("Semantic Scholar OA PDF skipped (publisher domain or non-HTTPS)");
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

    // Fallback: publisher PDF URL (may require auth via `downloader auth capture`)
    debug!(
        resolver = config.resolver_name,
        "No open-access alternative found; falling back to publisher PDF URL"
    );
    config.fallback_pdf_url.to_string()
}

/// DOI resolver domains that are never direct PDFs — they redirect to publisher pages.
const DOI_RESOLVER_DOMAINS: &[&str] = &["doi.org", "dx.doi.org"];

/// Returns true if `url` is a usable open-access PDF:
/// - `https://` scheme
/// - NOT a DOI resolver URL (`doi.org`, `dx.doi.org`)
/// - NOT hosted on any of `rejected_domains` or their subdomains
pub(crate) fn is_usable_oa_url(url: &str, rejected_domains: &[&str]) -> bool {
    if !url.starts_with("https://") {
        return false;
    }
    let Some(host) = Url::parse(url)
        .ok()
        .and_then(|parsed| parsed.host_str().map(str::to_ascii_lowercase))
    else {
        return false;
    };
    // DOI resolver URLs are never direct PDFs — they just redirect to publisher pages.
    if DOI_RESOLVER_DOMAINS.iter().any(|d| host == *d) {
        return false;
    }
    !rejected_domains.iter().any(|domain| {
        host == *domain
            || (host.len() > domain.len()
                && host.as_bytes()[host.len() - domain.len() - 1] == b'.'
                && host.ends_with(*domain))
    })
}

// ==================== Metadata ====================

pub(crate) fn build_metadata(paper: &S2PaperResponse, doi: &str) -> HashMap<String, String> {
    let mut metadata = HashMap::with_capacity(5);

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

// ==================== Error helpers ====================

pub(crate) fn s2_error_reason(status: u16) -> String {
    match status {
        404 => "Paper not found in Semantic Scholar. \
                Why: this DOI may not be indexed yet. \
                Fix: other resolvers (e.g. Crossref) will be tried automatically."
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

    fn acm_config(fallback: &str) -> S2ResolveConfig<'_> {
        S2ResolveConfig {
            fallback_pdf_url: fallback,
            publisher_domains: &["dl.acm.org"],
            resolver_name: "acm",
        }
    }

    fn wiley_config(fallback: &str) -> S2ResolveConfig<'_> {
        S2ResolveConfig {
            fallback_pdf_url: fallback,
            publisher_domains: &["onlinelibrary.wiley.com", "wiley.com"],
            resolver_name: "wiley",
        }
    }

    fn make_paper(oa_url: Option<&str>, arxiv_id: Option<&str>) -> S2PaperResponse {
        S2PaperResponse {
            title: None,
            authors: None,
            year: None,
            external_ids: arxiv_id.map(|id| S2ExternalIds {
                arxiv: Some(id.to_string()),
            }),
            open_access_pdf: oa_url.map(|url| S2OpenAccessPdf {
                url: Some(url.to_string()),
            }),
        }
    }

    // ==================== is_usable_oa_url ====================

    #[test]
    fn test_is_usable_oa_url_accepts_arxiv_https() {
        assert!(is_usable_oa_url(
            "https://arxiv.org/pdf/2108.04144",
            &["dl.acm.org"]
        ));
    }

    #[test]
    fn test_is_usable_oa_url_rejects_http_scheme() {
        assert!(!is_usable_oa_url(
            "http://arxiv.org/pdf/2108.04144",
            &["dl.acm.org"]
        ));
    }

    #[test]
    fn test_is_usable_oa_url_rejects_exact_domain_match() {
        assert!(!is_usable_oa_url(
            "https://dl.acm.org/doi/pdf/10.1145/123",
            &["dl.acm.org"]
        ));
    }

    #[test]
    fn test_is_usable_oa_url_rejects_subdomain_of_rejected_domain() {
        assert!(!is_usable_oa_url(
            "https://mirror.dl.acm.org/doi/pdf/10.1145/123",
            &["dl.acm.org"]
        ));
    }

    #[test]
    fn test_is_usable_oa_url_multi_domain_rejection_list() {
        // Wiley domains
        assert!(!is_usable_oa_url(
            "https://onlinelibrary.wiley.com/doi/pdf/10.1002/foo",
            &["onlinelibrary.wiley.com", "wiley.com"]
        ));
        assert!(!is_usable_oa_url(
            "https://wiley.com/something",
            &["onlinelibrary.wiley.com", "wiley.com"]
        ));
        // arXiv is still accepted
        assert!(is_usable_oa_url(
            "https://arxiv.org/pdf/2108.04144",
            &["onlinelibrary.wiley.com", "wiley.com"]
        ));
    }

    #[test]
    fn test_is_usable_oa_url_rejects_subdomain_of_second_domain_in_list() {
        assert!(!is_usable_oa_url(
            "https://cdn.wiley.com/pdf/foo",
            &["onlinelibrary.wiley.com", "wiley.com"]
        ));
    }

    #[test]
    fn test_is_usable_oa_url_rejects_doi_org() {
        // doi.org URLs are redirect resolvers, not direct PDFs
        assert!(!is_usable_oa_url("https://doi.org/10.1002/cre2.70001", &[]));
        assert!(!is_usable_oa_url(
            "https://dx.doi.org/10.1002/cre2.70001",
            &[]
        ));
    }

    #[test]
    fn test_is_usable_oa_url_empty_rejected_domains_accepts_any_https() {
        assert!(is_usable_oa_url("https://anything.com/pdf", &[]));
    }

    // ==================== select_best_pdf_url ====================

    #[test]
    fn test_select_best_pdf_url_prefers_open_access_pdf_field_over_external_id() {
        let paper = make_paper(Some("https://arxiv.org/pdf/1234"), Some("9999"));
        let config = acm_config("https://dl.acm.org/doi/pdf/10.1145/x");
        assert_eq!(
            select_best_pdf_url(&paper, &config),
            "https://arxiv.org/pdf/1234"
        );
    }

    #[test]
    fn test_select_best_pdf_url_skips_publisher_oa_uses_arxiv() {
        let paper = make_paper(
            Some("https://dl.acm.org/doi/pdf/10.1145/x"),
            Some("2108.04144"),
        );
        let config = acm_config("https://dl.acm.org/doi/pdf/10.1145/x");
        assert_eq!(
            select_best_pdf_url(&paper, &config),
            "https://arxiv.org/pdf/2108.04144"
        );
    }

    #[test]
    fn test_select_best_pdf_url_falls_back_to_config_fallback() {
        let paper = make_paper(None, None);
        let fallback = "https://dl.acm.org/doi/pdf/10.1145/x";
        let config = acm_config(fallback);
        assert_eq!(select_best_pdf_url(&paper, &config), fallback);
    }

    #[test]
    fn test_select_best_pdf_url_wiley_rejects_onlinelibrary_url() {
        let paper = make_paper(
            Some("https://onlinelibrary.wiley.com/doi/pdf/10.1002/foo"),
            Some("2108.04144"),
        );
        let config = wiley_config("https://onlinelibrary.wiley.com/doi/pdf/10.1002/foo");
        assert_eq!(
            select_best_pdf_url(&paper, &config),
            "https://arxiv.org/pdf/2108.04144"
        );
    }

    // ==================== build_metadata ====================

    #[test]
    fn test_build_metadata_all_fields_present() {
        let paper = S2PaperResponse {
            title: Some("My Title".to_string()),
            authors: Some(vec![
                S2Author {
                    name: Some("Alice".to_string()),
                },
                S2Author {
                    name: Some("Bob".to_string()),
                },
            ]),
            year: Some(2023),
            external_ids: None,
            open_access_pdf: None,
        };
        let m = build_metadata(&paper, "10.1002/foo");
        assert_eq!(m.get("doi").map(String::as_str), Some("10.1002/foo"));
        assert_eq!(
            m.get("source_url").map(String::as_str),
            Some("https://doi.org/10.1002/foo")
        );
        assert_eq!(m.get("title").map(String::as_str), Some("My Title"));
        assert_eq!(
            m.get("authors").map(String::as_str),
            Some("Alice; Bob")
        );
        assert_eq!(m.get("year").map(String::as_str), Some("2023"));
    }

    #[test]
    fn test_build_metadata_all_fields_null() {
        let paper = S2PaperResponse {
            title: None,
            authors: None,
            year: None,
            external_ids: None,
            open_access_pdf: None,
        };
        let m = build_metadata(&paper, "10.1002/foo");
        assert_eq!(m.get("doi").map(String::as_str), Some("10.1002/foo"));
        assert_eq!(
            m.get("source_url").map(String::as_str),
            Some("https://doi.org/10.1002/foo")
        );
        assert!(m.get("title").is_none());
        assert!(m.get("authors").is_none());
        assert!(m.get("year").is_none());
    }

    #[test]
    fn test_build_metadata_empty_title_and_authors_not_inserted() {
        let paper = S2PaperResponse {
            title: Some(String::new()),
            authors: Some(vec![]),
            year: None,
            external_ids: None,
            open_access_pdf: None,
        };
        let m = build_metadata(&paper, "10.1002/foo");
        assert!(m.get("title").is_none());
        assert!(m.get("authors").is_none());
    }

    // ==================== s2_error_reason ====================

    #[test]
    fn test_s2_error_reason_404_mentions_not_found() {
        assert!(s2_error_reason(404).contains("not found"));
    }

    #[test]
    fn test_s2_error_reason_429_mentions_rate_limit() {
        assert!(s2_error_reason(429).contains("rate limit"));
    }

    #[test]
    fn test_s2_error_reason_500_mentions_unavailable() {
        assert!(s2_error_reason(500).contains("unavailable"));
    }

    #[test]
    fn test_s2_error_reason_other_includes_status_code() {
        assert!(s2_error_reason(418).contains("418"));
    }
}
