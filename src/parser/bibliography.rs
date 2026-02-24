//! Bibliography extraction and per-entry parsing helpers.
//!
//! This module segments multi-line bibliography text into candidate entries,
//! parses each candidate with existing reference metadata logic, and exposes
//! summary helpers for downstream output.

use std::sync::LazyLock;

use regex::Regex;

use tracing::debug;

use super::input::ParsedItem;
use super::reference::{Confidence, parse_reference_metadata};

#[allow(clippy::expect_used)]
static NUMBERED_PREFIX_PATTERN: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^\s*(?:\[\d{1,3}\]|\d{1,3}[.)])\s*(.+)$")
        .expect("bibliography numbered prefix regex is valid")
});

#[allow(clippy::expect_used)]
static YEAR_PATTERN: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"\b(?:18|19|20)\d{2}\b").expect("bibliography year regex is valid")
});

#[allow(clippy::expect_used)]
static AUTHOR_START_PATTERN: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^[\p{Lu}][\p{L}'`\-]+,\s*(?:[\p{Lu}]\.|[\p{Lu}][\p{L}]+)")
        .expect("bibliography author-start regex is valid")
});

#[allow(clippy::expect_used)]
static YEAR_START_PATTERN: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^\(?\d{4}\)?\b").expect("bibliography year-start regex is valid")
});

/// Result of bibliography parsing.
#[derive(Debug, Clone, Default)]
pub struct BibliographyParseResult {
    /// Entries parsed into references.
    pub parsed: Vec<ParsedItem>,
    /// Reference-like entries that could not be confidently parsed.
    pub uncertain: Vec<String>,
    /// Total reference candidates found.
    pub total_found: usize,
}

impl BibliographyParseResult {
    /// Creates an empty parse result.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }
}

/// Aggregated summary counts for bibliography extraction.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BibliographySummary {
    /// Total candidates found.
    pub found: usize,
    /// Parsed references count.
    pub parsed: usize,
    /// Uncertain references count.
    pub uncertain: usize,
}

impl BibliographySummary {
    /// Returns the AC5 required summary format.
    #[must_use]
    pub fn format_message(&self) -> String {
        format!(
            "Found {} references ({} parsed, {} uncertain)",
            self.found, self.parsed, self.uncertain
        )
    }
}

/// Splits input into bibliography entry candidates.
#[tracing::instrument(skip(input), fields(input_len = input.len()))]
#[must_use]
pub fn extract_bibliography_entries(input: &str) -> Vec<String> {
    let mut blocks: Vec<Vec<String>> = Vec::new();
    let mut current_block: Vec<String> = Vec::new();

    for line in input.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            if !current_block.is_empty() {
                blocks.push(current_block);
                current_block = Vec::new();
            }
            continue;
        }

        current_block.push(trimmed.to_string());
    }

    if !current_block.is_empty() {
        blocks.push(current_block);
    }

    blocks
        .into_iter()
        .flat_map(|block| segment_block_entries(&block))
        .filter(|entry| is_reference_like_entry(entry))
        .collect()
}

/// Parses bibliography input into parsed and uncertain reference entries.
#[tracing::instrument(skip(input), fields(input_len = input.len()))]
#[must_use]
pub fn parse_bibliography(input: &str) -> BibliographyParseResult {
    let mut result = BibliographyParseResult::new();
    let entries = extract_bibliography_entries(input);

    for entry in entries {
        let metadata = parse_reference_metadata(&entry);
        if metadata.confidence != Confidence::Low
            || !metadata.authors.is_empty()
            || metadata.year.is_some()
        {
            result.parsed.push(ParsedItem::reference(&entry, &entry));
        } else {
            result
                .uncertain
                .push(format!("unparseable reference-like entry: {entry}"));
        }
    }

    result.total_found = result.parsed.len() + result.uncertain.len();
    debug!(
        total = result.total_found,
        parsed = result.parsed.len(),
        uncertain = result.uncertain.len(),
        "Bibliography parsing complete"
    );
    result
}

/// Returns summary counts for bibliography output.
#[tracing::instrument(skip(result), fields(total = result.total_found))]
#[must_use]
pub fn summarize_bibliography(result: &BibliographyParseResult) -> BibliographySummary {
    BibliographySummary {
        found: result.total_found,
        parsed: result.parsed.len(),
        uncertain: result.uncertain.len(),
    }
}

fn segment_block_entries(block_lines: &[String]) -> Vec<String> {
    let mut entries = Vec::new();
    let mut current = String::new();

    for line in block_lines {
        if should_ignore_line(line) {
            continue;
        }

        let (starts_numbered, content) = strip_numbered_prefix(line);
        if content.is_empty() {
            continue;
        }

        if starts_numbered {
            push_entry_if_any(&mut entries, &mut current);
            current.push_str(&content);
            continue;
        }

        if current.is_empty() {
            current.push_str(&content);
            continue;
        }

        if should_start_new_entry(&current, &content) {
            push_entry_if_any(&mut entries, &mut current);
            current.push_str(&content);
            continue;
        }

        current.push(' ');
        current.push_str(&content);
    }

    push_entry_if_any(&mut entries, &mut current);
    entries
}

fn push_entry_if_any(entries: &mut Vec<String>, current: &mut String) {
    let trimmed = current.trim();
    if !trimmed.is_empty() {
        entries.push(trimmed.to_string());
    }
    current.clear();
}

fn strip_numbered_prefix(line: &str) -> (bool, String) {
    if let Some(capture) = NUMBERED_PREFIX_PATTERN.captures(line)
        && let Some(content) = capture.get(1)
    {
        return (true, content.as_str().trim().to_string());
    }

    (false, line.trim().to_string())
}

fn should_ignore_line(line: &str) -> bool {
    let normalized = line
        .trim()
        .trim_end_matches(':')
        .trim()
        .to_ascii_lowercase();

    if matches!(
        normalized.as_str(),
        "references"
            | "bibliography"
            | "works cited"
            | "literature"
            | "sources"
            | "further reading"
            | "cited works"
            | "reference list"
    ) {
        return true;
    }

    line.chars()
        .all(|ch| ch.is_ascii_punctuation() || ch.is_whitespace())
}

fn is_reference_like_entry(entry: &str) -> bool {
    if entry.len() < 20 {
        return false;
    }

    let lower = entry.to_ascii_lowercase();
    let has_year = YEAR_PATTERN.is_match(entry);
    let comma_count = entry.matches(',').count();
    let period_count = entry.matches('.').count();
    let has_author_start = AUTHOR_START_PATTERN.is_match(entry);
    let has_keyword = lower.contains("et al.")
        || lower.contains("journal")
        || lower.contains("vol.")
        || lower.contains("pp.");

    (has_year && (has_author_start || comma_count >= 2 || has_keyword || period_count >= 2))
        || comma_count >= 3
        || (has_keyword && (has_author_start || comma_count >= 2 || period_count >= 2))
}

fn should_start_new_entry(current: &str, next_line: &str) -> bool {
    let starts_like_new =
        AUTHOR_START_PATTERN.is_match(next_line) || YEAR_START_PATTERN.is_match(next_line);
    let ends_sentence = current.trim_end().ends_with('.');

    starts_like_new && ends_sentence
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_bibliography_entries_numbered_lists() {
        let input =
            "1. Smith, J. (2024). A title. Journal.\n2) Jones, K. (2023). Another title. Journal.";
        let entries = extract_bibliography_entries(input);

        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0], "Smith, J. (2024). A title. Journal.");
        assert_eq!(entries[1], "Jones, K. (2023). Another title. Journal.");
    }

    #[test]
    fn test_extract_bibliography_entries_blank_line_multiline() {
        let input = "Smith, J. (2024). Title one.\nJournal Name, 1(2), 3-4.\n\nJones, K. (2023). Title two.\nAnother Journal, 5(6), 7-8.";
        let entries = extract_bibliography_entries(input);

        assert_eq!(entries.len(), 2);
        assert!(entries[0].contains("Journal Name"));
        assert!(entries[1].contains("Another Journal"));
    }

    #[test]
    fn test_extract_bibliography_entries_adjacent_unnumbered_entries() {
        let input = "Smith, J. (2024). Title one. Journal.\n2024. Journal Overview and Findings in Practice.";
        let entries = extract_bibliography_entries(input);

        assert_eq!(entries.len(), 2);
        assert!(
            entries[0].contains("Title one"),
            "First entry should contain 'Title one', got: {}",
            entries[0]
        );
        assert!(
            entries[1].contains("Journal Overview"),
            "Second entry should contain 'Journal Overview', got: {}",
            entries[1]
        );
    }

    #[test]
    fn test_extract_bibliography_entries_unicode_author_name() {
        let input = "García, J. (2024). Título del artículo. Revista Científica.";
        let entries = extract_bibliography_entries(input);

        assert_eq!(entries.len(), 1);
    }

    #[test]
    fn test_extract_bibliography_entries_wrapped_lines_joined() {
        let input = "1. Smith, J. (2024). A very long title\nthat wraps to next line.\nJournal Name, 1(2), 3-4.";
        let entries = extract_bibliography_entries(input);

        assert_eq!(entries.len(), 1);
        assert!(entries[0].contains("that wraps to next line."));
    }

    #[test]
    fn test_extract_bibliography_entries_filters_heading_and_separators() {
        let input = "References\n-----\n1. Smith, J. (2024). Title. Journal.";
        let entries = extract_bibliography_entries(input);

        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0], "Smith, J. (2024). Title. Journal.");
    }

    #[test]
    fn test_extract_bibliography_entries_rejects_non_bibliography_prose() {
        let input = "This is plain prose and should not become a bibliography entry because it has no citation structure.";
        let entries = extract_bibliography_entries(input);

        assert!(entries.is_empty());
    }

    #[test]
    fn test_extract_bibliography_entries_rejects_prose_with_keyword_only() {
        let input = "This was published in a journal of applied mechanics and is widely cited in the field.";
        let entries = extract_bibliography_entries(input);

        assert!(entries.is_empty());
    }

    #[test]
    fn test_extract_bibliography_entries_filters_works_cited_heading() {
        let input = "Works Cited:\n1. Smith, J. (2024). Title. Journal.";
        let entries = extract_bibliography_entries(input);

        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0], "Smith, J. (2024). Title. Journal.");
    }

    #[test]
    fn test_extract_bibliography_entries_rejects_prose_with_year() {
        let input =
            "In 2024 we conducted an internal review and this sentence is not a citation entry.";
        let entries = extract_bibliography_entries(input);

        assert!(entries.is_empty());
    }

    #[test]
    fn test_parse_bibliography_mixed_valid_uncertain_counts_match() {
        let input =
            "1. Smith, J. (2024). Valid title. Journal.\n2. foo, bar, baz, qux, quux, corge";
        let result = parse_bibliography(input);

        assert_eq!(result.parsed.len(), 1);
        assert_eq!(result.uncertain.len(), 1);
        assert_eq!(
            result.total_found,
            result.parsed.len() + result.uncertain.len()
        );
        assert!(result.uncertain[0].contains("unparseable reference-like entry"));
    }

    #[test]
    fn test_parse_bibliography_adjacent_entries_both_parsed() {
        let input = "Smith, J. (2024). Complete Title. Journal Name, 1(2), 3-4.\n2024. Journal Overview and Findings in Practice.";
        let result = parse_bibliography(input);

        assert_eq!(result.parsed.len(), 2);
        assert_eq!(result.uncertain.len(), 0);
    }

    #[test]
    fn test_summarize_bibliography_ac5_format_and_invariant() {
        let input =
            "1. Smith, J. (2024). Valid title. Journal.\n2. foo, bar, baz, qux, quux, corge";
        let result = parse_bibliography(input);
        let summary = summarize_bibliography(&result);

        assert_eq!(summary.found, summary.parsed + summary.uncertain);
        assert_eq!(
            summary.format_message(),
            "Found 2 references (1 parsed, 1 uncertain)"
        );
    }
}
