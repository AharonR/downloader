//! RIS bibliography format parser.
//!
//! Parses RIS (Research Information Systems) format files into structured
//! items suitable for the download pipeline. RIS is a line-oriented tagged
//! format where each record starts with `TY  - ` and ends with `ER  - `.
//!
//! # Supported tags
//!
//! | Tag | Field       |
//! |-----|-------------|
//! | TY  | Entry type  |
//! | DO  | DOI         |
//! | UR  | URL         |
//! | TI  | Title       |
//! | AU  | Author      |
//! | PY  | Year        |
//! | ER  | End record  |
//!
//! # Example
//!
//! ```
//! use downloader_core::parser::parse_ris_content;
//!
//! let ris = "TY  - JOUR\nTI  - A Title\nDO  - 10.1234/example\nER  - \n";
//! let result = parse_ris_content(ris);
//! assert_eq!(result.entries.len(), 1);
//! assert_eq!(result.entries[0].doi.as_deref(), Some("10.1234/example"));
//! ```

use std::sync::LazyLock;

use regex::Regex;
use tracing::{debug, warn};

use super::doi::extract_dois;
use super::input::{InputType, ParsedItem};

#[allow(clippy::expect_used)]
static YEAR_PATTERN: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\b(?:18|19|20)\d{2}\b").expect("ris year regex is valid"));

/// A parsed RIS reference entry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RisEntry {
    /// RIS entry type (e.g. `JOUR`, `BOOK`, `CONF`).
    pub entry_type: String,
    /// Extracted DOI, normalized to bare format when present.
    pub doi: Option<String>,
    /// URL extracted from the `UR` tag when present.
    pub url: Option<String>,
    /// Title from the `TI` tag when present.
    pub title: Option<String>,
    /// Comma-joined authors from one or more `AU` tags when present.
    pub authors: Option<String>,
    /// Four-digit year from the `PY` tag when present.
    pub year: Option<u16>,
    /// Raw original text for this entry.
    pub raw: String,
}

/// Batch parse result for RIS input.
#[derive(Debug, Clone, Default)]
pub struct RisParseResult {
    /// Structured parsed entries.
    pub entries: Vec<RisEntry>,
    /// Items mapped for the download pipeline.
    pub items: Vec<ParsedItem>,
    /// Actionable skip / error messages (What/Why/Fix format).
    pub skipped: Vec<String>,
    /// Total `TY  - ` records found before validation.
    pub total_found: usize,
}

impl RisParseResult {
    /// Creates a new empty result.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }
}

/// Parses RIS format content and extracts entries with DOIs, URLs, and metadata.
///
/// Empty input returns an empty result (not an error). Records that cannot be
/// identified (no `TY` tag, missing `ER` terminator) produce a skip message
/// in `result.skipped` and do not panic.
///
/// DOI takes priority over URL: when both are present, the entry yields a DOI
/// item. The URL item is only emitted when no DOI is available.
#[tracing::instrument(skip(input), fields(input_len = input.len()))]
#[must_use]
pub fn parse_ris_content(input: &str) -> RisParseResult {
    let mut result = RisParseResult::new();

    if input.trim().is_empty() {
        debug!("Empty RIS input");
        return result;
    }

    let raw_segments = segment_records(input);
    result.total_found = raw_segments.len();

    for raw in &raw_segments {
        match parse_record(raw) {
            RecordOutcome::Parsed(entry) => {
                emit_items_for_entry(&mut result, entry);
            }
            RecordOutcome::Skip(message) => {
                warn!(message = %message, "Skipped RIS record");
                result.skipped.push(message);
            }
        }
    }

    debug!(
        total_found = result.total_found,
        entries = result.entries.len(),
        items = result.items.len(),
        skipped = result.skipped.len(),
        "RIS parsing complete"
    );

    result
}

// ---------------------------------------------------------------------------
// Internal helpers
// ---------------------------------------------------------------------------

#[derive(Debug)]
enum RecordOutcome {
    Parsed(RisEntry),
    Skip(String),
}

/// Splits input into per-record raw text blocks delimited by `TY  - ` … `ER  - `.
fn segment_records(input: &str) -> Vec<String> {
    let mut segments = Vec::new();
    let mut current: Vec<&str> = Vec::new();
    let mut in_record = false;

    for line in input.lines() {
        let trimmed = line.trim_end();

        if is_tag(trimmed, "TY") {
            // Start a new record; discard any accumulated non-record lines.
            if in_record && !current.is_empty() {
                // Unterminated previous record — keep it for error reporting.
                segments.push(current.join("\n"));
                current.clear();
            }
            in_record = true;
            current.push(trimmed);
            continue;
        }

        if is_tag(trimmed, "ER") {
            if in_record {
                current.push(trimmed);
                segments.push(current.join("\n"));
                current.clear();
                in_record = false;
            }
            // ER outside of a record is silently ignored.
            continue;
        }

        if in_record {
            current.push(trimmed);
        }
    }

    // Any remaining open record (missing ER) is captured as an unterminated segment.
    if in_record && !current.is_empty() {
        segments.push(current.join("\n"));
    }

    segments
}

/// Returns true when the line has the RIS tag prefix `TAG  -` (standard: `TAG  - value`).
///
/// RIS tags are two letters followed by two spaces, a dash, and (optionally) a space and value.
/// The `ER  - ` end-of-record marker has no value; after `trim_end()` it becomes `ER  -`.
/// This function accepts both forms so that value-less tags are recognised reliably.
fn is_tag(line: &str, tag: &str) -> bool {
    // The normalised prefix without the trailing space covers both
    // "TAG  - VALUE" and "TAG  -" (after trim_end strips the trailing space from "TAG  - ").
    let prefix = format!("{tag}  -");
    if !line.starts_with(prefix.as_str()) {
        return false;
    }
    // Ensure the character after "TAG  -" (if any) is a space or end-of-string.
    matches!(line[prefix.len()..].chars().next(), None | Some(' '))
}

/// Extracts the value after the `TAG  - ` prefix, trimming whitespace.
fn tag_value<'a>(line: &'a str, tag: &str) -> Option<&'a str> {
    let prefix = format!("{tag}  - ");
    line.strip_prefix(prefix.as_str()).map(str::trim)
}

fn parse_record(raw: &str) -> RecordOutcome {
    // Require a properly terminated record (ER line present).
    let has_er = raw.lines().any(|l| is_tag(l.trim_end(), "ER"));

    // Collect fields from each line.
    let mut entry_type = String::new();
    let mut doi: Option<String> = None;
    let mut url: Option<String> = None;
    let mut title: Option<String> = None;
    let mut authors: Vec<String> = Vec::new();
    let mut year: Option<u16> = None;

    for line in raw.lines() {
        let line = line.trim_end();

        if let Some(v) = tag_value(line, "TY") {
            entry_type = v.to_string();
        } else if let Some(v) = tag_value(line, "DO") {
            if doi.is_none() {
                doi = normalize_doi(v);
            }
        } else if let Some(v) = tag_value(line, "UR") {
            if url.is_none() && !v.is_empty() {
                url = Some(v.to_string());
            }
        } else if let Some(v) = tag_value(line, "TI") {
            if title.is_none() && !v.is_empty() {
                title = Some(v.to_string());
            }
        } else if let Some(v) = tag_value(line, "AU") {
            if !v.is_empty() {
                authors.push(v.to_string());
            }
        } else if let Some(v) = tag_value(line, "PY") {
            if year.is_none() {
                year = normalize_year(v);
            }
        }
    }

    if entry_type.is_empty() {
        return RecordOutcome::Skip(
            "What: RIS record missing TY (type) tag. \
             Why: every RIS record must start with 'TY  - TYPE'. \
             Fix: ensure the file begins each record with a TY  - line."
                .to_string(),
        );
    }

    if !has_er {
        return RecordOutcome::Skip(format!(
            "What: RIS record for type '{entry_type}' is missing its ER  -  terminator. \
             Why: the record may be truncated or the file is malformed. \
             Fix: ensure each record ends with an 'ER  - ' line."
        ));
    }

    let authors_str = if authors.is_empty() {
        None
    } else {
        Some(authors.join(", "))
    };

    RecordOutcome::Parsed(RisEntry {
        entry_type,
        doi,
        url,
        title,
        authors: authors_str,
        year,
        raw: raw.to_string(),
    })
}

/// Emits `ParsedItem`s for the entry into the result, following the DOI-over-URL priority rule.
fn emit_items_for_entry(result: &mut RisParseResult, entry: RisEntry) {
    // Build a human-readable reference value for the Reference item.
    let reference_value = build_reference_value(&entry);

    if let Some(ref doi) = entry.doi {
        result.items.push(ParsedItem::new(
            entry.raw.clone(),
            InputType::Doi,
            doi.clone(),
        ));
    } else if let Some(ref url) = entry.url {
        result.items.push(ParsedItem::new(
            entry.raw.clone(),
            InputType::Url,
            url.clone(),
        ));
    }

    if let Some(ref_val) = reference_value {
        result
            .items
            .push(ParsedItem::reference(entry.raw.clone(), ref_val));
    }

    result.entries.push(entry);
}

fn normalize_doi(value: &str) -> Option<String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return None;
    }
    // Delegate to the existing DOI extractor for consistent normalization.
    extract_dois(trimmed)
        .into_iter()
        .find_map(|r| r.ok().map(|item| item.value))
}

fn normalize_year(value: &str) -> Option<u16> {
    YEAR_PATTERN
        .find(value)
        .and_then(|m| m.as_str().parse::<u16>().ok())
}

fn build_reference_value(entry: &RisEntry) -> Option<String> {
    let mut parts = Vec::new();

    if let Some(ref authors) = entry.authors {
        parts.push(authors.clone());
    }
    if let Some(year) = entry.year {
        parts.push(format!("({year})"));
    }
    if let Some(ref title) = entry.title {
        if title.ends_with('.') {
            parts.push(title.clone());
        } else {
            parts.push(format!("{title}."));
        }
    }

    if parts.is_empty() {
        return None;
    }

    Some(parts.join(" "))
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use crate::parser::InputType;

    // ==================== AC: Valid .ris file ====================

    #[test]
    fn test_parse_ris_content_valid_journal_entry() {
        let input = "TY  - JOUR\nAU  - Smith, J.\nTI  - A Great Paper\nPY  - 2024\nDO  - 10.1234/example\nER  - \n";
        let result = parse_ris_content(input);

        assert_eq!(result.entries.len(), 1, "should parse one entry");
        assert!(result.skipped.is_empty(), "should have no skipped entries");

        let entry = &result.entries[0];
        assert_eq!(entry.entry_type, "JOUR");
        assert_eq!(entry.doi.as_deref(), Some("10.1234/example"));
        assert_eq!(entry.title.as_deref(), Some("A Great Paper"));
        assert_eq!(entry.authors.as_deref(), Some("Smith, J."));
        assert_eq!(entry.year, Some(2024));
    }

    #[test]
    fn test_parse_ris_content_multiple_entries() {
        let input = concat!(
            "TY  - JOUR\nTI  - First\nDO  - 10.1111/a\nER  - \n",
            "TY  - BOOK\nTI  - Second\nDO  - 10.2222/b\nER  - \n",
        );
        let result = parse_ris_content(input);

        assert_eq!(result.entries.len(), 2);
        assert_eq!(result.total_found, 2);
    }

    // ==================== AC: Entry with only DOI ====================

    #[test]
    fn test_parse_ris_content_entry_only_doi_emits_doi_item() {
        let input = "TY  - JOUR\nDO  - 10.9999/only-doi\nER  - \n";
        let result = parse_ris_content(input);

        assert_eq!(result.entries.len(), 1);
        let doi_items: Vec<_> = result
            .items
            .iter()
            .filter(|i| i.input_type == InputType::Doi)
            .collect();
        assert_eq!(doi_items.len(), 1);
        assert_eq!(doi_items[0].value, "10.9999/only-doi");
    }

    // ==================== AC: Entry with only URL ====================

    #[test]
    fn test_parse_ris_content_entry_only_url_emits_url_item() {
        let input = "TY  - JOUR\nTI  - URL Only\nUR  - https://example.com/paper.pdf\nER  - \n";
        let result = parse_ris_content(input);

        assert_eq!(result.entries.len(), 1);
        let url_items: Vec<_> = result
            .items
            .iter()
            .filter(|i| i.input_type == InputType::Url)
            .collect();
        assert_eq!(url_items.len(), 1);
        assert_eq!(url_items[0].value, "https://example.com/paper.pdf");
    }

    // ==================== AC: Entry with both DOI and URL (prefer DOI) ====================

    #[test]
    fn test_parse_ris_content_doi_preferred_over_url() {
        let input = "TY  - JOUR\nDO  - 10.1234/prefer\nUR  - https://example.com/paper\nER  - \n";
        let result = parse_ris_content(input);

        assert_eq!(result.entries.len(), 1);

        // Must have DOI item
        let doi_items: Vec<_> = result
            .items
            .iter()
            .filter(|i| i.input_type == InputType::Doi)
            .collect();
        assert_eq!(doi_items.len(), 1, "should emit DOI item");

        // Must NOT have URL item (DOI wins)
        let url_items: Vec<_> = result
            .items
            .iter()
            .filter(|i| i.input_type == InputType::Url)
            .collect();
        assert_eq!(
            url_items.len(),
            0,
            "should suppress URL item when DOI present"
        );
    }

    // ==================== AC: Malformed entries do not panic ====================

    #[test]
    fn test_parse_ris_content_malformed_missing_er_does_not_panic() {
        let input = "TY  - JOUR\nTI  - Unterminated\nDO  - 10.1234/missing-er\n";
        let result = parse_ris_content(input);

        // Should not panic; the entry is skipped with an error message.
        assert!(
            result.entries.is_empty(),
            "unterminated entry should be skipped"
        );
        assert_eq!(result.skipped.len(), 1, "should have one skip message");
        assert!(
            result.skipped[0].contains("What:"),
            "skip message should follow What/Why/Fix"
        );
    }

    #[test]
    fn test_parse_ris_content_malformed_missing_ty_does_not_panic() {
        // A record fragment with no TY tag is silently dropped during segmentation.
        let input = "AU  - Orphan, A.\nTI  - No Type Tag\nER  - \n";
        let result = parse_ris_content(input);

        // Without TY the segment is never started, so nothing is found.
        assert_eq!(result.total_found, 0);
        assert!(result.entries.is_empty());
    }

    #[test]
    fn test_parse_ris_content_empty_input_returns_empty_result() {
        let result = parse_ris_content("");
        assert!(result.entries.is_empty());
        assert_eq!(result.total_found, 0);
    }

    #[test]
    fn test_parse_ris_content_whitespace_only_returns_empty() {
        let result = parse_ris_content("   \n\t\n  ");
        assert!(result.entries.is_empty());
    }

    // ==================== AC: Multiple authors ====================

    #[test]
    fn test_parse_ris_content_multiple_au_tags_joined() {
        let input = "TY  - JOUR\nAU  - Smith, J.\nAU  - Doe, R.\nAU  - Lee, M.\nTI  - Multi-author\nDO  - 10.1234/multi\nER  - \n";
        let result = parse_ris_content(input);

        assert_eq!(result.entries.len(), 1);
        assert_eq!(
            result.entries[0].authors.as_deref(),
            Some("Smith, J., Doe, R., Lee, M.")
        );
    }

    // ==================== AC: Reference item emitted from metadata ====================

    #[test]
    fn test_parse_ris_content_reference_item_emitted_when_metadata_available() {
        let input =
            "TY  - JOUR\nAU  - Jones, K.\nPY  - 2023\nTI  - The Title\nDO  - 10.5678/ref\nER  - \n";
        let result = parse_ris_content(input);

        let ref_items: Vec<_> = result
            .items
            .iter()
            .filter(|i| i.input_type == InputType::Reference)
            .collect();
        assert_eq!(ref_items.len(), 1, "should emit one reference item");
        assert!(
            ref_items[0].value.contains("Jones, K."),
            "reference should include author"
        );
        assert!(
            ref_items[0].value.contains("(2023)"),
            "reference should include year"
        );
        assert!(
            ref_items[0].value.contains("The Title"),
            "reference should include title"
        );
    }

    #[test]
    fn test_parse_ris_content_no_reference_item_when_no_metadata() {
        // Entry with only a DOI and no title/author/year produces no reference item.
        let input = "TY  - JOUR\nDO  - 10.1234/minimal\nER  - \n";
        let result = parse_ris_content(input);

        let ref_items: Vec<_> = result
            .items
            .iter()
            .filter(|i| i.input_type == InputType::Reference)
            .collect();
        assert_eq!(
            ref_items.len(),
            0,
            "no reference item when no metadata fields"
        );
    }

    // ==================== AC: DOI URL form normalization ====================

    #[test]
    fn test_parse_ris_content_doi_url_form_normalized() {
        let input = "TY  - JOUR\nDO  - https://doi.org/10.1234/doi-url\nTI  - DOI as URL\nER  - \n";
        let result = parse_ris_content(input);

        assert_eq!(result.entries.len(), 1);
        assert_eq!(
            result.entries[0].doi.as_deref(),
            Some("10.1234/doi-url"),
            "DOI should be normalized to bare form"
        );
    }

    // ==================== AC: Year normalization ====================

    #[test]
    fn test_parse_ris_content_year_normalized_from_full_date() {
        // PY may contain a full date like "2024/01/15" — only the year is extracted.
        let input = "TY  - JOUR\nPY  - 2024/01/15\nDO  - 10.1234/year-test\nER  - \n";
        let result = parse_ris_content(input);

        assert_eq!(result.entries.len(), 1);
        assert_eq!(result.entries[0].year, Some(2024));
    }

    // ==================== AC: Mixed valid and malformed entries ====================

    #[test]
    fn test_parse_ris_content_valid_entry_after_malformed_parsed_successfully() {
        let input = concat!(
            // Malformed: missing ER
            "TY  - JOUR\nTI  - Bad Entry\nDO  - 10.1234/bad\n",
            // Valid entry follows
            "TY  - JOUR\nTI  - Good Entry\nDO  - 10.5678/good\nER  - \n",
        );
        let result = parse_ris_content(input);

        assert_eq!(result.entries.len(), 1, "should parse the valid entry");
        assert_eq!(result.entries[0].doi.as_deref(), Some("10.5678/good"));
        assert_eq!(result.skipped.len(), 1, "should report the malformed entry");
    }
}
