//! Shared utilities for resolver modules: host normalization, DOI prefix checks, and common regexes.

use std::sync::LazyLock;

use regex::Regex;
use url::Url;

use super::AuthRequirement;

/// Compiles a regex at static init; panics on invalid pattern.
pub fn compile_static_regex(pattern: &str) -> Regex {
    Regex::new(pattern).unwrap_or_else(|e| panic!("invalid static regex '{pattern}': {e}"))
}

/// Shared regex for extracting `citation_pdf_url` from HTML meta tags.
/// Used by `PubMed`, `IEEE`, and `Springer` resolvers.
pub static CITATION_PDF_RE: LazyLock<Regex> = LazyLock::new(|| {
    compile_static_regex(
        r#"(?is)<meta\s+[^>]*(?:name|property)\s*=\s*["']citation_pdf_url["'][^>]*content\s*=\s*["']([^"']+)["']"#,
    )
});

/// Normalizes a host string: trim, strip leading "www.", trailing '.', and lowercases.
#[must_use]
pub fn canonical_host(host: &str) -> String {
    host.trim()
        .trim_start_matches("www.")
        .trim_end_matches('.')
        .to_ascii_lowercase()
}

/// Parses `url_or_host` as a URL and returns the host, or normalizes it as a bare host string.
#[must_use]
pub fn parse_host_or_fallback(url_or_host: &str) -> String {
    Url::parse(url_or_host)
        .ok()
        .and_then(|url| url.host_str().map(std::string::ToString::to_string))
        .unwrap_or_else(|| canonical_host(url_or_host))
}

/// Returns true if the two host strings refer to the same host after normalization.
#[must_use]
pub fn hosts_match(lhs: &str, rhs: &str) -> bool {
    canonical_host(lhs) == canonical_host(rhs)
}

/// Returns true if `value` (after trim and lowercasing) starts with the given DOI prefix.
///
/// Both `value` and `prefix` are normalized: trimmed and converted to ASCII lowercase before
/// comparison, so callers may pass mixed-case input. For consistency with DOI display conventions,
/// prefer lowercase prefixes with a trailing slash (e.g. `"10.1109/"`).
#[must_use]
pub fn looks_like_doi(value: &str, prefix: &str) -> bool {
    let value_norm = value.trim().to_ascii_lowercase();
    let prefix_norm = prefix.trim().to_ascii_lowercase();
    value_norm.starts_with(prefix_norm.as_str())
}

/// Resolves a possibly relative URL string against a base URL.
///
/// Returns the value as-is if it already starts with `http://` or `https://`;
/// normalizes `//...` to `https:...`; otherwise joins with `base_url`.
#[must_use]
pub fn absolutize_url(value: &str, base_url: &Url) -> Option<String> {
    if value.starts_with("http://") || value.starts_with("https://") {
        return Some(value.to_string());
    }
    if value.starts_with("//") {
        return Some(format!("https:{value}"));
    }
    base_url.join(value).ok().map(|url| url.to_string())
}

/// Returns the first capture of `regex` in `html`, trimmed.
#[must_use]
pub fn extract_meta_value(html: &str, regex: &Regex) -> Option<String> {
    regex
        .captures(html)
        .and_then(|caps| caps.get(1).map(|m| m.as_str().trim().to_string()))
}

/// Regex for extracting a 4-digit year (19xx or 20xx) from a string.
pub static YEAR_VALUE_RE: LazyLock<Regex> =
    LazyLock::new(|| compile_static_regex(r"\b(19|20)\d{2}\b"));

/// Returns the first year-like match (19xx or 20xx) in `value`.
#[must_use]
pub fn extract_year_from_str(value: &str) -> Option<String> {
    YEAR_VALUE_RE
        .find(value)
        .map(|capture| capture.as_str().to_string())
}

/// Builds an auth requirement using `default_domain` when `domain` is empty.
#[must_use]
pub fn auth_requirement(
    domain: &str,
    default_domain: &str,
    message: impl Into<String>,
) -> AuthRequirement {
    AuthRequirement::new(
        if domain.is_empty() {
            default_domain
        } else {
            domain
        },
        message,
    )
}

/// Returns true if the HTTP status code indicates authentication is required.
#[must_use]
pub fn is_auth_required_status(status: u16) -> bool {
    matches!(status, 401 | 403 | 407)
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn test_canonical_host_trim_www_and_trailing_dot_lowercase() {
        assert_eq!(canonical_host("  www.Example.COM.  "), "example.com");
        assert_eq!(canonical_host("doi.org"), "doi.org");
        // Only literal "www." (lowercase) is stripped; "WWW." remains and is lowercased
        assert_eq!(canonical_host("WWW.DOI.ORG."), "www.doi.org");
    }

    #[test]
    fn test_parse_host_or_fallback_valid_url_returns_host() {
        assert_eq!(
            parse_host_or_fallback("https://ieeexplore.ieee.org/document/123"),
            "ieeexplore.ieee.org"
        );
    }

    #[test]
    fn test_parse_host_or_fallback_bare_host_uses_canonical_host() {
        assert_eq!(
            parse_host_or_fallback("  www.Example.COM.  "),
            "example.com"
        );
    }

    #[test]
    fn test_parse_host_or_fallback_invalid_url_fallbacks_to_canonical() {
        assert_eq!(parse_host_or_fallback("not-a-url"), "not-a-url");
    }

    #[test]
    fn test_hosts_match_same_host_after_normalization() {
        assert!(hosts_match("www.IEEE.org", "ieee.org"));
        assert!(hosts_match("ieeexplore.ieee.org", "ieeexplore.ieee.org"));
    }

    #[test]
    fn test_hosts_match_different_hosts_false() {
        assert!(!hosts_match("ieeexplore.ieee.org", "link.springer.com"));
    }

    #[test]
    fn test_looks_like_doi_prefix_match() {
        assert!(looks_like_doi("10.1109/foo", "10.1109/"));
        assert!(looks_like_doi("  10.1109/bar  ", "10.1109/"));
    }

    #[test]
    fn test_looks_like_doi_wrong_prefix_false() {
        assert!(!looks_like_doi("10.1007/bar", "10.1109/"));
    }

    #[test]
    fn test_looks_like_doi_prefix_normalized() {
        assert!(looks_like_doi("10.1109/x", "10.1109/"));
        assert!(looks_like_doi("10.1109/X", "10.1109/"));
    }

    #[test]
    fn test_looks_like_doi_prefix_trimmed_and_lowercased() {
        // Caller may pass messy prefix; it is normalized internally
        assert!(looks_like_doi("10.1109/foo", "  10.1109/  "));
        assert!(looks_like_doi("10.1109/foo", "10.1109/"));
    }

    #[test]
    fn test_looks_like_doi_empty_prefix_matches_any_value() {
        // Documented: empty prefix causes starts_with("") which is true for any string
        assert!(looks_like_doi("10.1109/foo", ""));
        assert!(looks_like_doi("anything", ""));
    }

    #[test]
    fn test_canonical_host_empty_and_whitespace() {
        assert_eq!(canonical_host(""), "");
        assert_eq!(canonical_host("  \t  "), "");
    }

    #[test]
    fn test_absolutize_url_absolute_unchanged() {
        let base = Url::parse("https://example.com/foo/").unwrap();
        assert_eq!(
            absolutize_url("https://other.com/path", &base),
            Some("https://other.com/path".to_string())
        );
        assert_eq!(
            absolutize_url("http://other.com/path", &base),
            Some("http://other.com/path".to_string())
        );
    }

    #[test]
    fn test_absolutize_url_protocol_relative() {
        let base = Url::parse("https://example.com/foo/").unwrap();
        assert_eq!(
            absolutize_url("//example.com/bar", &base),
            Some("https://example.com/bar".to_string())
        );
    }

    #[test]
    fn test_absolutize_url_relative() {
        let base = Url::parse("https://example.com/foo/").unwrap();
        assert_eq!(
            absolutize_url("bar", &base),
            Some("https://example.com/foo/bar".to_string())
        );
    }

    #[test]
    fn test_extract_year_from_str() {
        assert_eq!(
            extract_year_from_str("Published in 2023."),
            Some("2023".to_string())
        );
        assert_eq!(
            extract_year_from_str("1999-01-15"),
            Some("1999".to_string())
        );
        assert_eq!(extract_year_from_str("no year"), None);
    }

    #[test]
    fn test_auth_requirement_uses_domain_when_non_empty() {
        let req = auth_requirement("custom.example.com", "default.com", "Login required");
        assert_eq!(req.domain, "custom.example.com");
        assert_eq!(req.message, "Login required");
    }

    #[test]
    fn test_auth_requirement_uses_default_when_domain_empty() {
        let req = auth_requirement("", "default.com", "Login required");
        assert_eq!(req.domain, "default.com");
    }

    #[test]
    fn test_is_auth_required_status() {
        assert!(is_auth_required_status(401));
        assert!(is_auth_required_status(403));
        assert!(is_auth_required_status(407));
        assert!(!is_auth_required_status(200));
        assert!(!is_auth_required_status(404));
    }
}
