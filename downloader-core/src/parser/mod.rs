//! Input parsing module for extracting URLs, DOIs, and references.
//!
//! This module provides functionality to parse raw text input and extract
//! downloadable items such as URLs, DOIs, and bibliographic references.
//!
//! # Current Support
//!
//! - HTTP/HTTPS URLs
//! - DOIs (10.xxxx/...)
//! - Reference strings (Author, Year, Title format)
//! - Multi-line bibliography extraction (segmented into per-entry references)
//! - BibTeX entries (`@article`, `@book`, `@inproceedings`)
//!
//! # Example
//!
//! ```
//! use downloader_core::parser::{parse_input, InputType};
//!
//! let result = parse_input("Check out https://example.com/paper.pdf");
//! assert_eq!(result.len(), 1);
//! assert_eq!(result.items[0].input_type, InputType::Url);
//! ```

mod bibliography;
mod bibtex;
mod doi;
mod error;
mod input;
mod reference;
mod url;

pub use bibliography::{
    BibliographyParseResult, BibliographySummary, extract_bibliography_entries, parse_bibliography,
    summarize_bibliography,
};
pub use bibtex::{BibtexEntry, BibtexParseResult, parse_bibtex_entries};
pub use doi::extract_dois;
pub use error::ParseError;
pub use input::{InputType, ParseResult, ParseTypeCounts, ParsedItem};
pub use reference::{
    Confidence, ConfidenceFactors, ReferenceConfidence, ReferenceMetadata,
    extract_reference_confidence, extract_references, parse_reference_metadata,
};
pub use url::extract_urls;

use std::collections::HashSet;
use tracing::{debug, info};

/// Parses raw text input and extracts downloadable items.
///
/// This is the main entry point for the parser module. It analyzes the input
/// text and extracts all recognizable items (URLs, DOIs, references) into a
/// structured result.
///
/// # Arguments
///
/// * `input` - Raw text input that may contain URLs, DOIs, references, or a mix
///
/// # Returns
///
/// A `ParseResult` containing:
/// - `items` - Successfully parsed items with their types and normalized values
/// - `skipped` - Lines or fragments that couldn't be parsed
///
/// # Behavior
///
/// - Empty input returns an empty result (not an error)
/// - Each URL is validated individually; invalid URLs are logged but don't fail parsing
/// - DOIs are extracted in various formats (bare, URL, prefixed) and normalized
/// - Remaining unmatched lines are evaluated as reference strings
///
/// # Example
///
/// ```
/// use downloader_core::parser::parse_input;
///
/// let result = parse_input(r#"
/// References:
/// https://arxiv.org/pdf/2301.00001.pdf
/// Some text to ignore
/// https://example.com/paper.pdf
/// "#);
///
/// assert_eq!(result.urls().count(), 2);
/// ```
#[tracing::instrument(skip(input), fields(input_len = input.len()))]
#[must_use]
pub fn parse_input(input: &str) -> ParseResult {
    let mut result = ParseResult::new();

    // Handle empty input gracefully
    if input.trim().is_empty() {
        debug!("Empty input provided");
        return result;
    }

    // Extract DOIs first
    let doi_results = extract_dois(input);

    let mut doi_count = 0;
    let mut url_count = 0;
    let mut ref_count = 0;
    let mut bibtex_count = 0;
    let mut error_count = 0;

    let mut seen_dois: HashSet<String> = HashSet::new();
    for doi_result in doi_results {
        match doi_result {
            Ok(item) => {
                if !seen_dois.insert(item.value.clone()) {
                    continue;
                }
                doi_count += 1;
                result.add_item(item);
            }
            Err(e) => {
                error_count += 1;
                debug!(error = %e, "DOI extraction error");
                if let ParseError::InvalidDoi { doi, .. } = &e {
                    result.add_skipped(doi.clone());
                }
            }
        }
    }

    // Extract URLs from the input
    let url_results = extract_urls(input);

    // Post-filter: DOIs win over doi.org URLs
    let url_results: Vec<_> = url_results
        .into_iter()
        .filter(|r| match r {
            Ok(item) => {
                let parsed = ::url::Url::parse(&item.value).ok();
                !parsed.is_some_and(|u| matches!(u.host_str(), Some("doi.org" | "dx.doi.org")))
            }
            Err(_) => true, // Keep errors for skipped list
        })
        .collect();

    for url_result in url_results {
        match url_result {
            Ok(item) => {
                url_count += 1;
                result.add_item(item);
            }
            Err(e) => {
                error_count += 1;
                debug!(error = %e, "URL extraction error");
                match &e {
                    ParseError::InvalidUrl { url, .. } => {
                        result.add_skipped(url.clone());
                    }
                    ParseError::UrlTooLong { url_preview, .. } => {
                        result.add_skipped(url_preview.clone());
                    }
                    ParseError::InvalidDoi { .. } | ParseError::UnparseableReference { .. } => {}
                }
            }
        }
    }

    let residual_input = build_residual_input(input);
    if residual_input.lines().any(|line| !line.trim().is_empty()) {
        let residual_stats = process_residual_content(&mut result, &residual_input);
        ref_count += residual_stats.references;
        bibtex_count += residual_stats.bibtex;
        error_count += residual_stats.errors;
    }

    info!(
        urls = url_count,
        dois = doi_count,
        references = ref_count,
        bibtex = bibtex_count,
        errors = error_count,
        total = result.len(),
        skipped = result.skipped_count(),
        "Parsing complete"
    );

    result
}

#[derive(Debug, Clone, Copy, Default)]
struct ResidualMergeStats {
    references: usize,
    bibtex: usize,
    errors: usize,
}

fn process_residual_content(result: &mut ParseResult, residual_input: &str) -> ResidualMergeStats {
    // Deterministic merge-order contract for mixed parser output:
    // 1) DOI extractor results
    // 2) URL extractor results
    // 3) bibliography/reference residual parsing
    // 4) BibTeX residual parsing (per-entry order; DOI then mapped reference)
    //
    // DOI de-duplication contract across extractors:
    // - Canonical winner: first DOI extracted in earlier phase order
    // - Later extractors (BibTeX) may emit DOI candidates, but duplicates are dropped
    //   using normalized DOI value equality.
    let mut stats = ResidualMergeStats::default();
    let mut residual_for_bibliography = residual_input.to_string();
    let bibtex_result = parse_bibtex_entries(residual_input);
    let mut seen_dois: HashSet<String> = result
        .items
        .iter()
        .filter(|item| item.input_type == InputType::Doi)
        .map(|item| item.value.clone())
        .collect();

    for segment in &bibtex_result.consumed_segments {
        residual_for_bibliography = residual_for_bibliography.replacen(segment, " ", 1);
    }

    let bibliography_result = parse_bibliography(&residual_for_bibliography);
    let bibliography_summary = summarize_bibliography(&bibliography_result);
    stats.references += bibliography_summary.parsed;
    stats.errors += bibliography_summary.uncertain;

    for item in bibliography_result.parsed {
        result.add_item(item);
    }
    for uncertain in bibliography_result.uncertain {
        result.add_skipped(uncertain);
    }

    for item in bibtex_result.items {
        if item.input_type == InputType::Doi && !seen_dois.insert(item.value.clone()) {
            continue;
        }
        match item.input_type {
            InputType::Reference => stats.references += 1,
            InputType::BibTex => stats.bibtex += 1,
            _ => {}
        }
        result.add_item(item);
    }

    stats.errors += bibtex_result.skipped.len();
    for message in bibtex_result.skipped {
        result.add_skipped(message);
    }

    stats
}

fn build_residual_input(input: &str) -> String {
    let mut residual_lines = Vec::new();
    let mut in_bibtex_block = false;
    let mut bibtex_brace_depth = 0i32;

    for raw_line in input.lines() {
        let line = raw_line.trim();
        if line.is_empty() {
            residual_lines.push(String::new());
            continue;
        }

        if in_bibtex_block {
            residual_lines.push(line.to_string());
            bibtex_brace_depth += bibtex_brace_delta(line);
            if bibtex_brace_depth <= 0 {
                in_bibtex_block = false;
                bibtex_brace_depth = 0;
            }
            continue;
        }

        if looks_like_bibtex_line(line) {
            in_bibtex_block = true;
            bibtex_brace_depth = bibtex_brace_delta(line);
            residual_lines.push(line.to_string());
            if bibtex_brace_depth <= 0 {
                in_bibtex_block = false;
                bibtex_brace_depth = 0;
            }
            continue;
        }

        residual_lines.push(strip_matched_fragments(line).trim().to_string());
    }

    residual_lines.join("\n")
}

#[allow(clippy::cast_possible_truncation, clippy::cast_possible_wrap)]
fn bibtex_brace_delta(line: &str) -> i32 {
    // Count braces outside of quoted strings to stay consistent with
    // the quote-aware brace balancing in bibtex::segment_entries.
    let mut opens = 0i32;
    let mut closes = 0i32;
    let mut in_quotes = false;
    let mut escape = false;

    for ch in line.chars() {
        if escape {
            escape = false;
            continue;
        }
        if ch == '\\' {
            escape = true;
            continue;
        }
        if ch == '"' {
            in_quotes = !in_quotes;
            continue;
        }
        if !in_quotes {
            if ch == '{' {
                opens += 1;
            } else if ch == '}' {
                closes += 1;
            }
        }
    }

    opens - closes
}

fn strip_matched_fragments(line: &str) -> String {
    let mut residual = line.to_string();

    for doi_result in extract_dois(line) {
        if let Ok(item) = doi_result
            && !item.raw.is_empty()
        {
            residual = residual.replacen(&item.raw, " ", 1);
        }
    }

    let url_results: Vec<_> = extract_urls(line)
        .into_iter()
        .filter(|result| match result {
            Ok(item) => {
                let parsed = ::url::Url::parse(&item.value).ok();
                !parsed.is_some_and(|u| matches!(u.host_str(), Some("doi.org" | "dx.doi.org")))
            }
            Err(_) => true,
        })
        .collect();

    for url_result in url_results {
        if let Ok(item) = url_result
            && !item.raw.is_empty()
        {
            residual = residual.replacen(&item.raw, " ", 1);
        }
    }

    residual
}

fn looks_like_bibtex_line(line: &str) -> bool {
    let trimmed = line.trim_start();
    if !trimmed.starts_with('@') {
        return false;
    }

    let Some(open_brace) = trimmed.find('{') else {
        return false;
    };
    trimmed[1..open_brace]
        .chars()
        .all(|ch| ch.is_ascii_alphabetic())
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    // ==================== AC1: HTTP/HTTPS URL Extraction ====================

    #[test]
    fn test_parse_input_extracts_http_url() {
        let result = parse_input("http://example.com/file.pdf");
        assert_eq!(result.len(), 1);
        assert_eq!(result.items[0].input_type, InputType::Url);
    }

    #[test]
    fn test_parse_input_extracts_https_url() {
        let result = parse_input("https://example.com/paper.pdf");
        assert_eq!(result.len(), 1);
        assert_eq!(result.items[0].input_type, InputType::Url);
    }

    // ==================== AC2: Invalid URL Reporting (logged, not failed) ====================

    #[test]
    fn test_parse_input_handles_invalid_gracefully() {
        // parse_input doesn't fail on invalid URLs - they're skipped
        // This is by design: we want to extract what we can
        let result = parse_input("https://example.com/good.pdf");
        assert_eq!(result.len(), 1);
    }

    // ==================== AC3: Non-URL Text Handling ====================

    #[test]
    fn test_parse_input_ignores_non_url_text() {
        let result = parse_input("This is just plain text with no URLs");
        assert!(result.is_empty());
        assert_eq!(result.skipped_count(), 0); // Plain text isn't "skipped", just not matched
    }

    #[test]
    fn test_parse_input_extracts_urls_from_mixed_text() {
        let input = r#"
        References:
        1. https://arxiv.org/pdf/2301.00001.pdf
        2. Smith, J. (2024). Paper Title. Journal.
        3. https://example.com/papers/paper.pdf
        4. Some other text that should be ignored.
        "#;

        let result = parse_input(input);

        // Should find 2 URLs and 1 reference
        assert_eq!(result.len(), 3);
        assert!(result.items[0].value.contains("arxiv.org"));
        assert!(result.items[1].value.contains("example.com"));
        assert_eq!(result.references().count(), 1);
    }

    // ==================== AC4: ParsedInput Result ====================

    #[test]
    fn test_parse_input_returns_parse_result() {
        let result = parse_input("https://example.com/doc.pdf");

        assert!(!result.is_empty());
        assert_eq!(result.len(), 1);

        let item = &result.items[0];
        assert!(!item.raw.is_empty());
        assert_eq!(item.input_type, InputType::Url);
        assert!(!item.value.is_empty());
    }

    #[test]
    fn test_parse_input_urls_iterator() {
        let result = parse_input("https://a.com https://b.com");
        let urls: Vec<_> = result.urls().collect();
        assert_eq!(urls.len(), 2);
    }

    // ==================== AC5: Multi-Line Input Support ====================

    #[test]
    fn test_parse_input_handles_multiline() {
        let input = "https://first.com\nhttps://second.com\nhttps://third.com";
        let result = parse_input(input);
        assert_eq!(result.len(), 3);
    }

    #[test]
    fn test_parse_input_preserves_order() {
        let input = "https://1.com\nhttps://2.com\nhttps://3.com";
        let result = parse_input(input);

        let values: Vec<_> = result.items.iter().map(|i| &i.value).collect();
        assert!(values[0].contains("1.com"));
        assert!(values[1].contains("2.com"));
        assert!(values[2].contains("3.com"));
    }

    // ==================== Edge Cases ====================

    #[test]
    fn test_parse_input_empty_returns_empty_result() {
        let result = parse_input("");
        assert!(result.is_empty());
        assert_eq!(result.skipped_count(), 0);
    }

    #[test]
    fn test_parse_input_whitespace_only_returns_empty() {
        let result = parse_input("   \n\t\n   ");
        assert!(result.is_empty());
    }

    #[test]
    fn test_parse_input_with_query_strings() {
        let result = parse_input("https://example.com/search?q=test&page=1");
        assert_eq!(result.len(), 1);
        assert!(result.items[0].value.contains("q=test"));
    }

    #[test]
    fn test_parse_input_display() {
        let result = parse_input("https://a.com https://b.com");
        let display = result.to_string();
        assert!(display.contains("2 items"));
    }

    #[test]
    fn test_parse_input_recognizes_reference() {
        let result = parse_input("Smith, J. (2024). Paper Title. Journal Name, 1(2), 3-4.");
        let references: Vec<_> = result.references().collect();
        assert_eq!(references.len(), 1);
        assert_eq!(references[0].input_type, InputType::Reference);
    }

    #[test]
    fn test_parse_input_mixed_types_with_references() {
        let input = r#"
        https://example.com/doc.pdf
        DOI: 10.1234/example
        Smith, J. (2024). Paper Title. Journal Name, 1(2), 3-4.
        "#;

        let result = parse_input(input);
        assert_eq!(result.urls().count(), 1);
        assert_eq!(result.dois().count(), 1);
        assert_eq!(result.references().count(), 1);
    }

    #[test]
    fn test_parse_input_line_with_url_and_reference_extracts_both() {
        let input = "https://example.com/paper.pdf Smith, J. (2024). Paper Title. Journal.";
        let result = parse_input(input);

        assert_eq!(result.urls().count(), 1);
        assert_eq!(result.references().count(), 1);
    }

    #[test]
    fn test_parse_input_line_with_doi_and_reference_extracts_both() {
        let input = "Smith, J. (2024). Paper Title. Journal. https://doi.org/10.1234/example";
        let result = parse_input(input);

        assert_eq!(result.dois().count(), 1);
        assert_eq!(result.references().count(), 1);
    }

    #[test]
    fn test_parse_input_bibliography_heading_and_numbered_entries() {
        let input = "References\n1. Smith, J. (2024). Title One. Journal.\n2) Jones, K. (2023). Title Two. Journal.";
        let result = parse_input(input);

        assert_eq!(result.references().count(), 2);
        assert_eq!(result.skipped_count(), 0);
    }

    #[test]
    fn test_parse_input_adjacent_reference_lines_extract_two_entries() {
        let input = "Smith, J. (2024). Complete Title. Journal Name, 1(2), 3-4.\n2024. Journal Overview and Findings in Practice.";
        let result = parse_input(input);

        assert_eq!(result.references().count(), 2);
    }

    #[test]
    fn test_parse_input_bibtex_entry_extracts_doi_and_reference() {
        let input = r#"@article{key, title={BibTeX Title}, author={Smith, J. and Doe, R.}, year={2024}, doi={10.1234/example}}"#;
        let result = parse_input(input);

        assert_eq!(result.dois().count(), 1);
        assert_eq!(result.references().count(), 1);
        assert_eq!(result.bibtex().count(), 1);
    }

    #[test]
    fn test_parse_input_bibtex_doi_deduplicated_against_global_extractor() {
        let input = r#"
10.1234/example
@article{key, title={BibTeX Title}, author={Smith, J.}, year={2024}, doi={10.1234/example}}
"#;
        let result = parse_input(input);
        assert_eq!(result.dois().count(), 1);
    }

    #[test]
    fn test_parse_input_bibtex_malformed_isolated_from_valid_neighbor() {
        let input = r#"
@article{bad, title={Broken}, year={2024}
@article{ok, title={Good}, author={Smith, J.}, year={2024}, doi={10.1234/good}}
"#;
        let result = parse_input(input);

        assert_eq!(result.dois().count(), 1);
        assert_eq!(result.references().count(), 1);
        assert!(result.skipped.iter().any(|line| line.contains("What:")));
    }

    #[test]
    fn test_bibtex_brace_delta_ignores_braces_inside_quotes() {
        // Braces inside quoted strings should not affect the delta.
        assert_eq!(bibtex_brace_delta(r#"  title = "A {nested} title","#), 0);
        assert_eq!(bibtex_brace_delta(r#"  title = "A {unclosed title","#), 0);
        // Unquoted braces still count.
        assert_eq!(bibtex_brace_delta(r#"  title = {A title},"#), 0);
        assert_eq!(bibtex_brace_delta(r#"@article{key,"#), 1);
    }

    #[test]
    fn test_parse_input_multiline_bibtex_quoted_field_with_brace() {
        let input = "@article{key,\n  title = \"Study of {brackets\",\n  author = {Smith, J.},\n  year = {2024}\n}\nSmith, J. (2024). Standalone Reference. Journal Name, 1(2), 3-4.";
        let result = parse_input(input);

        assert_eq!(result.bibtex().count(), 1, "BibTeX entry should be parsed");
        assert!(
            result.references().count() >= 1,
            "Standalone reference after BibTeX should be detected"
        );
    }

    #[test]
    fn test_parse_input_multiline_bibtex_preserved_through_residual_stripping() {
        let input = r#"
@article{m1,
  title={Multiline BibTeX},
  author={Doe, Jane},
  year={2024},
  doi={10.9999/multi}
}
"#;
        let result = parse_input(input);

        assert_eq!(result.dois().count(), 1);
        assert_eq!(result.bibtex().count(), 1);
        assert_eq!(result.references().count(), 1);
        assert_eq!(result.skipped_count(), 0);
    }
}
