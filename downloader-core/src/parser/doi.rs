//! DOI detection, validation, and normalization from text input.

use std::sync::LazyLock;

use regex::Regex;
use tracing::{debug, trace};

use super::error::ParseError;
use super::input::ParsedItem;
use super::url::clean_url_trailing;

/// Regex pattern for bare DOIs: `10.XXXX/suffix`
/// Handles nested registrants like `10.1000.10/example`.
/// Note: Preceding character check (to reject IP-like patterns) is done in code
/// since the `regex` crate doesn't support lookbehind.
#[allow(clippy::expect_used)]
static DOI_PATTERN: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"10\.\d{4,9}(?:\.\d+)*/[^\s<>"'\]]+"#).expect("DOI regex is valid") // Static pattern, safe to panic
});

/// Regex pattern for DOI URLs: `https://doi.org/10.XXXX/suffix` or `https://dx.doi.org/...`
#[allow(clippy::expect_used)]
static DOI_URL_PATTERN: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"https?://(?:dx\.)?doi\.org/(10\.\d{4,9}(?:\.\d+)*/[^\s<>"'\]]+)"#)
        .expect("DOI URL regex is valid") // Static pattern, safe to panic
});

/// Regex pattern for `DOI:` prefixed DOIs: `DOI: 10.XXXX/suffix`
#[allow(clippy::expect_used)]
static DOI_PREFIX_PATTERN: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"(?i)doi:\s*(10\.\d{4,9}(?:\.\d+)*/[^\s<>"'\]]+)"#)
        .expect("DOI prefix regex is valid") // Static pattern, safe to panic
});

/// Result type for DOI extraction operations.
pub type DoiExtractionResult = Result<ParsedItem, ParseError>;

/// Extracts and validates DOIs from text input.
///
/// This function finds all DOIs in the input text in various formats
/// (bare, URL, prefixed), validates them, and returns a list of results.
///
/// # Arguments
///
/// * `input` - Text input that may contain DOIs mixed with other content
///
/// # Returns
///
/// A vector of results, one per DOI candidate found. Each result is either:
/// - `Ok(ParsedItem)` - A valid DOI was extracted and normalized
/// - `Err(ParseError)` - The DOI was invalid (malformed, missing suffix, etc.)
///
/// # Examples
///
/// ```
/// use downloader_core::parser::extract_dois;
///
/// let results = extract_dois("See DOI: 10.1234/example for details");
/// assert_eq!(results.len(), 1);
/// assert!(results[0].is_ok());
/// ```
#[tracing::instrument(skip(input), fields(input_len = input.len()))]
#[must_use]
pub fn extract_dois(input: &str) -> Vec<DoiExtractionResult> {
    let mut results = Vec::new();
    let mut seen_ranges: Vec<(usize, usize)> = Vec::new();

    // Extract DOI URLs first (most specific pattern)
    for cap in DOI_URL_PATTERN.captures_iter(input) {
        if let Some(full_match) = cap.get(0) {
            let raw = full_match.as_str();
            let doi_part = &cap[1];
            seen_ranges.push((full_match.start(), full_match.end()));
            trace!(raw = %raw, "found DOI URL candidate");
            process_doi(raw, doi_part, &mut results);
        }
    }

    // Extract DOI: prefixed DOIs
    for cap in DOI_PREFIX_PATTERN.captures_iter(input) {
        if let Some(full_match) = cap.get(0) {
            if overlaps(&seen_ranges, full_match.start(), full_match.end()) {
                continue;
            }
            let raw = full_match.as_str();
            let doi_part = &cap[1];
            seen_ranges.push((full_match.start(), full_match.end()));
            trace!(raw = %raw, "found DOI prefix candidate");
            process_doi(raw, doi_part, &mut results);
        }
    }

    // Extract bare DOIs
    for m in DOI_PATTERN.find_iter(input) {
        if overlaps(&seen_ranges, m.start(), m.end()) {
            continue;
        }
        // Check preceding character to reject false positives:
        // - IP-like patterns (e.g., 192.10.1234/24) - preceded by digit or dot
        // - Version numbers (e.g., v10.1234/rc1) - preceded by letter
        if m.start() > 0 {
            let prev_byte = input.as_bytes()[m.start() - 1];
            if prev_byte.is_ascii_alphanumeric() || prev_byte == b'.' {
                continue;
            }
        }
        let raw = m.as_str();
        seen_ranges.push((m.start(), m.end()));
        trace!(raw = %raw, "found bare DOI candidate");
        process_doi(raw, raw, &mut results);
    }

    results
}

/// Check if a range overlaps with any already-seen range.
fn overlaps(seen: &[(usize, usize)], start: usize, end: usize) -> bool {
    seen.iter().any(|&(s, e)| start < e && end > s)
}

/// Process a DOI candidate through normalize → clean → validate pipeline.
fn process_doi(raw: &str, doi_part: &str, results: &mut Vec<DoiExtractionResult>) {
    let normalized = normalize_doi(doi_part);
    let cleaned = clean_url_trailing(&normalized);
    let cleaned = clean_doi_parens(cleaned);
    let cleaned = clean_doi_braces(&cleaned);

    match validate_doi(&cleaned) {
        Ok(validated) => {
            debug!(doi = %validated, "DOI validated");
            results.push(Ok(ParsedItem::doi(raw, validated)));
        }
        Err(e) => {
            debug!(doi = %cleaned, error = %e, "DOI validation failed");
            results.push(Err(e));
        }
    }
}

/// Normalizes a DOI by stripping prefixes and decoding.
///
/// Strips URL prefixes (`https://doi.org/`, `https://dx.doi.org/`),
/// text prefixes (`doi:`, `DOI:`), URL-decodes, and trims whitespace.
#[must_use]
fn normalize_doi(input: &str) -> String {
    let mut doi = input.trim();

    // Strip URL prefixes
    for prefix in &[
        "https://doi.org/",
        "http://doi.org/",
        "https://dx.doi.org/",
        "http://dx.doi.org/",
    ] {
        if let Some(stripped) = doi.strip_prefix(prefix) {
            doi = stripped;
            break;
        }
    }

    // Strip doi: prefix (case-insensitive)
    if doi.len() >= 4 && doi[..4].eq_ignore_ascii_case("doi:") {
        doi = doi[4..].trim_start();
    }

    // URL-decode
    match urlencoding::decode(doi) {
        Ok(decoded) => decoded.trim().to_string(),
        Err(_) => doi.trim().to_string(),
    }
}

/// Validates a DOI string and returns the validated DOI.
///
/// # Validation rules:
/// - Must start with `10.`
/// - Registrant code must be 4+ digits (including nested like `10.1000.10`)
/// - Must have a non-empty suffix after `/`
fn validate_doi(doi: &str) -> Result<String, ParseError> {
    // Must start with 10.
    if !doi.starts_with("10.") {
        return Err(ParseError::invalid_doi(doi, "DOI must start with '10.'"));
    }

    // Find the first /
    let Some(slash_pos) = doi.find('/') else {
        return Err(ParseError::doi_no_suffix(doi));
    };

    // Validate registrant code (between "10." and "/")
    let registrant = &doi[3..slash_pos];
    if registrant.is_empty() {
        return Err(ParseError::invalid_doi(
            doi,
            "missing registrant code after '10.'",
        ));
    }

    // First segment of registrant must be 4+ digits
    let first_segment = registrant.split('.').next().unwrap_or("");
    if first_segment.len() < 4 || !first_segment.chars().all(|c| c.is_ascii_digit()) {
        return Err(ParseError::invalid_doi(
            doi,
            "registrant code must have at least 4 digits",
        ));
    }

    // Suffix must be non-empty
    let suffix = &doi[slash_pos + 1..];
    if suffix.is_empty() {
        return Err(ParseError::doi_no_suffix(doi));
    }

    Ok(doi.to_string())
}

/// Cleans unmatched trailing parentheses from DOI suffix.
///
/// DOIs can contain parentheses in their suffix (e.g., `10.1002/(SICI)1097-4636`),
/// but are often wrapped in parentheses in text (e.g., `(10.1234/example)`).
/// This strips trailing `)` only if unmatched.
fn clean_doi_parens(doi: &str) -> String {
    let mut result = doi.to_string();

    // Find suffix (after first /)
    if let Some(slash_pos) = result.find('/') {
        // Strip trailing ) while unbalanced
        while result.ends_with(')') && {
            let s = &result[slash_pos + 1..];
            s.chars().filter(|&c| c == ')').count() > s.chars().filter(|&c| c == '(').count()
        } {
            result.pop();
        }
    }

    result
}

/// Cleans unmatched trailing braces from DOI suffix.
fn clean_doi_braces(doi: &str) -> String {
    let mut result = doi.to_string();

    if let Some(slash_pos) = result.find('/') {
        while result.ends_with('}') && {
            let s = &result[slash_pos + 1..];
            s.chars().filter(|&c| c == '}').count() > s.chars().filter(|&c| c == '{').count()
        } {
            result.pop();
        }
    }

    result
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::super::input::InputType;
    use super::*;

    // ==================== Happy Path Tests ====================

    #[test]
    fn test_extract_dois_bare_doi_detected() {
        let results = extract_dois("10.1234/example");
        assert_eq!(results.len(), 1);
        let item = results[0].as_ref().unwrap();
        assert_eq!(item.input_type, InputType::Doi);
        assert_eq!(item.value, "10.1234/example");
    }

    #[test]
    fn test_extract_dois_long_registrant_detected() {
        let results = extract_dois("10.12345678/example");
        assert_eq!(results.len(), 1);
        assert!(results[0].is_ok());
        assert_eq!(results[0].as_ref().unwrap().value, "10.12345678/example");
    }

    #[test]
    fn test_extract_dois_nested_registrant_detected() {
        let results = extract_dois("10.1000.10/example");
        assert_eq!(results.len(), 1);
        assert!(results[0].is_ok());
        assert_eq!(results[0].as_ref().unwrap().value, "10.1000.10/example");
    }

    #[test]
    fn test_extract_dois_complex_suffix_detected() {
        let results = extract_dois("10.1038/s41586-024-07386-0");
        assert_eq!(results.len(), 1);
        assert!(results[0].is_ok());
        assert_eq!(
            results[0].as_ref().unwrap().value,
            "10.1038/s41586-024-07386-0"
        );
    }

    #[test]
    fn test_extract_dois_elsevier_suffix_detected() {
        let results = extract_dois("10.1016/j.cell.2024.01.001");
        assert_eq!(results.len(), 1);
        assert!(results[0].is_ok());
        assert_eq!(
            results[0].as_ref().unwrap().value,
            "10.1016/j.cell.2024.01.001"
        );
    }

    #[test]
    fn test_extract_dois_doi_url_detected() {
        let results = extract_dois("https://doi.org/10.1234/example");
        assert_eq!(results.len(), 1);
        let item = results[0].as_ref().unwrap();
        assert_eq!(item.input_type, InputType::Doi);
        assert_eq!(item.value, "10.1234/example");
        assert_eq!(item.raw, "https://doi.org/10.1234/example");
    }

    #[test]
    fn test_extract_dois_dx_doi_url_detected() {
        let results = extract_dois("https://dx.doi.org/10.1234/example");
        assert_eq!(results.len(), 1);
        assert!(results[0].is_ok());
        assert_eq!(results[0].as_ref().unwrap().value, "10.1234/example");
    }

    #[test]
    fn test_extract_dois_http_doi_url_detected() {
        let results = extract_dois("http://doi.org/10.1234/example");
        assert_eq!(results.len(), 1);
        assert!(results[0].is_ok());
        assert_eq!(results[0].as_ref().unwrap().value, "10.1234/example");
    }

    #[test]
    fn test_extract_dois_doi_prefix_detected() {
        let results = extract_dois("DOI: 10.1234/example");
        assert_eq!(results.len(), 1);
        let item = results[0].as_ref().unwrap();
        assert_eq!(item.value, "10.1234/example");
    }

    #[test]
    fn test_extract_dois_doi_prefix_lowercase_detected() {
        let results = extract_dois("doi:10.1234/example");
        assert_eq!(results.len(), 1);
        assert!(results[0].is_ok());
        assert_eq!(results[0].as_ref().unwrap().value, "10.1234/example");
    }

    #[test]
    fn test_extract_dois_multiple_in_text() {
        let input = "10.1234/first 10.5678/second 10.9012/third";
        let results = extract_dois(input);
        assert_eq!(results.len(), 3);
        assert!(results.iter().all(|r| r.is_ok()));
    }

    #[test]
    fn test_extract_dois_from_mixed_text() {
        let input = "See paper at 10.1038/nature12373 for details. Also check DOI: 10.1016/j.cell.2024.01.001 later.";
        let results = extract_dois(input);
        assert_eq!(results.len(), 2);
        assert!(results.iter().all(|r| r.is_ok()));
    }

    // ==================== Normalization Tests ====================

    #[test]
    fn test_normalize_doi_strips_url_prefix() {
        assert_eq!(normalize_doi("https://doi.org/10.1234/x"), "10.1234/x");
    }

    #[test]
    fn test_normalize_doi_strips_doi_prefix() {
        assert_eq!(normalize_doi("DOI: 10.1234/x"), "10.1234/x");
    }

    #[test]
    fn test_normalize_doi_trims_whitespace() {
        assert_eq!(normalize_doi("  10.1234/x  "), "10.1234/x");
    }

    #[test]
    fn test_normalize_doi_url_decodes() {
        let result = normalize_doi("https://doi.org/10.1002%2F(SICI)1097-4636");
        assert!(result.contains("10.1002/(SICI)1097-4636"));
    }

    // ==================== Validation Error Tests ====================

    #[test]
    fn test_validate_doi_rejects_no_suffix() {
        let result = validate_doi("10.1234/");
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_doi_rejects_short_registrant() {
        let result = validate_doi("10.12/example");
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_doi_rejects_no_registrant() {
        let result = validate_doi("10./example");
        assert!(result.is_err());
    }

    // ==================== Edge Case & Trailing Punctuation Tests ====================

    #[test]
    fn test_extract_dois_trailing_period_cleaned() {
        let results = extract_dois("10.1234/example.");
        assert_eq!(results.len(), 1);
        let item = results[0].as_ref().unwrap();
        assert_eq!(item.value, "10.1234/example");
    }

    #[test]
    fn test_extract_dois_trailing_comma_cleaned() {
        let results = extract_dois("10.1234/example,");
        assert_eq!(results.len(), 1);
        let item = results[0].as_ref().unwrap();
        assert_eq!(item.value, "10.1234/example");
    }

    #[test]
    fn test_extract_dois_in_parentheses() {
        let results = extract_dois("(10.1234/example)");
        assert_eq!(results.len(), 1);
        let item = results[0].as_ref().unwrap();
        assert_eq!(item.value, "10.1234/example");
    }

    #[test]
    fn test_extract_dois_parens_in_suffix_preserved() {
        let results = extract_dois("10.1002/(SICI)1097-4636");
        assert_eq!(results.len(), 1);
        let item = results[0].as_ref().unwrap();
        assert_eq!(item.value, "10.1002/(SICI)1097-4636");
    }

    #[test]
    fn test_extract_dois_trailing_braces_cleaned() {
        let results = extract_dois("doi={10.1234/example}}");
        assert_eq!(results.len(), 1);
        let item = results[0].as_ref().unwrap();
        assert_eq!(item.value, "10.1234/example");
    }

    #[test]
    fn test_extract_dois_empty_input_returns_empty() {
        let results = extract_dois("");
        assert!(results.is_empty());
    }

    // ==================== False-Positive Prevention Tests ====================

    #[test]
    fn test_extract_dois_ignores_version_number() {
        // "v10.1234/rc1" - preceded by letter 'v', should NOT match
        let results = extract_dois("v10.1234/rc1");
        assert!(
            results.is_empty(),
            "v10.1234/rc1 should not match (preceded by alpha char)"
        );
    }

    #[test]
    fn test_extract_dois_ignores_score_fraction() {
        // "rated 10.5/10" - registrant "5" is too short (< 4 digits)
        let results = extract_dois("rated 10.5/10");
        assert!(
            results.is_empty(),
            "10.5/10 should not match (registrant too short)"
        );
    }

    #[test]
    fn test_extract_dois_ignores_ip_like_pattern() {
        // "192.10.1234/24" - preceded by digits and dot, should not match
        let results = extract_dois("192.10.1234/24");
        assert!(
            results.is_empty(),
            "IP-like pattern should not match due to negative lookbehind"
        );
    }

    #[test]
    fn test_extract_dois_section_reference_matches() {
        // "Section 10.1234/A describes..." SHOULD match - this IS a valid DOI pattern
        let results = extract_dois("Section 10.1234/A describes...");
        assert_eq!(results.len(), 1);
        assert!(results[0].is_ok());
    }

    #[test]
    fn test_extract_dois_ignores_short_registrant_fraction() {
        // "10.12/something" - registrant < 4 digits
        let results = extract_dois("10.12/something");
        assert!(
            results.is_empty(),
            "10.12/something should not match (registrant < 4 digits)"
        );
    }
}
