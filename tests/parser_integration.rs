//! Integration tests for the parser module.
//!
//! These tests verify the parser's behavior with realistic inputs
//! and across module boundaries.

use downloader_core::parser::{
    Confidence, ConfidenceFactors, InputType, extract_reference_confidence, parse_bibliography,
    parse_input, parse_reference_metadata, summarize_bibliography,
};

/// Test parsing a realistic bibliography with URLs and DOIs mixed in.
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

    let urls: Vec<_> = result.urls().collect();
    let dois: Vec<_> = result.dois().collect();

    assert_eq!(urls.len(), 2, "Should extract exactly 2 URLs");
    assert_eq!(dois.len(), 1, "Should extract exactly 1 DOI");
    assert!(
        urls[0].value.contains("arxiv.org"),
        "First URL should be arxiv"
    );
    assert!(
        urls[1].value.contains("example.com"),
        "Second URL should be example.com"
    );
    assert_eq!(dois[0].value, "10.1234/example");
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

/// Test URLs from academic sources - doi.org URL classified as DOI.
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

    // Total items: 4 URLs + 1 DOI
    assert_eq!(result.len(), 5);

    let urls: Vec<_> = result.urls().collect();
    let dois: Vec<_> = result.dois().collect();
    assert_eq!(urls.len(), 4, "4 non-DOI URLs");
    assert_eq!(dois.len(), 1, "doi.org URL classified as DOI");
    assert_eq!(dois[0].value, "10.1234/example");
}

// ==================== DOI Integration Tests ====================

/// Test DOIs detected in mixed input with correct counts.
#[test]
fn test_parse_input_detects_doi_in_mixed_input() {
    let input = r#"
https://example.com/paper.pdf
10.1038/s41586-024-07386-0
https://another.com/doc.pdf
DOI: 10.1016/j.cell.2024.01.001
"#;
    let result = parse_input(input);

    let urls: Vec<_> = result.urls().collect();
    let dois: Vec<_> = result.dois().collect();

    assert_eq!(urls.len(), 2, "Should find 2 URLs");
    assert_eq!(dois.len(), 2, "Should find 2 DOIs");
}

/// Test doi.org URL classified as DOI, not URL.
#[test]
fn test_parse_input_doi_url_classified_as_doi() {
    let result = parse_input("https://doi.org/10.1234/example");

    assert_eq!(result.len(), 1);
    let item = &result.items[0];
    assert_eq!(item.input_type, InputType::Doi, "doi.org URL should be DOI");
    assert_eq!(item.value, "10.1234/example");
}

/// Test non-DOI URLs remain as URL type.
#[test]
fn test_parse_input_non_doi_url_still_url() {
    let result = parse_input("https://example.com/paper.pdf");

    assert_eq!(result.len(), 1);
    assert_eq!(result.items[0].input_type, InputType::Url);
}

/// Test dois() iterator returns only DOI items.
#[test]
fn test_parse_input_dois_iterator() {
    let input = "https://example.com/doc.pdf 10.1234/example https://other.com";
    let result = parse_input(input);

    let dois: Vec<_> = result.dois().collect();
    assert_eq!(dois.len(), 1);
    assert_eq!(dois[0].input_type, InputType::Doi);
    assert_eq!(dois[0].value, "10.1234/example");
}

/// Test duplicate DOI is de-duplicated by canonical DOI value.
#[test]
fn test_parse_input_duplicate_doi_deduplicated() {
    let input = "10.1234/example\n10.1234/example";
    let result = parse_input(input);

    let dois: Vec<_> = result.dois().collect();
    assert_eq!(dois.len(), 1, "Duplicate DOIs should be de-duplicated");
    assert_eq!(dois[0].value, "10.1234/example");
}

/// Test realistic bibliography with DOIs, URLs, and plain text.
#[test]
fn test_parse_input_bibliography_with_dois() {
    let input = r#"
References:
[1] Zhang, Y. et al. (2024). Neural networks. Nature, 620, 47-53.
    https://doi.org/10.1038/s41586-024-07386-0
[2] Smith, J. (2024). Cell biology advances. Cell, 187(1), 1-15.
    DOI: 10.1016/j.cell.2024.01.001
[3] Available at https://arxiv.org/pdf/2301.00001.pdf
[4] See also 10.1371/journal.pone.0123456
"#;

    let result = parse_input(input);

    let urls: Vec<_> = result.urls().collect();
    let dois: Vec<_> = result.dois().collect();

    assert_eq!(urls.len(), 1, "Should find 1 plain URL (arxiv)");
    assert_eq!(dois.len(), 3, "Should find 3 DOIs");
    assert!(urls[0].value.contains("arxiv.org"));
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

// ==================== Reference Integration Tests ====================

#[test]
fn test_parse_input_reference_string_recognized() {
    let input = "Smith, J. (2024). Paper Title. Journal Name, 1(2), 3-4.";
    let result = parse_input(input);

    let references: Vec<_> = result.references().collect();
    assert_eq!(references.len(), 1, "Should detect one reference");
    assert_eq!(references[0].input_type, InputType::Reference);
    assert_eq!(
        references[0].value,
        "Smith, J. (2024). Paper Title. Journal Name, 1(2), 3-4."
    );
}

#[test]
fn test_parse_input_mixed_urls_dois_references() {
    let input = r#"
https://example.com/paper.pdf
doi:10.1234/example
Smith, J. (2024). Paper Title. Journal Name, 1(2), 3-4.
"#;
    let result = parse_input(input);

    assert_eq!(result.urls().count(), 1, "Should extract URL");
    assert_eq!(result.dois().count(), 1, "Should extract DOI");
    assert_eq!(result.references().count(), 1, "Should extract reference");
}

#[test]
fn test_parse_input_reference_confidence_levels() {
    let input = r#"
Smith, J. (2024). Complete Title. Journal Name, 1(2), 3-4.
2024. Journal Overview and Findings in Practice.
"#;

    let result = parse_input(input);
    let mut confidences = result
        .references()
        .map(|item| parse_reference_metadata(&item.value).confidence)
        .collect::<Vec<_>>();

    confidences.sort_by_key(|value| match value {
        Confidence::High => 0,
        Confidence::Medium => 1,
        Confidence::Low => 2,
    });

    assert!(
        confidences.contains(&Confidence::High),
        "Expected at least one High confidence reference"
    );
    assert!(
        confidences.iter().any(|value| *value != Confidence::High),
        "Expected at least one non-High confidence reference, got: {:?}",
        confidences
    );
}

#[test]
fn test_parse_reference_confidence_exposes_deterministic_factors() {
    let details = extract_reference_confidence("Smith, J. (2024). Complete Title. Journal Name.");
    assert_eq!(details.level, Confidence::High);
    assert_eq!(
        details.factors,
        ConfidenceFactors {
            has_authors: true,
            has_year: true,
            has_title: true,
            author_count: 1
        }
    );

    let weak = extract_reference_confidence("2024");
    assert_eq!(weak.level, Confidence::Low);
    assert_eq!(weak.factors.has_year, true);
    assert_eq!(weak.factors.has_title, false);
    assert_eq!(weak.factors.has_authors, false);
}

#[test]
fn test_parse_input_unparseable_reference_skipped() {
    let input = "foo, bar, baz, qux, quux, corge";
    let result = parse_input(input);

    assert_eq!(
        result.references().count(),
        0,
        "No parseable references expected"
    );
    assert_eq!(
        result.skipped_count(),
        1,
        "Reference-like but unparseable line should be skipped"
    );
}

#[test]
fn test_parse_input_bibliography_numbered_entries() {
    let input = "References\n1. Smith, J. (2024). Title One. Journal.\n2) Jones, K. (2023). Title Two. Journal.";
    let result = parse_input(input);

    assert_eq!(result.references().count(), 2);
}

#[test]
fn test_parse_input_bibliography_blank_line_entries() {
    let input = "Smith, J. (2024). Title one.\nJournal Name, 1(2), 3-4.\n\nJones, K. (2023). Title two.\nAnother Journal, 5(6), 7-8.";
    let result = parse_input(input);

    assert_eq!(result.references().count(), 2);
}

#[test]
fn test_parse_input_bibliography_summary_counts() {
    let input = "1. Smith, J. (2024). Valid title. Journal.\n2. foo, bar, baz, qux, quux, corge";
    let parsed = parse_bibliography(input);
    let summary = summarize_bibliography(&parsed);

    assert_eq!(summary.found, summary.parsed + summary.uncertain);
    assert_eq!(
        summary.format_message(),
        "Found 2 references (1 parsed, 1 uncertain)"
    );
}

#[test]
fn test_parse_input_bibliography_with_doi_url_mixture() {
    let input = "References:\n1. Smith, J. (2024). Title. Journal. https://doi.org/10.1234/example\n2. Jones, K. (2023). Follow-up Title. Journal. https://example.com/paper.pdf";
    let result = parse_input(input);

    assert_eq!(result.dois().count(), 1);
    assert_eq!(result.urls().count(), 1);
    assert_eq!(result.references().count(), 2);
}

#[test]
fn test_parse_input_bibtex_only_entry() {
    let input = r#"@article{key, title={BibTeX Title}, author={Smith, J. and Doe, R.}, year={2024}, doi={10.1234/example}}"#;
    let result = parse_input(input);

    assert_eq!(result.dois().count(), 1);
    assert_eq!(result.references().count(), 1);
    assert_eq!(result.skipped_count(), 0);
}

#[test]
fn test_parse_input_mixed_order_contract_url_doi_reference_then_bibtex() {
    let input = r#"
https://example.com/paper.pdf
10.1111/alpha
Smith, J. (2024). Existing Reference. Journal.
@article{key, title={BibTeX Title}, author={Doe, R.}, year={2023}, doi={10.2222/beta}}
"#;
    let result = parse_input(input);

    assert_eq!(result.items.len(), 6);
    assert_eq!(result.items[0].input_type, InputType::Doi);
    assert_eq!(result.items[1].input_type, InputType::Doi);
    assert_eq!(result.items[2].input_type, InputType::Url);
    assert_eq!(result.items[3].input_type, InputType::Reference);
    assert_eq!(result.items[4].input_type, InputType::BibTex);
    assert_eq!(result.items[5].input_type, InputType::Reference);
}

#[test]
fn test_parse_input_bibtex_doi_deduplication_against_existing_extractor() {
    let input = r#"
10.1234/shared
@article{key, title={BibTeX Title}, author={Smith, J.}, year={2024}, doi={10.1234/shared}}
"#;
    let result = parse_input(input);

    let dois: Vec<_> = result.dois().collect();
    assert_eq!(dois.len(), 1);
    assert_eq!(dois[0].value, "10.1234/shared");
}

#[test]
fn test_parse_input_type_counts_include_bibtex_and_total_accounting() {
    let input = r#"
https://example.com/a.pdf
10.1000/test
Smith, J. (2024). Existing Reference. Journal.
@article{key, title={BibTeX Title}, author={Doe, R.}, year={2023}, doi={10.2000/bib}}
"#;

    let result = parse_input(input);
    let counts = result.type_counts();

    assert_eq!(counts.urls, 1);
    assert_eq!(counts.dois, 2);
    assert_eq!(counts.references, 2);
    assert_eq!(counts.bibtex, 1);
    assert_eq!(counts.total(), result.len());
    assert_eq!(
        result.len() + result.skipped_count(),
        counts.total() + result.skipped_count()
    );
}

#[test]
fn test_parse_input_bibtex_malformed_isolation_keeps_valid_neighbor() {
    let input = r#"
@article{bad, title={Broken}, year={2024}
@article{ok, title={Good Title}, author={Smith, J.}, year={2024}, doi={10.1234/good}}
"#;
    let result = parse_input(input);

    assert_eq!(result.dois().count(), 1);
    assert_eq!(result.references().count(), 1);
    assert!(
        result
            .skipped
            .iter()
            .any(|line| line.contains("malformed BibTeX")),
        "Expected actionable malformed BibTeX message"
    );
}
