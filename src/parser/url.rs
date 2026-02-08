//! URL extraction and validation from text input.

use std::sync::LazyLock;

use regex::Regex;
use tracing::{debug, trace};
use url::Url;

use super::error::{MAX_URL_LENGTH, ParseError};
use super::input::ParsedItem;

/// Regex pattern for finding URLs in text.
/// Matches http:// and https:// URLs, capturing until whitespace or common delimiters.
#[allow(clippy::expect_used)]
static URL_PATTERN: LazyLock<Regex> = LazyLock::new(|| {
    // Match http:// or https:// followed by non-whitespace, non-angle-bracket, non-quote chars
    // This handles URLs embedded in text, HTML, markdown, etc.
    Regex::new(r#"https?://[^\s<>"'\]]+"#).expect("URL regex is valid") // Static pattern, safe to panic
});

/// Result type for URL extraction operations.
pub type UrlExtractionResult = Result<ParsedItem, ParseError>;

/// Extracts and validates URLs from text input.
///
/// This function finds all HTTP/HTTPS URLs in the input text, validates them,
/// and returns a list of results. Each URL is individually validated, so some
/// may succeed while others fail.
///
/// # Arguments
///
/// * `input` - Text input that may contain URLs mixed with other content
///
/// # Returns
///
/// A vector of results, one per URL candidate found. Each result is either:
/// - `Ok(ParsedItem)` - A valid URL was extracted and normalized
/// - `Err(ParseError)` - The URL was invalid (malformed, wrong scheme, etc.)
///
/// # Examples
///
/// ```
/// use downloader_core::parser::extract_urls;
///
/// let results = extract_urls("Check out https://example.com/doc.pdf for details");
/// assert_eq!(results.len(), 1);
/// assert!(results[0].is_ok());
/// ```
#[tracing::instrument(skip(input), fields(input_len = input.len()))]
#[must_use]
pub fn extract_urls(input: &str) -> Vec<UrlExtractionResult> {
    let mut results = Vec::new();

    for url_match in URL_PATTERN.find_iter(input) {
        let raw_url = url_match.as_str();
        // Clean up trailing punctuation that might have been captured
        let cleaned = clean_url_trailing(raw_url);
        trace!(url = %cleaned, "found URL candidate");

        match validate_url(cleaned) {
            Ok(validated) => {
                debug!(url = %validated, "URL validated");
                results.push(Ok(ParsedItem::url(raw_url, validated)));
            }
            Err(e) => {
                debug!(url = %cleaned, error = %e, "URL validation failed");
                results.push(Err(e));
            }
        }
    }

    results
}

/// Cleans trailing punctuation that often gets captured with URLs.
fn clean_url_trailing(url: &str) -> &str {
    // Common trailing chars that aren't part of URLs when embedded in text
    let mut result = url;

    // Handle trailing punctuation, but preserve valid URL chars
    while let Some(last) = result.chars().last() {
        match last {
            // These are often sentence-ending punctuation, not part of URL
            '.' | ',' | ';' | ':' | '!' | '?' => {
                // But don't strip if it looks like a file extension
                if last == '.' {
                    // Check if this looks like a file extension (1-5 alphanumeric chars after last dot)
                    if let Some(dot_pos) = result.rfind('.') {
                        let after_dot = &result[dot_pos + 1..];
                        let ext_len = after_dot.len();
                        // Valid extensions are 1-5 chars, all alphanumeric
                        if (1..=5).contains(&ext_len)
                            && after_dot.chars().all(|c| c.is_ascii_alphanumeric())
                        {
                            break; // Keep the dot, it's likely part of filename
                        }
                    }
                }
                result = &result[..result.len() - 1];
            }
            // Closing parens/brackets at end are usually not part of URL
            ')' | ']' => {
                // Unless there's a matching opener in the URL (like Wikipedia URLs)
                let open = if last == ')' { '(' } else { '[' };
                let open_count = result.chars().filter(|&c| c == open).count();
                let close_count = result.chars().filter(|&c| c == last).count();
                if close_count > open_count {
                    result = &result[..result.len() - 1];
                } else {
                    break;
                }
            }
            _ => break,
        }
    }

    result
}

/// Validates a URL string and normalizes it.
///
/// # Validation rules:
/// - Must not exceed `MAX_URL_LENGTH` (2000 chars)
/// - Must be parseable by the `url` crate
/// - Must use http or https scheme (no ftp, file, etc.)
/// - Must have a host (domain or IP)
fn validate_url(raw: &str) -> Result<String, ParseError> {
    // Check URL length first (prevents memory issues with very long URLs)
    if raw.len() > MAX_URL_LENGTH {
        return Err(ParseError::too_long(raw));
    }

    // Parse with url crate for full validation
    let parsed = Url::parse(raw).map_err(|e| ParseError::malformed(raw, &e.to_string()))?;

    // Only allow http and https
    match parsed.scheme() {
        "http" | "https" => {}
        scheme => return Err(ParseError::unsupported_scheme(raw, scheme)),
    }

    // Must have a host
    if parsed.host().is_none() {
        return Err(ParseError::no_host(raw));
    }

    // Return the parsed URL as string (normalized)
    Ok(parsed.to_string())
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::super::input::InputType;
    use super::*;

    // ==================== AC1: HTTP/HTTPS URL Extraction ====================

    #[test]
    fn test_extract_urls_single_http() {
        let input = "http://example.com/file.pdf";
        let results = extract_urls(input);
        assert_eq!(results.len(), 1);
        assert!(results[0].is_ok());
        let item = results[0].as_ref().unwrap();
        assert_eq!(item.input_type, InputType::Url);
        assert_eq!(item.value, "http://example.com/file.pdf");
    }

    #[test]
    fn test_extract_urls_single_https() {
        let input = "https://example.com/paper.pdf";
        let results = extract_urls(input);
        assert_eq!(results.len(), 1);
        assert!(results[0].is_ok());
        let item = results[0].as_ref().unwrap();
        assert_eq!(item.value, "https://example.com/paper.pdf");
    }

    #[test]
    fn test_extract_urls_validates_structure() {
        // Valid URL with scheme and host
        let results = extract_urls("https://example.com");
        assert_eq!(results.len(), 1);
        assert!(results[0].is_ok());

        // URL is normalized (trailing slash added)
        let item = results[0].as_ref().unwrap();
        assert_eq!(item.value, "https://example.com/");
    }

    // ==================== AC2: Invalid URL Reporting ====================

    #[test]
    fn test_validate_url_rejects_ftp() {
        let result = validate_url("ftp://files.example.com/file.pdf");
        assert!(result.is_err());
        let err = result.unwrap_err();
        if let ParseError::InvalidUrl {
            reason, suggestion, ..
        } = err
        {
            assert!(reason.contains("ftp"), "should mention ftp scheme");
            assert!(suggestion.contains("http"), "should suggest http");
        } else {
            panic!("Expected InvalidUrl error");
        }
    }

    #[test]
    fn test_validate_url_rejects_file() {
        let result = validate_url("file:///home/user/doc.pdf");
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_url_rejects_mailto() {
        let result = validate_url("mailto:user@example.com");
        assert!(result.is_err());
    }

    // ==================== AC3: Non-URL Text Handling ====================

    #[test]
    fn test_extract_urls_no_urls() {
        let input = "This is just plain text with no URLs";
        let results = extract_urls(input);
        assert!(results.is_empty());
    }

    #[test]
    fn test_extract_urls_empty_input() {
        let input = "";
        let results = extract_urls(input);
        assert!(results.is_empty());
    }

    #[test]
    fn test_extract_urls_mixed_text() {
        let input = "Check out https://example.com/paper.pdf for details";
        let results = extract_urls(input);
        assert_eq!(results.len(), 1);
        assert!(results[0].is_ok());
    }

    #[test]
    fn test_extract_urls_ignores_partial_urls() {
        let input = "Go to example.com for more info"; // No scheme
        let results = extract_urls(input);
        assert!(results.is_empty(), "should not match URLs without scheme");
    }

    // ==================== AC4: ParsedInput Result ====================

    #[test]
    fn test_extract_urls_returns_parsed_item() {
        let input = "https://example.com/doc.pdf";
        let results = extract_urls(input);

        let item = results[0].as_ref().unwrap();
        assert!(!item.raw.is_empty(), "should have raw input");
        assert_eq!(item.input_type, InputType::Url);
        assert!(!item.value.is_empty(), "should have normalized value");
    }

    // ==================== AC5: Multi-Line Input Support ====================

    #[test]
    fn test_extract_urls_multiple_lines() {
        let input = "https://example.com/a.pdf\nhttps://example.com/b.pdf";
        let results = extract_urls(input);
        assert_eq!(results.len(), 2);
        assert!(results.iter().all(|r| r.is_ok()));
    }

    #[test]
    fn test_extract_urls_space_separated() {
        let input = "https://a.com/1.pdf https://b.com/2.pdf https://c.com/3.pdf";
        let results = extract_urls(input);
        assert_eq!(results.len(), 3);
    }

    #[test]
    fn test_extract_urls_preserves_order() {
        let input = "https://first.com\nhttps://second.com\nhttps://third.com";
        let results = extract_urls(input);
        let urls: Vec<_> = results
            .iter()
            .filter_map(|r| r.as_ref().ok())
            .map(|item| item.value.as_str())
            .collect();
        assert_eq!(urls[0], "https://first.com/");
        assert_eq!(urls[1], "https://second.com/");
        assert_eq!(urls[2], "https://third.com/");
    }

    // ==================== Edge Cases ====================

    #[test]
    fn test_extract_urls_with_query_string() {
        let input = "https://example.com/search?q=rust&page=1";
        let results = extract_urls(input);
        assert_eq!(results.len(), 1);
        let item = results[0].as_ref().unwrap();
        assert!(item.value.contains("q=rust"));
        assert!(item.value.contains("page=1"));
    }

    #[test]
    fn test_extract_urls_with_fragment() {
        let input = "https://example.com/page#section";
        let results = extract_urls(input);
        assert_eq!(results.len(), 1);
        let item = results[0].as_ref().unwrap();
        assert!(item.value.contains("#section"));
    }

    #[test]
    fn test_extract_urls_handles_url_encoded() {
        let input = "https://example.com/path/to/caf%C3%A9.pdf";
        let results = extract_urls(input);
        assert_eq!(results.len(), 1);
        assert!(results[0].is_ok());
        // Verify encoded characters are preserved in output
        let item = results[0].as_ref().unwrap();
        assert!(
            item.value.contains("%C3%A9"),
            "URL-encoded chars should be preserved"
        );
    }

    #[test]
    fn test_validate_url_rejects_too_long() {
        let long_url = "https://example.com/".to_string() + &"a".repeat(2500);
        let result = validate_url(&long_url);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(matches!(err, ParseError::UrlTooLong { .. }));
    }

    #[test]
    fn test_validate_url_accepts_max_length() {
        // Just under the limit should work
        let url = "https://example.com/".to_string() + &"a".repeat(1970);
        assert!(url.len() < 2000);
        let result = validate_url(&url);
        assert!(result.is_ok());
    }

    #[test]
    fn test_extract_urls_strips_trailing_punctuation() {
        let input = "See https://example.com/doc.pdf.";
        let results = extract_urls(input);
        assert_eq!(results.len(), 1);
        let item = results[0].as_ref().unwrap();
        // Should keep .pdf but strip trailing period
        assert!(item.value.ends_with(".pdf"), "value: {}", item.value);
    }

    #[test]
    fn test_extract_urls_handles_parentheses_in_text() {
        let input = "(see https://example.com/doc.pdf)";
        let results = extract_urls(input);
        assert_eq!(results.len(), 1);
        let item = results[0].as_ref().unwrap();
        // Should not include trailing )
        assert!(!item.value.ends_with(')'), "should strip trailing paren");
    }

    #[test]
    fn test_extract_urls_preserves_wikipedia_style_parens() {
        let input = "https://en.wikipedia.org/wiki/URL_(disambiguation)";
        let results = extract_urls(input);
        assert_eq!(results.len(), 1);
        let item = results[0].as_ref().unwrap();
        // Wikipedia URLs have matched parens, should preserve
        assert!(item.value.contains("(disambiguation)"));
    }

    #[test]
    fn test_extract_urls_with_port() {
        let input = "https://localhost:8080/path";
        let results = extract_urls(input);
        assert_eq!(results.len(), 1);
        let item = results[0].as_ref().unwrap();
        assert!(item.value.contains(":8080"));
    }

    #[test]
    fn test_extract_urls_with_auth() {
        let input = "https://user:pass@example.com/secure";
        let results = extract_urls(input);
        assert_eq!(results.len(), 1);
        assert!(results[0].is_ok());
    }

    #[test]
    fn test_clean_url_trailing_preserves_file_extensions() {
        // Standard 3-4 char extensions
        assert_eq!(
            clean_url_trailing("https://example.com/file.pdf"),
            "https://example.com/file.pdf"
        );
        assert_eq!(
            clean_url_trailing("https://example.com/file.html"),
            "https://example.com/file.html"
        );
        assert_eq!(
            clean_url_trailing("https://example.com/file.docx"),
            "https://example.com/file.docx"
        );
        // Short extensions (1-2 chars)
        assert_eq!(
            clean_url_trailing("https://example.com/file.gz"),
            "https://example.com/file.gz"
        );
        assert_eq!(
            clean_url_trailing("https://example.com/file.md"),
            "https://example.com/file.md"
        );
        assert_eq!(
            clean_url_trailing("https://example.com/file.c"),
            "https://example.com/file.c"
        );
        // Longer extensions (5 chars)
        assert_eq!(
            clean_url_trailing("https://example.com/file.xhtml"),
            "https://example.com/file.xhtml"
        );
    }

    #[test]
    fn test_clean_url_trailing_strips_sentence_punctuation() {
        assert_eq!(
            clean_url_trailing("https://example.com,"),
            "https://example.com"
        );
        assert_eq!(
            clean_url_trailing("https://example.com;"),
            "https://example.com"
        );
        assert_eq!(
            clean_url_trailing("https://example.com!"),
            "https://example.com"
        );
        assert_eq!(
            clean_url_trailing("https://example.com?"),
            "https://example.com"
        );
    }
}
