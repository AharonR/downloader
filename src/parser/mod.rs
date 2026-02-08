//! Input parsing module for extracting URLs, DOIs, and references.
//!
//! This module provides functionality to parse raw text input and extract
//! downloadable items such as URLs, DOIs, and bibliographic references.
//!
//! # Current Support
//!
//! - HTTP/HTTPS URLs
//!
//! # Future Support (Epic 2)
//!
//! - DOIs (10.xxxx/...)
//! - Reference strings (Author, Year, Title format)
//! - BibTeX entries
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

mod error;
mod input;
mod url;

pub use error::ParseError;
pub use input::{InputType, ParseResult, ParsedItem};
pub use url::extract_urls;

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
/// - Non-URL text is currently ignored (will be expanded for DOIs/references in Epic 2)
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

    // Extract URLs from the input
    let url_results = extract_urls(input);

    let mut url_count = 0;
    let mut error_count = 0;

    for url_result in url_results {
        match url_result {
            Ok(item) => {
                url_count += 1;
                result.add_item(item);
            }
            Err(e) => {
                error_count += 1;
                debug!(error = %e, "URL extraction error");
                // Extract the URL from the error for the skipped list
                match &e {
                    ParseError::InvalidUrl { url, .. } => {
                        result.add_skipped(url.clone());
                    }
                    ParseError::UrlTooLong { url_preview, .. } => {
                        result.add_skipped(url_preview.clone());
                    }
                }
            }
        }
    }

    info!(
        urls = url_count,
        errors = error_count,
        total = result.len(),
        skipped = result.skipped_count(),
        "Parsing complete"
    );

    result
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

        // Should find 2 URLs, ignore the rest
        assert_eq!(result.len(), 2);
        assert!(result.items[0].value.contains("arxiv.org"));
        assert!(result.items[1].value.contains("example.com"));
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
}
