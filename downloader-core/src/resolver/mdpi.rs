//! MDPI resolver for `www.mdpi.com` URLs and `10.3390/*` DOI inputs.
//!
//! `www.mdpi.com` uses Akamai CDN bot detection that blocks all programmatic
//! access with HTTP 403. The actual PDFs are served from `mdpi-res.com` without
//! bot detection. This resolver intercepts MDPI inputs, calls the Crossref API
//! to obtain metadata, and constructs a direct `mdpi-res.com` CDN download URL.

use std::collections::HashMap;
use std::time::Duration;

use async_trait::async_trait;
use reqwest::Client;
use tracing::{debug, warn};
use url::Url;

use crate::parser::InputType;

use super::crossref::{
    CrossrefMessage, CrossrefResponse, CrossrefSearchResponse, extract_metadata as crossref_extract_metadata,
};
use super::http_client::{build_resolver_http_client, standard_user_agent};
use super::utils::{canonical_host, looks_like_doi, validate_crossref_mailto};
use super::{ResolveContext, ResolveError, ResolveStep, ResolvedUrl, Resolver, ResolverPriority};

const DEFAULT_CROSSREF_BASE_URL: &str = "https://api.crossref.org";
const MDPI_DOI_PREFIX: &str = "10.3390/";
const MDPI_HOST: &str = "mdpi.com";
const MDPI_CDN_BASE: &str = "https://mdpi-res.com/d_attachment";
/// Short timeout for CDN slug HEAD probes — we want to fail fast and fall back, not stall.
const CDN_PROBE_TIMEOUT: Duration = Duration::from_secs(5);

/// Journals where `lowercase(container-title with spaces stripped)` does not match the CDN slug.
/// Maps lowercased container-title → CDN slug.
static MULTI_WORD_CDN_OVERRIDES: &[(&str, &str)] = &[
    ("applied sciences", "applsci"),
];

/// Specialized resolver for MDPI articles.
///
/// Rewrites `www.mdpi.com` URLs and `10.3390/*` DOIs to direct `mdpi-res.com`
/// CDN download URLs, bypassing Akamai bot detection on `www.mdpi.com`.
pub struct MdpiResolver {
    client: Client,
    crossref_base_url: String,
    crossref_mailto: String,
}

impl MdpiResolver {
    /// Creates a new `MdpiResolver` with the production Crossref API URL.
    ///
    /// # Errors
    ///
    /// Returns [`ResolveError`] if HTTP client construction fails or `mailto` is invalid.
    pub fn new(crossref_mailto: impl Into<String>) -> Result<Self, ResolveError> {
        Self::build(crossref_mailto.into(), DEFAULT_CROSSREF_BASE_URL.to_string())
    }

    /// Creates an `MdpiResolver` with a custom Crossref base URL (for testing with wiremock).
    ///
    /// # Errors
    ///
    /// Returns [`ResolveError`] if HTTP client construction fails or `mailto` is invalid.
    pub fn with_crossref_base_url(
        crossref_mailto: impl Into<String>,
        crossref_base_url: impl Into<String>,
    ) -> Result<Self, ResolveError> {
        Self::build(crossref_mailto.into(), crossref_base_url.into())
    }

    fn build(crossref_mailto: String, crossref_base_url: String) -> Result<Self, ResolveError> {
        validate_crossref_mailto(&crossref_mailto)?;
        let client = build_resolver_http_client("mdpi", standard_user_agent("mdpi"), None)?;
        Ok(Self {
            client,
            crossref_base_url,
            crossref_mailto,
        })
    }
}

impl std::fmt::Debug for MdpiResolver {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MdpiResolver")
            .field("crossref_base_url", &self.crossref_base_url)
            .field("crossref_mailto", &self.crossref_mailto)
            .finish_non_exhaustive()
    }
}

#[async_trait]
impl Resolver for MdpiResolver {
    fn name(&self) -> &'static str {
        "mdpi"
    }

    fn priority(&self) -> ResolverPriority {
        ResolverPriority::Specialized
    }

    fn can_handle(&self, input: &str, input_type: InputType) -> bool {
        match input_type {
            InputType::Doi => looks_like_doi(input, MDPI_DOI_PREFIX),
            InputType::Url => {
                let Ok(url) = Url::parse(input) else {
                    return false;
                };
                let Some(host) = url.host_str() else {
                    return false;
                };
                canonical_host(host) == MDPI_HOST && is_mdpi_article_path(url.path())
            }
            _ => false,
        }
    }

    #[tracing::instrument(skip(self, _ctx), fields(resolver = "mdpi", input = %input))]
    async fn resolve(
        &self,
        input: &str,
        _ctx: &ResolveContext,
    ) -> Result<ResolveStep, ResolveError> {
        let trimmed = input.trim();

        if looks_like_doi(trimmed, MDPI_DOI_PREFIX) {
            self.resolve_doi(trimmed).await
        } else {
            self.resolve_url(trimmed).await
        }
    }
}

impl MdpiResolver {
    async fn resolve_doi(&self, doi: &str) -> Result<ResolveStep, ResolveError> {
        let Some(doi_slug) = extract_slug_from_doi(doi) else {
            return Ok(ResolveStep::Failed(ResolveError::resolution_failed(
                doi,
                "Could not extract journal slug from MDPI DOI. \
                 Why: DOI suffix after 10.3390/ has no alphabetic prefix. \
                 Fix: verify the DOI is a valid MDPI article DOI.",
            )));
        };

        debug!(doi = %doi, doi_slug = %doi_slug, "Extracted MDPI journal slug from DOI");

        let encoded_doi = urlencoding::encode(doi);
        let encoded_mailto = urlencoding::encode(&self.crossref_mailto);
        let url = format!(
            "{}/works/{}?mailto={}",
            self.crossref_base_url, encoded_doi, encoded_mailto
        );

        debug!(api_url = %url, "Calling Crossref API for MDPI DOI");

        let response = match self.client.get(&url).send().await {
            Ok(resp) => resp,
            Err(e) => {
                warn!(error = %e, "Crossref API request failed for MDPI DOI");
                return Ok(ResolveStep::Failed(ResolveError::resolution_failed(
                    doi,
                    "Cannot reach Crossref API. Check your network connection.",
                )));
            }
        };

        let status = response.status();
        if !status.is_success() {
            let reason = crossref_error_reason(status.as_u16());
            debug!(status = status.as_u16(), %reason, "Crossref API error for MDPI DOI");
            return Ok(ResolveStep::Failed(ResolveError::resolution_failed(
                doi, &reason,
            )));
        }

        let body = match response.json::<CrossrefResponse>().await {
            Ok(parsed) => parsed,
            Err(e) => {
                warn!(error = %e, "Failed to parse Crossref response for MDPI DOI");
                return Ok(ResolveStep::body_parse_failed(doi, "Crossref"));
            }
        };

        if !body.status.eq_ignore_ascii_case("ok") {
            warn!(status = %body.status, "Crossref response status was not ok for MDPI DOI");
            return Ok(ResolveStep::Failed(ResolveError::resolution_failed(
                doi,
                "Unexpected Crossref response status",
            )));
        }

        let msg = &body.message;

        let Some(volume) = msg.volume.as_deref() else {
            return Ok(ResolveStep::Failed(ResolveError::resolution_failed(
                doi,
                "Crossref response missing volume for MDPI article. \
                 Why: Crossref metadata incomplete for this DOI. \
                 Fix: try the DOI URL directly in a browser.",
            )));
        };

        let Some(issue) = msg.issue.as_deref() else {
            return Ok(ResolveStep::Failed(ResolveError::resolution_failed(
                doi,
                "Crossref response missing issue for MDPI article. \
                 Why: Crossref metadata incomplete for this DOI. \
                 Fix: try the DOI URL directly in a browser.",
            )));
        };

        // doi_slug is used here for article-number parsing — it MUST remain the DOI slug,
        // not the CDN slug, because the DOI suffix encodes the abbreviated slug.
        let Some(article_number) = extract_article_from_doi_suffix(doi, &doi_slug, volume, issue)
        else {
            return Ok(ResolveStep::Failed(ResolveError::resolution_failed(
                doi,
                "Could not derive article number from MDPI DOI suffix. \
                 Why: DOI suffix does not match expected {slug}{volume}{issue}{article} pattern. \
                 Fix: verify the DOI is a valid MDPI article DOI.",
            )));
        };

        let cdn_slug = self.resolve_cdn_slug(msg.container_title_str(), &doi_slug).await;

        let cdn_url = build_cdn_url(&cdn_slug, volume, &article_number);
        let metadata = extract_metadata(msg, doi, doi);

        debug!(cdn_url = %cdn_url, cdn_slug = %cdn_slug, "Constructed MDPI CDN URL");
        Ok(ResolveStep::Url(ResolvedUrl::with_metadata(
            cdn_url, metadata,
        )))
    }

    async fn resolve_url(&self, input: &str) -> Result<ResolveStep, ResolveError> {
        let Some(parts) = parse_mdpi_url(input) else {
            return Ok(ResolveStep::Failed(ResolveError::resolution_failed(
                input,
                "Could not parse MDPI article URL. \
                 Why: URL path does not match expected /{ISSN}/{volume}/{issue}/{article} pattern. \
                 Fix: use a standard MDPI article URL or provide the DOI instead.",
            )));
        };

        debug!(
            issn = %parts.issn,
            volume = %parts.volume,
            article = %parts.article,
            "Parsed MDPI URL, searching Crossref by ISSN"
        );

        let encoded_mailto = urlencoding::encode(&self.crossref_mailto);
        let encoded_issn = urlencoding::encode(&parts.issn);
        let encoded_article = urlencoding::encode(&parts.article);
        let url = format!(
            "{}/works?filter=issn:{}&query={}&rows=5&mailto={}",
            self.crossref_base_url, encoded_issn, encoded_article, encoded_mailto
        );

        debug!(api_url = %url, "Calling Crossref search API for MDPI URL");

        let response = match self.client.get(&url).send().await {
            Ok(resp) => resp,
            Err(e) => {
                warn!(error = %e, "Crossref search API request failed for MDPI URL");
                return Ok(ResolveStep::Failed(ResolveError::resolution_failed(
                    input,
                    "Cannot reach Crossref API. Check your network connection.",
                )));
            }
        };

        let status = response.status();
        if !status.is_success() {
            let reason = crossref_error_reason(status.as_u16());
            debug!(status = status.as_u16(), %reason, "Crossref search API error");
            return Ok(ResolveStep::Failed(ResolveError::resolution_failed(
                input, &reason,
            )));
        }

        let body = match response.json::<CrossrefSearchResponse>().await {
            Ok(parsed) => parsed,
            Err(e) => {
                warn!(error = %e, "Failed to parse Crossref search response");
                return Ok(ResolveStep::body_parse_failed(input, "Crossref search"));
            }
        };

        if !body.status.eq_ignore_ascii_case("ok") {
            warn!(status = %body.status, "Crossref search response status was not ok");
            return Ok(ResolveStep::Failed(ResolveError::resolution_failed(
                input,
                "Unexpected Crossref search response status",
            )));
        }

        let items = body.message.items.as_deref().unwrap_or(&[]);
        let matched = items.iter().find(|item| {
            item.volume.as_deref() == Some(&parts.volume)
                && item.issue.as_deref() == Some(&parts.issue)
        });

        let Some(item) = matched else {
            debug!("No matching article found in Crossref search results");
            return Ok(ResolveStep::Failed(ResolveError::resolution_failed(
                input,
                "Could not find matching MDPI article in Crossref. \
                 Why: no Crossref result matched the volume and issue from the URL. \
                 Fix: verify the URL is correct, or provide the DOI instead.",
            )));
        };

        let item_doi = item.doi.as_deref().unwrap_or("");
        let Some(doi_slug) = extract_slug_from_doi(item_doi) else {
            return Ok(ResolveStep::Failed(ResolveError::resolution_failed(
                input,
                "Could not extract journal slug from Crossref DOI. \
                 Why: the DOI returned by Crossref does not follow the MDPI 10.3390/{slug} pattern. \
                 Fix: provide the DOI directly instead of the URL.",
            )));
        };

        let cdn_slug = self.resolve_cdn_slug(item.container_title_str(), &doi_slug).await;

        let cdn_url = build_cdn_url(&cdn_slug, &parts.volume, &parts.article);
        let metadata = extract_metadata(item, item_doi, input);

        debug!(cdn_url = %cdn_url, cdn_slug = %cdn_slug, "Constructed MDPI CDN URL from search");
        Ok(ResolveStep::Url(ResolvedUrl::with_metadata(
            cdn_url, metadata,
        )))
    }

    /// Resolves the CDN path slug for an MDPI journal.
    ///
    /// The CDN slug (e.g., `"sensors"`, `"applsci"`) differs from the DOI slug
    /// (e.g., `"s"`, `"app"`) for many journals. Strategy:
    ///
    /// 1. **Single-word `container_title`** — lowercased title is always the CDN slug.
    /// 2. **Multi-word `container_title`** — probe candidates via HEAD request:
    ///    a. lowercase + strip non-alpha (e.g., `"Marine Drugs"` → `"marinedrugs"`)
    ///    b. DOI slug fallback (e.g., `"jof"`, `"ijms"`)
    ///    c. Known override (e.g., `"Applied Sciences"` → `"applsci"`)
    /// 3. **No `container_title`** — DOI slug (preserves prior behaviour).
    async fn resolve_cdn_slug(&self, container_title: Option<&str>, doi_slug: &str) -> String {
        let mut candidates = cdn_slug_candidates(container_title, doi_slug);

        // Single candidate means single-word title or no title — use directly, no probe needed.
        if candidates.len() == 1 {
            return candidates.remove(0);
        }

        // Multi-word journals: probe each candidate via HEAD.
        debug!(
            ?candidates,
            "Probing MDPI CDN slug candidates for multi-word journal"
        );

        for candidate in &candidates {
            let probe_url = format!("{MDPI_CDN_BASE}/{candidate}/");
            match self.client.head(&probe_url).timeout(CDN_PROBE_TIMEOUT).send().await {
                Ok(resp) if resp.status().is_success() || resp.status().as_u16() == 403 => {
                    // 200 or 403 both indicate the path exists on the CDN.
                    debug!(cdn_slug = %candidate, "MDPI CDN slug confirmed via HEAD probe");
                    return candidate.clone();
                }
                Ok(_) => {}
                Err(e) => {
                    debug!(error = %e, candidate = %candidate, "HEAD probe error; trying next");
                }
            }
        }

        // All probes failed — use first candidate and let the download surface the 404.
        warn!(
            doi_slug = %doi_slug,
            ?candidates,
            "No MDPI CDN slug candidate confirmed via HEAD probe. \
             Why: journal CDN path may use an unknown abbreviation. \
             Fix: report this DOI at the project issue tracker for an override table update."
        );
        candidates.remove(0)
    }
}

// ==================== URL / DOI Parsing ====================

/// Parsed components from an MDPI article URL path.
#[derive(Debug, PartialEq)]
struct MdpiUrlParts {
    issn: String,
    volume: String,
    issue: String,
    article: String,
}

/// Returns true if `path` looks like an MDPI article path (`/{ISSN}/{vol}/{issue}/{article}[/pdf]`).
fn is_mdpi_article_path(path: &str) -> bool {
    let path = path.trim_end_matches('/');
    let path = path.strip_suffix("/pdf").unwrap_or(path);
    let segments: Vec<&str> = path.split('/').filter(|s| !s.is_empty()).collect();
    segments.len() == 4
        && looks_like_issn(segments[0])
        && segments[1].chars().all(|c| c.is_ascii_digit())
        && segments[2].chars().all(|c| c.is_ascii_digit())
        && segments[3].chars().all(|c| c.is_ascii_digit())
}

/// Returns true if `value` matches the ISSN pattern `DDDD-DDDD`.
fn looks_like_issn(value: &str) -> bool {
    let bytes = value.as_bytes();
    bytes.len() == 9
        && bytes[4] == b'-'
        && bytes[..4].iter().all(u8::is_ascii_digit)
        && bytes[5..].iter().all(u8::is_ascii_digit)
}

/// Parses an MDPI article URL into its component parts.
fn parse_mdpi_url(input: &str) -> Option<MdpiUrlParts> {
    let url = Url::parse(input).ok()?;
    let path = url.path();
    let path = path.trim_end_matches('/');
    let path = path.strip_suffix("/pdf").unwrap_or(path);
    let segments: Vec<&str> = path.split('/').filter(|s| !s.is_empty()).collect();

    if segments.len() != 4
        || !looks_like_issn(segments[0])
        || !segments[1].chars().all(|c| c.is_ascii_digit())
        || !segments[2].chars().all(|c| c.is_ascii_digit())
        || !segments[3].chars().all(|c| c.is_ascii_digit())
    {
        return None;
    }

    Some(MdpiUrlParts {
        issn: segments[0].to_string(),
        volume: segments[1].to_string(),
        issue: segments[2].to_string(),
        article: segments[3].to_string(),
    })
}

/// Extracts the journal slug from an MDPI DOI suffix.
///
/// For `10.3390/electronics13132567`, returns `Some("electronics")`.
/// The slug is the leading alphabetic portion after the `10.3390/` prefix.
fn extract_slug_from_doi(doi: &str) -> Option<String> {
    let suffix = doi
        .trim()
        .to_ascii_lowercase()
        .strip_prefix("10.3390/")
        .map(str::to_string)?;

    let slug: String = suffix.chars().take_while(char::is_ascii_alphabetic).collect();

    if slug.is_empty() {
        None
    } else {
        Some(slug)
    }
}

/// Extracts the article number from an MDPI DOI suffix by stripping slug, volume, and issue.
///
/// For DOI `10.3390/electronics13132567` with slug=`electronics`, volume=`13`, issue=`13`:
/// suffix after prefix = `electronics13132567` → strip slug → `13132567` → strip volume →
/// `132567` → strip issue → `2567`.
fn extract_article_from_doi_suffix(
    doi: &str,
    slug: &str,
    volume: &str,
    issue: &str,
) -> Option<String> {
    let suffix = doi.trim().to_ascii_lowercase();
    let suffix = suffix.strip_prefix("10.3390/")?;
    let remainder = suffix.strip_prefix(slug)?;
    let remainder = remainder.strip_prefix(volume)?;

    // MDPI DOIs may zero-pad the issue number (e.g., issue "1" → "01" in DOI).
    // Try exact match first, then zero-padded to 2 digits.
    let remainder = if let Some(r) = remainder.strip_prefix(issue) {
        r
    } else {
        let padded = format!("{issue:0>2}");
        remainder.strip_prefix(padded.as_str())?
    };

    if remainder.is_empty() || !remainder.chars().all(|c| c.is_ascii_digit()) {
        return None;
    }

    Some(remainder.to_string())
}

// ==================== CDN URL Construction ====================

/// Constructs the `mdpi-res.com` CDN download URL for an MDPI article.
fn build_cdn_url(slug: &str, volume: &str, article_number: &str) -> String {
    let article_padded = format!("{article_number:0>5}");
    let file_stem = format!("{slug}-{volume}-{article_padded}");
    format!("{MDPI_CDN_BASE}/{slug}/{file_stem}/article_deploy/{file_stem}.pdf")
}

/// Returns ordered CDN slug candidates for a journal.
///
/// Single-word container-titles always resolve deterministically (the lowercased title).
/// Multi-word titles generate up to three candidates in priority order:
/// 1. Lowercase, non-alpha stripped (e.g., `"Marine Drugs"` → `"marinedrugs"`)
/// 2. DOI slug (e.g., `"jof"` for Journal of Fungi)
/// 3. Known override from [`MULTI_WORD_CDN_OVERRIDES`] (e.g., `"applsci"`)
///
/// Callers probe these candidates via HEAD to find which one the CDN accepts.
fn cdn_slug_candidates(container_title: Option<&str>, doi_slug: &str) -> Vec<String> {
    let Some(title) = container_title else {
        return vec![doi_slug.to_string()];
    };

    let title_lower = title.to_ascii_lowercase();

    // Single-word: lowercased title is always the correct CDN slug.
    if !title_lower.contains(' ') {
        return vec![title_lower];
    }

    // Multi-word: build candidates in priority order.
    let mut candidates: Vec<String> = Vec::new();

    let stripped: String = title_lower.chars().filter(char::is_ascii_alphabetic).collect();
    candidates.push(stripped);

    let doi = doi_slug.to_string();
    if !candidates.contains(&doi) {
        candidates.push(doi);
    }

    if let Some(&(_, override_slug)) = MULTI_WORD_CDN_OVERRIDES
        .iter()
        .find(|&&(name, _)| name == title_lower)
    {
        let owned = override_slug.to_string();
        if !candidates.contains(&owned) {
            candidates.push(owned);
        }
    }

    candidates
}

// ==================== Metadata Extraction ====================

/// Extracts metadata using the shared Crossref helper, then adds `source_url`.
fn extract_metadata(
    message: &CrossrefMessage,
    doi: &str,
    source_url: &str,
) -> HashMap<String, String> {
    let mut metadata = crossref_extract_metadata(message, doi);
    metadata.insert("source_url".to_string(), source_url.to_string());
    metadata
}

// ==================== Helpers ====================

fn crossref_error_reason(status: u16) -> String {
    match status {
        404 => "DOI not found in Crossref database".to_string(),
        429 => "Crossref rate limit exceeded. Try again in a few seconds.".to_string(),
        s if s >= 500 => "Crossref API unavailable. Try again later.".to_string(),
        s => format!("Crossref API returned HTTP {s}"),
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use crate::test_support::socket_guard::start_mock_server_or_skip;
    use wiremock::matchers::{method, path, path_regex, query_param};
    use wiremock::{Mock, ResponseTemplate};

    // ==================== Name & Priority ====================

    #[test]
    fn test_name_and_priority() {
        let resolver = MdpiResolver::new("test@example.com").unwrap();
        assert_eq!(resolver.name(), "mdpi");
        assert_eq!(resolver.priority(), ResolverPriority::Specialized);
    }

    // ==================== can_handle ====================

    #[test]
    fn test_can_handle_mdpi_doi() {
        let resolver = MdpiResolver::new("test@example.com").unwrap();
        assert!(resolver.can_handle("10.3390/electronics13132567", InputType::Doi));
        assert!(resolver.can_handle("10.3390/s24010123", InputType::Doi));
        assert!(resolver.can_handle("10.3390/ijerph2400001", InputType::Doi));
    }

    #[test]
    fn test_cannot_handle_non_mdpi_doi() {
        let resolver = MdpiResolver::new("test@example.com").unwrap();
        assert!(!resolver.can_handle("10.1109/5.771073", InputType::Doi));
        assert!(!resolver.can_handle("10.1007/s00521-024", InputType::Doi));
    }

    #[test]
    fn test_can_handle_mdpi_url() {
        let resolver = MdpiResolver::new("test@example.com").unwrap();
        assert!(resolver.can_handle(
            "https://www.mdpi.com/2079-9292/13/13/2567",
            InputType::Url
        ));
        assert!(resolver.can_handle(
            "https://mdpi.com/2079-9292/13/13/2567",
            InputType::Url
        ));
    }

    #[test]
    fn test_can_handle_mdpi_url_with_pdf_suffix() {
        let resolver = MdpiResolver::new("test@example.com").unwrap();
        assert!(resolver.can_handle(
            "https://www.mdpi.com/2079-9292/13/13/2567/pdf",
            InputType::Url
        ));
    }

    #[test]
    fn test_cannot_handle_non_article_mdpi_url() {
        let resolver = MdpiResolver::new("test@example.com").unwrap();
        assert!(!resolver.can_handle(
            "https://www.mdpi.com/journal/electronics",
            InputType::Url
        ));
        assert!(!resolver.can_handle(
            "https://www.mdpi.com/about",
            InputType::Url
        ));
        assert!(!resolver.can_handle(
            "https://www.mdpi.com/search?q=test",
            InputType::Url
        ));
    }

    #[test]
    fn test_cannot_handle_mdpi_res_url() {
        let resolver = MdpiResolver::new("test@example.com").unwrap();
        assert!(!resolver.can_handle(
            "https://mdpi-res.com/d_attachment/electronics/electronics-13-02567/article_deploy/electronics-13-02567.pdf",
            InputType::Url
        ));
    }

    // ==================== URL / DOI Parsing ====================

    #[test]
    fn test_parse_mdpi_url_standard() {
        let parts = parse_mdpi_url("https://www.mdpi.com/2079-9292/13/13/2567").unwrap();
        assert_eq!(parts, MdpiUrlParts {
            issn: "2079-9292".to_string(),
            volume: "13".to_string(),
            issue: "13".to_string(),
            article: "2567".to_string(),
        });
    }

    #[test]
    fn test_parse_mdpi_url_with_pdf_suffix() {
        let parts = parse_mdpi_url("https://www.mdpi.com/2079-9292/13/13/2567/pdf").unwrap();
        assert_eq!(parts.issn, "2079-9292");
        assert_eq!(parts.article, "2567");
    }

    #[test]
    fn test_parse_mdpi_url_rejects_non_article_path() {
        assert!(parse_mdpi_url("https://www.mdpi.com/journal/electronics").is_none());
        assert!(parse_mdpi_url("https://www.mdpi.com/about").is_none());
    }

    #[test]
    fn test_extract_slug_from_doi_standard() {
        assert_eq!(
            extract_slug_from_doi("10.3390/electronics13132567"),
            Some("electronics".to_string())
        );
    }

    #[test]
    fn test_extract_slug_from_doi_single_letter() {
        assert_eq!(
            extract_slug_from_doi("10.3390/s24010123"),
            Some("s".to_string())
        );
    }

    #[test]
    fn test_extract_slug_from_doi_multi_char() {
        assert_eq!(
            extract_slug_from_doi("10.3390/ijerph2400001"),
            Some("ijerph".to_string())
        );
    }

    #[test]
    fn test_extract_slug_from_doi_non_mdpi() {
        assert_eq!(extract_slug_from_doi("10.1109/foo"), None);
    }

    // ==================== Article number from DOI suffix ====================

    #[test]
    fn test_extract_article_from_doi_suffix_standard() {
        assert_eq!(
            extract_article_from_doi_suffix("10.3390/electronics13132567", "electronics", "13", "13"),
            Some("2567".to_string())
        );
    }

    #[test]
    fn test_extract_article_from_doi_suffix_single_letter_slug() {
        assert_eq!(
            extract_article_from_doi_suffix("10.3390/s24010123", "s", "24", "1"),
            Some("0123".to_string())
        );
    }

    #[test]
    fn test_extract_article_from_doi_suffix_sustainability() {
        assert_eq!(
            extract_article_from_doi_suffix("10.3390/su16010001", "su", "16", "1"),
            Some("0001".to_string())
        );
    }

    #[test]
    fn test_extract_article_from_doi_suffix_mismatch() {
        // Wrong volume — should fail
        assert_eq!(
            extract_article_from_doi_suffix("10.3390/electronics13132567", "electronics", "14", "13"),
            None
        );
    }

    #[test]
    fn test_extract_article_from_doi_suffix_non_mdpi_doi() {
        assert_eq!(
            extract_article_from_doi_suffix("10.1109/foo123", "foo", "1", "2"),
            None
        );
    }

    // ==================== CDN URL Construction ====================

    #[test]
    fn test_build_cdn_url_standard() {
        assert_eq!(
            build_cdn_url("electronics", "13", "2567"),
            "https://mdpi-res.com/d_attachment/electronics/electronics-13-02567/article_deploy/electronics-13-02567.pdf"
        );
    }

    #[test]
    fn test_build_cdn_url_short_article_number() {
        assert_eq!(
            build_cdn_url("sensors", "24", "123"),
            "https://mdpi-res.com/d_attachment/sensors/sensors-24-00123/article_deploy/sensors-24-00123.pdf"
        );
    }

    #[test]
    fn test_build_cdn_url_five_digit_article() {
        assert_eq!(
            build_cdn_url("molecules", "29", "12345"),
            "https://mdpi-res.com/d_attachment/molecules/molecules-29-12345/article_deploy/molecules-29-12345.pdf"
        );
    }

    // ==================== ISSN validation ====================

    #[test]
    fn test_looks_like_issn() {
        assert!(looks_like_issn("2079-9292"));
        assert!(looks_like_issn("1660-4601"));
        assert!(!looks_like_issn("electronics"));
        assert!(!looks_like_issn("2079929"));
        assert!(!looks_like_issn("20799-292"));
    }

    // ==================== Mailto validation ====================

    #[test]
    fn test_rejects_invalid_mailto() {
        let result = MdpiResolver::new("invalid\nmailto@example.com");
        assert!(result.is_err());
    }

    // ==================== Resolver Integration (wiremock) ====================

    fn crossref_mdpi_works_response() -> serde_json::Value {
        // Matches real Crossref behavior: no article-number field (always absent for MDPI).
        // container-title is a single word, so CDN slug = lowercase(title) = "electronics".
        serde_json::json!({
            "status": "ok",
            "message": {
                "DOI": "10.3390/electronics13132567",
                "title": ["Smart Home IoT Sensors"],
                "author": [
                    {"given": "Alice", "family": "Smith"},
                    {"given": "Bob", "family": "Jones"}
                ],
                "container-title": ["Electronics"],
                "volume": "13",
                "issue": "13",
                "published": {"date-parts": [[2024, 7, 1]]}
            }
        })
    }

    fn crossref_mdpi_search_response() -> serde_json::Value {
        // Matches real Crossref behavior: no article-number, issue present.
        // container-title single word → CDN slug = "electronics".
        serde_json::json!({
            "status": "ok",
            "message": {
                "items": [{
                    "DOI": "10.3390/electronics13132567",
                    "title": ["Smart Home IoT Sensors"],
                    "author": [{"given": "Alice", "family": "Smith"}],
                    "container-title": ["Electronics"],
                    "volume": "13",
                    "issue": "13",
                    "published": {"date-parts": [[2024, 7, 1]]}
                }]
            }
        })
    }

    #[tokio::test]
    async fn test_resolve_doi_success() {
        let Some(mock_server) = start_mock_server_or_skip().await else {
            return;
        };

        Mock::given(method("GET"))
            .and(path_regex(r"/works/10\..+"))
            .respond_with(
                ResponseTemplate::new(200).set_body_json(crossref_mdpi_works_response()),
            )
            .mount(&mock_server)
            .await;

        let resolver =
            MdpiResolver::with_crossref_base_url("test@example.com", mock_server.uri()).unwrap();
        let ctx = ResolveContext::default();
        let result = resolver
            .resolve("10.3390/electronics13132567", &ctx)
            .await
            .unwrap();

        match result {
            ResolveStep::Url(resolved) => {
                assert_eq!(
                    resolved.url,
                    "https://mdpi-res.com/d_attachment/electronics/electronics-13-02567/article_deploy/electronics-13-02567.pdf"
                );
                assert_eq!(resolved.metadata.get("title").unwrap(), "Smart Home IoT Sensors");
                assert_eq!(resolved.metadata.get("authors").unwrap(), "Smith, Alice; Jones, Bob");
                assert_eq!(resolved.metadata.get("year").unwrap(), "2024");
                assert_eq!(resolved.metadata.get("doi").unwrap(), "10.3390/electronics13132567");
            }
            other => panic!("Expected ResolveStep::Url, got: {other:?}"),
        }
    }

    #[tokio::test]
    async fn test_resolve_url_success() {
        let Some(mock_server) = start_mock_server_or_skip().await else {
            return;
        };

        Mock::given(method("GET"))
            .and(path("/works"))
            .and(query_param("filter", "issn:2079-9292"))
            .respond_with(
                ResponseTemplate::new(200).set_body_json(crossref_mdpi_search_response()),
            )
            .mount(&mock_server)
            .await;

        let resolver =
            MdpiResolver::with_crossref_base_url("test@example.com", mock_server.uri()).unwrap();
        let ctx = ResolveContext::default();
        let result = resolver
            .resolve("https://www.mdpi.com/2079-9292/13/13/2567", &ctx)
            .await
            .unwrap();

        match result {
            ResolveStep::Url(resolved) => {
                assert_eq!(
                    resolved.url,
                    "https://mdpi-res.com/d_attachment/electronics/electronics-13-02567/article_deploy/electronics-13-02567.pdf"
                );
                assert_eq!(resolved.metadata.get("title").unwrap(), "Smart Home IoT Sensors");
            }
            other => panic!("Expected ResolveStep::Url, got: {other:?}"),
        }
    }

    #[tokio::test]
    async fn test_resolve_doi_crossref_404() {
        let Some(mock_server) = start_mock_server_or_skip().await else {
            return;
        };

        Mock::given(method("GET"))
            .and(path_regex(r"/works/10\..+"))
            .respond_with(ResponseTemplate::new(404))
            .mount(&mock_server)
            .await;

        let resolver =
            MdpiResolver::with_crossref_base_url("test@example.com", mock_server.uri()).unwrap();
        let ctx = ResolveContext::default();
        let result = resolver
            .resolve("10.3390/electronics99990001", &ctx)
            .await
            .unwrap();

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
    async fn test_resolve_doi_missing_volume() {
        let Some(mock_server) = start_mock_server_or_skip().await else {
            return;
        };

        Mock::given(method("GET"))
            .and(path_regex(r"/works/10\..+"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "status": "ok",
                "message": {
                    "DOI": "10.3390/electronics13132567",
                    "title": ["Test"],
                    "issue": "13"
                }
            })))
            .mount(&mock_server)
            .await;

        let resolver =
            MdpiResolver::with_crossref_base_url("test@example.com", mock_server.uri()).unwrap();
        let ctx = ResolveContext::default();
        let result = resolver
            .resolve("10.3390/electronics13132567", &ctx)
            .await
            .unwrap();

        assert!(
            matches!(result, ResolveStep::Failed(_)),
            "Should fail when volume is missing"
        );
    }

    #[tokio::test]
    async fn test_resolve_url_no_matching_item() {
        let Some(mock_server) = start_mock_server_or_skip().await else {
            return;
        };

        // Return an item with a different issue so it won't match the URL's issue=13
        Mock::given(method("GET"))
            .and(path("/works"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "status": "ok",
                "message": {
                    "items": [{
                        "DOI": "10.3390/electronics13010001",
                        "volume": "13",
                        "issue": "1"
                    }]
                }
            })))
            .mount(&mock_server)
            .await;

        let resolver =
            MdpiResolver::with_crossref_base_url("test@example.com", mock_server.uri()).unwrap();
        let ctx = ResolveContext::default();
        let result = resolver
            .resolve("https://www.mdpi.com/2079-9292/13/13/9999", &ctx)
            .await
            .unwrap();

        assert!(
            matches!(result, ResolveStep::Failed(_)),
            "Should fail when no matching article found"
        );
    }

    #[tokio::test]
    async fn test_resolve_doi_sends_mailto() {
        let Some(mock_server) = start_mock_server_or_skip().await else {
            return;
        };

        Mock::given(method("GET"))
            .and(path_regex(r"/works/10\..+"))
            .and(query_param("mailto", "test@example.com"))
            .respond_with(
                ResponseTemplate::new(200).set_body_json(crossref_mdpi_works_response()),
            )
            .mount(&mock_server)
            .await;

        let resolver =
            MdpiResolver::with_crossref_base_url("test@example.com", mock_server.uri()).unwrap();
        let ctx = ResolveContext::default();
        let result = resolver
            .resolve("10.3390/electronics13132567", &ctx)
            .await
            .unwrap();

        assert!(
            matches!(result, ResolveStep::Url(_)),
            "Should succeed when mailto param is present"
        );
    }

    // ==================== cdn_slug_candidates ====================

    #[test]
    fn test_cdn_slug_candidates_single_word_uses_title() {
        // Single-word title → lowercase title, ignores doi_slug.
        assert_eq!(cdn_slug_candidates(Some("Sensors"), "s"), vec!["sensors"]);
        assert_eq!(cdn_slug_candidates(Some("Electronics"), "electronics"), vec!["electronics"]);
        assert_eq!(cdn_slug_candidates(Some("Entropy"), "e"), vec!["entropy"]);
        assert_eq!(cdn_slug_candidates(Some("Materials"), "ma"), vec!["materials"]);
    }

    #[test]
    fn test_cdn_slug_candidates_no_container_title_uses_doi_slug() {
        assert_eq!(cdn_slug_candidates(None, "s"), vec!["s"]);
        assert_eq!(cdn_slug_candidates(None, "electronics"), vec!["electronics"]);
    }

    #[test]
    fn test_cdn_slug_candidates_multi_word_includes_concatenated_and_doi_slug() {
        // "Marine Drugs" → concatenated "marinedrugs" + DOI slug "md" (no override).
        let candidates = cdn_slug_candidates(Some("Marine Drugs"), "md");
        assert_eq!(candidates[0], "marinedrugs", "first candidate must be concatenated");
        assert!(candidates.contains(&"md".to_string()), "doi slug must be a candidate");
    }

    #[test]
    fn test_cdn_slug_candidates_multi_word_with_override() {
        // "Applied Sciences" → "appliedsciences" + "app" (DOI slug) + "applsci" (override).
        let candidates = cdn_slug_candidates(Some("Applied Sciences"), "app");
        assert_eq!(candidates[0], "appliedsciences");
        assert!(candidates.contains(&"app".to_string()));
        assert!(candidates.contains(&"applsci".to_string()));
        assert_eq!(
            candidates.last().unwrap(),
            "applsci",
            "override must be last candidate"
        );
    }

    #[test]
    fn test_cdn_slug_candidates_deduplicates() {
        // When doi_slug matches the concatenated form, no duplicate.
        let candidates = cdn_slug_candidates(Some("Electronics"), "electronics");
        assert_eq!(candidates.len(), 1);
    }

    // ==================== Resolver Integration — CDN slug fix ====================

    #[tokio::test]
    async fn test_resolve_doi_single_letter_slug_uses_container_title() {
        // Regression test: DOI slug "s" must NOT appear in the CDN URL.
        // container-title "Sensors" (single word) → CDN slug "sensors".
        let Some(mock_server) = start_mock_server_or_skip().await else {
            return;
        };

        Mock::given(method("GET"))
            .and(path_regex(r"/works/10\..+"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "status": "ok",
                "message": {
                    "DOI": "10.3390/s24010123",
                    "title": ["Sensor Network Study"],
                    "author": [{"given": "Alice", "family": "Smith"}],
                    "container-title": ["Sensors"],
                    "volume": "24",
                    "issue": "1",
                    "published": {"date-parts": [[2024, 1, 1]]}
                }
            })))
            .mount(&mock_server)
            .await;

        let resolver =
            MdpiResolver::with_crossref_base_url("test@example.com", mock_server.uri()).unwrap();
        let ctx = ResolveContext::default();
        let result = resolver.resolve("10.3390/s24010123", &ctx).await.unwrap();

        match result {
            ResolveStep::Url(resolved) => {
                assert_eq!(
                    resolved.url,
                    "https://mdpi-res.com/d_attachment/sensors/sensors-24-00123/article_deploy/sensors-24-00123.pdf",
                    "CDN URL must use 'sensors', not 's'"
                );
            }
            other => panic!("Expected ResolveStep::Url, got: {other:?}"),
        }
    }

    #[tokio::test]
    async fn test_resolve_url_single_word_container_title_used_for_cdn_slug() {
        // Regression test for resolve_url path: container-title "Sensors" → CDN slug "sensors".
        let Some(mock_server) = start_mock_server_or_skip().await else {
            return;
        };

        Mock::given(method("GET"))
            .and(path("/works"))
            .and(query_param("filter", "issn:1424-8220"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "status": "ok",
                "message": {
                    "items": [{
                        "DOI": "10.3390/s24010123",
                        "title": ["Sensor Network Study"],
                        "author": [{"given": "Alice", "family": "Smith"}],
                        "container-title": ["Sensors"],
                        "volume": "24",
                        "issue": "1",
                        "published": {"date-parts": [[2024, 1, 1]]}
                    }]
                }
            })))
            .mount(&mock_server)
            .await;

        let resolver =
            MdpiResolver::with_crossref_base_url("test@example.com", mock_server.uri()).unwrap();
        let ctx = ResolveContext::default();
        let result = resolver
            .resolve("https://www.mdpi.com/1424-8220/24/1/123", &ctx)
            .await
            .unwrap();

        match result {
            ResolveStep::Url(resolved) => {
                assert_eq!(
                    resolved.url,
                    "https://mdpi-res.com/d_attachment/sensors/sensors-24-00123/article_deploy/sensors-24-00123.pdf",
                    "CDN URL must use 'sensors', not 's'"
                );
            }
            other => panic!("Expected ResolveStep::Url, got: {other:?}"),
        }
    }

    #[tokio::test]
    async fn test_resolve_doi_no_container_title_falls_back_to_doi_slug() {
        // When Crossref omits container-title, the DOI slug is used (legacy behaviour preserved).
        let Some(mock_server) = start_mock_server_or_skip().await else {
            return;
        };

        Mock::given(method("GET"))
            .and(path_regex(r"/works/10\..+"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "status": "ok",
                "message": {
                    "DOI": "10.3390/electronics13132567",
                    "title": ["Test Article"],
                    "volume": "13",
                    "issue": "13"
                    // container-title intentionally absent
                }
            })))
            .mount(&mock_server)
            .await;

        let resolver =
            MdpiResolver::with_crossref_base_url("test@example.com", mock_server.uri()).unwrap();
        let ctx = ResolveContext::default();
        let result = resolver
            .resolve("10.3390/electronics13132567", &ctx)
            .await
            .unwrap();

        match result {
            ResolveStep::Url(resolved) => {
                assert!(
                    resolved.url.contains("/electronics/"),
                    "Should fall back to DOI slug 'electronics': {}",
                    resolved.url
                );
            }
            other => panic!("Expected ResolveStep::Url, got: {other:?}"),
        }
    }
}
