//! Integration tests for the parser module.
//!
//! These tests verify the parser's behavior with realistic inputs
//! and across module boundaries.

use downloader_core::parser::{InputType, parse_input};

/// Test parsing a realistic bibliography with URLs mixed in.
#[test]
fn test_parse_realistic_bibliography_with_urls() {
    let input = r#"
References:
1. https://arxiv.org/pdf/2301.00001.pdf
2. Smith, J. (2024). Paper Title. Journal.
3. https://example.com/papers/paper.pdf
4. Some other text that should be ignored.
5. doi:10.1234/example (DOI support coming in Epic 2)
"#;

    let result = parse_input(input);

    // Should find 2 URLs (other text ignored for now)
    let urls: Vec<_> = result.urls().collect();

    assert_eq!(urls.len(), 2, "Should extract exactly 2 URLs");
    assert!(
        urls[0].value.contains("arxiv.org"),
        "First URL should be arxiv"
    );
    assert!(
        urls[1].value.contains("example.com"),
        "Second URL should be example.com"
    );
}

/// Test parsing URLs embedded in markdown text.
#[test]
fn test_parse_urls_in_markdown() {
    let input = r#"
# Research Links

- [Paper 1](https://example.com/paper1.pdf) - Good paper
- [Paper 2](https://example.com/paper2.pdf) - Another one
- See also: https://arxiv.org/abs/2301.00001

## Notes

Check https://github.com/user/repo for code.
"#;

    let result = parse_input(input);

    // Should find 4 URLs
    assert_eq!(result.len(), 4);

    // All should be URLs
    assert!(result.items.iter().all(|i| i.input_type == InputType::Url));
}

/// Test parsing URLs from plain list (one per line).
#[test]
fn test_parse_url_list() {
    let input = r#"
https://example.com/doc1.pdf
https://example.com/doc2.pdf
https://example.com/doc3.pdf
https://example.com/doc4.pdf
https://example.com/doc5.pdf
"#;

    let result = parse_input(input);

    assert_eq!(result.len(), 5, "Should extract all 5 URLs");
    assert!(!result.is_empty());
    assert_eq!(result.skipped_count(), 0);
}

/// Test that URLs with various formats are handled.
#[test]
fn test_parse_various_url_formats() {
    let input = r#"
https://example.com/simple
https://example.com/path/to/file.pdf
https://example.com/search?q=test&page=1
https://example.com/page#section
https://user:pass@example.com/auth
https://localhost:8080/local
http://insecure.example.com/http
"#;

    let result = parse_input(input);

    assert_eq!(result.len(), 7, "Should extract all URL formats");
}

/// Test that order is preserved in multi-line input.
#[test]
fn test_order_preservation() {
    let input = "https://first.com\nhttps://second.com\nhttps://third.com";
    let result = parse_input(input);

    let values: Vec<_> = result.items.iter().map(|i| &i.value).collect();

    assert!(values[0].contains("first"), "First URL should be first");
    assert!(values[1].contains("second"), "Second URL should be second");
    assert!(values[2].contains("third"), "Third URL should be third");
}

/// Test handling of empty and whitespace-only input.
#[test]
fn test_empty_input_handling() {
    assert!(parse_input("").is_empty());
    assert!(parse_input("   ").is_empty());
    assert!(parse_input("\n\n\n").is_empty());
    assert!(parse_input("\t\t").is_empty());
}

/// Test that URLs embedded in sentences are extracted.
#[test]
fn test_urls_in_sentences() {
    let input = "Check out https://example.com/doc.pdf for more info.";
    let result = parse_input(input);

    assert_eq!(result.len(), 1);
    // The trailing period should be stripped
    assert!(result.items[0].value.ends_with(".pdf"));
}

/// Test mixed valid and invalid URLs.
#[test]
fn test_mixed_valid_invalid_urls() {
    // Note: The regex only matches http:// and https://, so ftp:// won't be found
    let input = r#"
https://valid.com/good.pdf
Some text without URLs
https://another-valid.com/also-good.pdf
"#;

    let result = parse_input(input);

    // Should find 2 valid URLs
    assert_eq!(result.len(), 2);
}

/// Test ParseResult display formatting.
#[test]
fn test_parse_result_display() {
    let result = parse_input("https://a.com https://b.com https://c.com");

    let display = result.to_string();
    assert!(
        display.contains("3 items"),
        "Display should show item count"
    );
}

/// Test URLs from academic sources.
#[test]
fn test_academic_urls() {
    let input = r#"
https://arxiv.org/abs/2301.00001
https://arxiv.org/pdf/2301.00001.pdf
https://www.sciencedirect.com/science/article/pii/S0123456789012345
https://doi.org/10.1234/example
https://pubmed.ncbi.nlm.nih.gov/12345678/
"#;

    let result = parse_input(input);

    // Should extract all these academic URLs
    assert_eq!(result.len(), 5);
}

/// Test that very long URLs are rejected gracefully.
#[test]
fn test_very_long_url_rejected() {
    // Create a URL exceeding the 2000 char limit
    let long_path = "a".repeat(2500);
    let input = format!("https://example.com/{}", long_path);

    let result = parse_input(&input);

    // URL should be skipped (added to skipped list), not included in items
    assert_eq!(result.len(), 0, "Too-long URL should not be in items");
    assert_eq!(result.skipped_count(), 1, "Too-long URL should be skipped");
}
