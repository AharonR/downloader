//! BibTeX parsing helpers for supported entry types.

use std::sync::LazyLock;

use regex::Regex;

use super::doi::extract_dois;
use super::input::ParsedItem;

#[allow(clippy::expect_used)]
static YEAR_PATTERN: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\b(?:18|19|20)\d{2}\b").expect("bibtex year regex is valid"));
#[allow(clippy::expect_used)]
static AUTHOR_SPLIT_PATTERN: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?i)\s+and\s+").expect("bibtex author split regex is valid"));

const SUPPORTED_TYPES: [&str; 3] = ["article", "book", "inproceedings"];
const IGNORED_BLOCK_TYPES: [&str; 3] = ["comment", "preamble", "string"];

/// Parsed BibTeX entry model for Story 2.6 scope.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BibtexEntry {
    /// Entry type (`article`, `book`, `inproceedings`).
    pub entry_type: String,
    /// Citation key after `@type{`.
    pub key: String,
    /// Original text for this entry.
    pub raw: String,
    /// Parsed DOI (normalized to bare format) when available.
    pub doi: Option<String>,
    /// Parsed title when available.
    pub title: Option<String>,
    /// Parsed normalized authors when available.
    pub author: Option<String>,
    /// Parsed 4-digit year when available.
    pub year: Option<u16>,
}

/// Batch parse result for BibTeX input.
#[derive(Debug, Clone, Default)]
pub struct BibtexParseResult {
    /// Parsed structured entries.
    pub entries: Vec<BibtexEntry>,
    /// Items mapped into parser output structures.
    pub items: Vec<ParsedItem>,
    /// Actionable parse/skip messages.
    pub skipped: Vec<String>,
    /// Total candidate `@...{...}` segments discovered.
    pub total_found: usize,
    /// Raw BibTeX-like candidate segments consumed from input, including malformed/unsupported.
    pub consumed_segments: Vec<String>,
}

impl BibtexParseResult {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }
}

/// Parses BibTeX entries from input text.
#[tracing::instrument(skip(input), fields(input_len = input.len()))]
#[must_use]
pub fn parse_bibtex_entries(input: &str) -> BibtexParseResult {
    let mut result = BibtexParseResult::new();
    let segments = segment_entries(input);
    result.total_found = segments.len();

    for raw_entry in &segments {
        match parse_entry(raw_entry) {
            EntryOutcome::Parsed(entry) => {
                result
                    .items
                    .push(ParsedItem::bibtex(&entry.raw, entry.key.clone()));

                if let Some(doi) = &entry.doi {
                    result.items.push(ParsedItem::doi(&entry.raw, doi));
                }

                if let Some(reference_value) = build_reference_value(&entry) {
                    result
                        .items
                        .push(ParsedItem::reference(&entry.raw, reference_value));
                }

                result.entries.push(entry);
            }
            EntryOutcome::Ignore => {}
            EntryOutcome::Skip(message) => result.skipped.push(message),
        }
    }
    result.consumed_segments = segments;

    result
}

#[derive(Debug)]
enum EntryOutcome {
    Parsed(BibtexEntry),
    Ignore,
    Skip(String),
}

fn segment_entries(input: &str) -> Vec<String> {
    let chars: Vec<(usize, char)> = input.char_indices().collect();
    let mut entries = Vec::new();
    let mut i = 0usize;

    while i < chars.len() {
        if chars[i].1 != '@' {
            i += 1;
            continue;
        }

        let mut j = i + 1;
        while j < chars.len() && chars[j].1.is_ascii_alphabetic() {
            j += 1;
        }
        while j < chars.len() && chars[j].1.is_whitespace() {
            j += 1;
        }

        if j >= chars.len() || chars[j].1 != '{' {
            i += 1;
            continue;
        }

        let start = chars[i].0;
        let mut depth = 0usize;
        let mut in_quotes = false;
        let mut escape = false;
        let mut found_end = None;

        for (k, (_, ch)) in chars.iter().enumerate().skip(j) {
            if escape {
                escape = false;
                continue;
            }
            if *ch == '\\' {
                escape = true;
                continue;
            }
            if *ch == '"' {
                in_quotes = !in_quotes;
                continue;
            }
            if in_quotes {
                continue;
            }
            if *ch == '{' {
                depth += 1;
                continue;
            }
            if *ch == '}' {
                if depth == 0 {
                    break;
                }
                depth -= 1;
                if depth == 0 {
                    found_end = Some(k);
                    break;
                }
            }
        }

        if let Some(end_index) = found_end {
            let end_exclusive = if end_index + 1 < chars.len() {
                chars[end_index + 1].0
            } else {
                input.len()
            };
            entries.push(input[start..end_exclusive].trim().to_string());
            i = end_index + 1;
        } else {
            // Recovery path for malformed entries with unbalanced braces:
            // capture the malformed segment until the next likely entry start
            // (`@` at line start), then continue scanning.
            let mut recovery = i + 1;
            while recovery < chars.len() {
                if chars[recovery].1 == '@'
                    && (recovery == 0 || matches!(chars[recovery - 1].1, '\n' | '\r'))
                {
                    break;
                }
                recovery += 1;
            }

            if recovery < chars.len() {
                let end_exclusive = chars[recovery].0;
                entries.push(input[start..end_exclusive].trim().to_string());
                i = recovery;
                continue;
            }

            entries.push(input[start..].trim().to_string());
            break;
        }
    }

    entries
}

fn parse_entry(raw_entry: &str) -> EntryOutcome {
    let trimmed = raw_entry.trim();
    let Some(at_pos) = trimmed.find('@') else {
        return EntryOutcome::Skip(
            "What: malformed BibTeX entry. Why: missing '@type{...}' prefix. Fix: start entries with @article{key, ...}."
                .to_string(),
        );
    };
    let after_at = &trimmed[at_pos + 1..];
    let Some(brace_pos) = after_at.find('{') else {
        return EntryOutcome::Skip(format!(
            "What: malformed BibTeX entry `{}`. Why: missing opening '{{' after entry type. Fix: use `@type{{key, field = value}}`.",
            preview(trimmed)
        ));
    };

    let entry_type = after_at[..brace_pos].trim().to_ascii_lowercase();
    if IGNORED_BLOCK_TYPES.contains(&entry_type.as_str()) {
        return EntryOutcome::Ignore;
    }
    if !SUPPORTED_TYPES.contains(&entry_type.as_str()) {
        return EntryOutcome::Skip(format!(
            "What: unsupported BibTeX entry type `@{entry_type}`. Why: Story 2.6 supports only @article/@book/@inproceedings. Fix: export supported types or use DOI/reference input for this entry."
        ));
    }

    let body = &after_at[brace_pos + 1..];
    if !trimmed.ends_with('}') {
        return EntryOutcome::Skip(format!(
            "What: malformed BibTeX entry `{}`. Why: unbalanced braces (entry never closed). Fix: ensure each '{{' has a matching '}}'.",
            preview(trimmed)
        ));
    }
    let body = &body[..body.len().saturating_sub(1)];
    let Some((key_raw, fields_raw)) = body.split_once(',') else {
        return EntryOutcome::Skip(format!(
            "What: malformed BibTeX entry `{}`. Why: missing citation key or field list. Fix: use `@{}{{key, field = value}}`.",
            preview(trimmed),
            entry_type
        ));
    };

    let key = key_raw.trim();
    if key.is_empty() {
        return EntryOutcome::Skip(format!(
            "What: malformed BibTeX entry `{}`. Why: empty citation key. Fix: provide a non-empty key before the first comma.",
            preview(trimmed)
        ));
    }

    let fields = match parse_fields(fields_raw) {
        Ok(fields) => fields,
        Err(reason) => {
            return EntryOutcome::Skip(format!(
                "What: malformed BibTeX field assignment in `{}`. Why: {}. Fix: use `field = {{value}}` or `field = \"value\"` with commas between fields.",
                preview(trimmed),
                reason
            ));
        }
    };

    let doi = fields
        .get("doi")
        .and_then(|value| normalize_doi_field(value));
    let title = fields
        .get("title")
        .cloned()
        .filter(|value| !value.is_empty());
    let author = fields
        .get("author")
        .map(|value| normalize_authors(value))
        .filter(|value| !value.is_empty());
    let year = fields.get("year").and_then(|value| normalize_year(value));

    EntryOutcome::Parsed(BibtexEntry {
        entry_type,
        key: key.to_string(),
        raw: trimmed.to_string(),
        doi,
        title,
        author,
        year,
    })
}

fn parse_fields(input: &str) -> Result<std::collections::HashMap<String, String>, String> {
    let mut pairs = Vec::new();
    let mut current = String::new();
    let mut depth = 0usize;
    let mut in_quotes = false;
    let mut escape = false;

    for ch in input.chars() {
        if escape {
            current.push(ch);
            escape = false;
            continue;
        }
        if ch == '\\' {
            current.push(ch);
            escape = true;
            continue;
        }
        if ch == '"' {
            in_quotes = !in_quotes;
            current.push(ch);
            continue;
        }
        if !in_quotes {
            if ch == '{' {
                depth += 1;
            } else if ch == '}' {
                if depth == 0 {
                    return Err("closing brace without matching opening brace".to_string());
                }
                depth -= 1;
            } else if ch == ',' && depth == 0 {
                let segment = current.trim();
                if !segment.is_empty() {
                    pairs.push(segment.to_string());
                }
                current.clear();
                continue;
            }
        }
        current.push(ch);
    }

    if in_quotes {
        return Err("unterminated quoted value".to_string());
    }
    if depth != 0 {
        return Err("unbalanced braces in field values".to_string());
    }

    let tail = current.trim();
    if !tail.is_empty() {
        pairs.push(tail.to_string());
    }

    let mut fields = std::collections::HashMap::new();
    for pair in pairs {
        let Some((name, value_raw)) = pair.split_once('=') else {
            return Err(format!("missing '=' in field segment `{pair}`"));
        };
        let field_name = name.trim().to_ascii_lowercase();
        if field_name.is_empty() {
            return Err("empty field name".to_string());
        }
        let value = strip_bibtex_value(value_raw.trim())
            .ok_or_else(|| format!("invalid value in field `{field_name}`"))?;
        // First-value-wins per standard BibTeX convention.
        fields.entry(field_name).or_insert(value);
    }

    Ok(fields)
}

fn strip_bibtex_value(value: &str) -> Option<String> {
    let trimmed = value.trim().trim_end_matches(',').trim();
    if trimmed.is_empty() {
        return None;
    }

    if trimmed.starts_with('{') && trimmed.ends_with('}') && trimmed.len() >= 2 {
        return Some(trimmed[1..trimmed.len() - 1].trim().to_string());
    }
    if trimmed.starts_with('"') && trimmed.ends_with('"') && trimmed.len() >= 2 {
        let inner = &trimmed[1..trimmed.len() - 1];
        return Some(inner.replace("\\\"", "\"").trim().to_string());
    }

    Some(trimmed.to_string())
}

fn normalize_doi_field(value: &str) -> Option<String> {
    extract_dois(value)
        .into_iter()
        .find_map(|item| item.ok().map(|parsed| parsed.value))
}

fn normalize_authors(value: &str) -> String {
    AUTHOR_SPLIT_PATTERN
        .split(value)
        .map(str::trim)
        .filter(|segment| !segment.is_empty())
        .collect::<Vec<_>>()
        .join(", ")
}

fn normalize_year(value: &str) -> Option<u16> {
    YEAR_PATTERN
        .find(value)
        .and_then(|m| m.as_str().parse::<u16>().ok())
}

fn build_reference_value(entry: &BibtexEntry) -> Option<String> {
    let mut parts = Vec::new();
    if let Some(author) = &entry.author {
        parts.push(author.clone());
    }
    if let Some(year) = entry.year {
        parts.push(format!("({year})"));
    }
    if let Some(title) = &entry.title {
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

fn preview(input: &str) -> String {
    const MAX: usize = 80;
    if input.chars().count() <= MAX {
        return input.to_string();
    }
    let shortened: String = input.chars().take(MAX).collect();
    format!("{shortened}...")
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use std::time::Instant;

    use super::*;
    use crate::parser::InputType;

    #[test]
    fn test_parse_bibtex_supported_entry_types() {
        let input = r#"
@article{a1, title={A}, author={Smith, J.}, year={2024}}
@book{b1, title={B}, author={Jones, K.}, year={2023}}
@inproceedings{c1, title={C}, author={Lee, M.}, year={2022}}
"#;
        let result = parse_bibtex_entries(input);
        assert_eq!(result.entries.len(), 3);
        assert!(result.skipped.is_empty());
    }

    #[test]
    fn test_parse_bibtex_extracts_doi_title_author_year() {
        let input = r#"@article{k, title={Paper Title}, author={Smith, J. and Doe, R.}, year={2024}, doi={https://doi.org/10.1234/example}}"#;
        let result = parse_bibtex_entries(input);
        assert_eq!(result.entries.len(), 1);
        let entry = &result.entries[0];
        assert_eq!(entry.doi.as_deref(), Some("10.1234/example"));
        assert_eq!(entry.title.as_deref(), Some("Paper Title"));
        assert_eq!(entry.author.as_deref(), Some("Smith, J., Doe, R."));
        assert_eq!(entry.year, Some(2024));
    }

    #[test]
    fn test_parse_bibtex_quoted_and_braced_values_supported() {
        let input = r#"@article{k, title="Quoted", author={Smith, J. and Doe, R.}, year="2024", doi="10.1234/example",}"#;
        let result = parse_bibtex_entries(input);
        assert_eq!(result.entries.len(), 1);
        assert_eq!(result.entries[0].title.as_deref(), Some("Quoted"));
    }

    #[test]
    fn test_parse_bibtex_nested_braces_and_escaped_quotes() {
        let input = r#"@article{k, title={A {Nested} Title with \"quote\"}, author={Smith, J.}, year={2024}}"#;
        let result = parse_bibtex_entries(input);
        assert_eq!(result.entries.len(), 1);
        assert!(
            result.entries[0]
                .title
                .as_ref()
                .is_some_and(|value| value.contains("Nested"))
        );
    }

    #[test]
    fn test_parse_bibtex_multiline_fields() {
        let input = r#"@article{key1,
  title = {A very long
           multiline title},
  author = {Smith, J. and Doe, R.},
  year = {2024},
  doi = {10.1234/example}
}"#;
        let result = parse_bibtex_entries(input);
        assert_eq!(result.entries.len(), 1);
        assert_eq!(
            result
                .items
                .iter()
                .filter(|i| i.input_type == InputType::Doi)
                .count(),
            1
        );
    }

    #[test]
    fn test_parse_bibtex_ignores_comment_preamble_string() {
        let input = r#"
@comment{this is ignored}
@preamble{"\newcommand{\noop}{}"}
@string{foo = "bar"}
@article{k, title={A}, author={Smith, J.}, year={2024}}
"#;
        let result = parse_bibtex_entries(input);
        assert_eq!(result.entries.len(), 1);
        assert!(result.skipped.is_empty());
    }

    #[test]
    fn test_parse_bibtex_unsupported_type_is_skipped_with_message() {
        let input = r#"@misc{k, title={A}, year={2024}}"#;
        let result = parse_bibtex_entries(input);
        assert!(result.entries.is_empty());
        assert_eq!(result.skipped.len(), 1);
        assert!(result.skipped[0].contains("unsupported BibTeX entry type"));
    }

    #[test]
    fn test_parse_bibtex_malformed_unbalanced_entry() {
        let input = r#"@article{k, title={A}, year={2024}"#;
        let result = parse_bibtex_entries(input);
        assert!(result.entries.is_empty());
        assert_eq!(result.skipped.len(), 1);
        assert!(result.skipped[0].contains("unbalanced braces"));
        assert!(result.skipped[0].contains("What:"));
        assert!(result.skipped[0].contains("Why:"));
        assert!(result.skipped[0].contains("Fix:"));
    }

    #[test]
    fn test_parse_bibtex_mixed_valid_and_malformed_entries() {
        let input = r#"
@article{ok, title={Good}, author={Smith, J.}, year={2024}}
@article{bad, title {Missing equals}, year={2024}}
@book{ok2, title={Book Title}, author={Jones, K.}, year={2023}}
"#;
        let result = parse_bibtex_entries(input);
        assert_eq!(result.entries.len(), 2);
        assert!(!result.skipped.is_empty());
    }

    #[test]
    fn test_parse_bibtex_malformed_unbalanced_does_not_swallow_next_valid_entry() {
        let input = r#"
@article{bad, title={Broken}, year={2024}
@article{ok, title={Good}, author={Smith, J.}, year={2024}, doi={10.1234/good}}
"#;

        let result = parse_bibtex_entries(input);
        assert_eq!(result.entries.len(), 1);
        assert_eq!(result.entries[0].key, "ok");
        assert!(result.skipped.iter().any(|line| line.contains("malformed")));
    }

    #[test]
    fn test_parse_bibtex_author_normalization_handles_and_variants() {
        let input = r#"@article{k, title={A}, author={Smith, J. AND   Doe, R.
and Lee, M.}, year={2024}}"#;
        let result = parse_bibtex_entries(input);
        assert_eq!(result.entries.len(), 1);
        assert_eq!(
            result.entries[0].author.as_deref(),
            Some("Smith, J., Doe, R., Lee, M.")
        );
    }

    #[test]
    fn test_parse_bibtex_bare_field_values() {
        let input = r#"@article{k, title = Bare Title, author = {Smith, J.}, year = 2024}"#;
        let result = parse_bibtex_entries(input);
        assert_eq!(result.entries.len(), 1);
        assert_eq!(result.entries[0].title.as_deref(), Some("Bare Title"));
        assert_eq!(result.entries[0].year, Some(2024));
    }

    #[test]
    fn test_parse_bibtex_duplicate_field_first_value_wins() {
        let input = r#"@article{k, title={First Title}, title={Second Title}, author={Smith, J.}, year={2024}}"#;
        let result = parse_bibtex_entries(input);
        assert_eq!(result.entries.len(), 1);
        assert_eq!(
            result.entries[0].title.as_deref(),
            Some("First Title"),
            "First-value-wins per standard BibTeX convention"
        );
    }

    #[test]
    fn test_build_reference_value_no_double_period() {
        let input =
            r#"@article{k, title={Title ending with period.}, author={Smith, J.}, year={2024}}"#;
        let result = parse_bibtex_entries(input);
        assert_eq!(result.entries.len(), 1);
        let ref_items: Vec<_> = result
            .items
            .iter()
            .filter(|i| i.input_type == InputType::Reference)
            .collect();
        assert_eq!(ref_items.len(), 1);
        assert!(
            !ref_items[0].value.contains(".."),
            "Reference value should not contain double period, got: {}",
            ref_items[0].value
        );
    }

    #[test]
    fn test_parse_bibtex_large_batch_120_entries() {
        let mut input = String::new();
        for idx in 0..120 {
            input.push_str(&format!(
                "@article{{k{idx}, title={{Title {idx}}}, author={{Smith, J. and Doe, R.}}, year={{2024}}, doi={{10.1234/test-{idx}}}}}\n"
            ));
        }

        let start = Instant::now();
        let result = parse_bibtex_entries(&input);
        let elapsed = start.elapsed();

        assert_eq!(result.entries.len(), 120);
        assert_eq!(result.items.len(), 360); // BibTeX marker + DOI + mapped reference per entry
        eprintln!("Parsed 120 BibTeX entries in {:?}", elapsed);
    }
}
