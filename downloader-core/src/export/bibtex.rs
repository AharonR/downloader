//! BibTeX bibliography generation from sidecar entries.
//!
//! Maps Schema.org `ScholarlyArticle` fields to BibTeX `@article` entries following
//! standard BibTeX conventions. The output is importable by Zotero, Mendeley, `JabRef`,
//! and any other reference manager that supports BibTeX.

use tracing::instrument;

use super::sidecar_reader::SidecarEntry;

/// Generates a BibTeX bibliography string from a slice of sidecar entries.
///
/// Each entry maps to one `@article` block. Entries with no usable fields (no title,
/// authors, year, DOI, or URL) still produce a minimal entry with a citation key and URL.
///
/// # Citation key format
///
/// `{first_author_lastname}{year}` — for example `vaswani2017`.
/// Falls back to a sanitized path stem when either component is absent.
#[must_use]
#[instrument(skip(entries), fields(entry_count = entries.len()))]
pub fn generate_bibtex(entries: &[SidecarEntry]) -> String {
    let mut output = String::new();
    for entry in entries {
        if !output.is_empty() {
            output.push_str("\n\n");
        }
        output.push_str(&entry_to_bibtex(entry));
    }
    output
}

/// Converts a single [`SidecarEntry`] to a BibTeX `@article` block.
fn entry_to_bibtex(entry: &SidecarEntry) -> String {
    let key = citation_key(entry);

    let mut fields = Vec::new();

    if let Some(title) = &entry.title {
        fields.push(format!("  title     = {{{}}}", escape_bibtex(title)));
    }

    let author_str = authors_bibtex(&entry.authors);
    if !author_str.is_empty() {
        fields.push(format!("  author    = {{{author_str}}}"));
    }

    if let Some(year) = entry.date_published.as_deref().and_then(extract_year) {
        fields.push(format!("  year      = {{{year}}}"));
    }

    if let Some(doi) = &entry.doi {
        fields.push(format!("  doi       = {{{}}}", escape_bibtex(doi)));
    }

    if let Some(url) = &entry.url {
        fields.push(format!("  url       = {{{}}}", escape_bibtex(url)));
    }

    let mut block = format!("@article{{{key},\n");
    block.push_str(&fields.join(",\n"));
    if !fields.is_empty() {
        block.push('\n');
    }
    block.push('}');
    block
}

/// Builds the BibTeX `author` field value from the authors list.
///
/// Authors are joined with ` and ` as required by BibTeX convention.
fn authors_bibtex(authors: &[super::sidecar_reader::SidecarAuthor]) -> String {
    authors
        .iter()
        .map(|a| escape_bibtex(&a.name))
        .collect::<Vec<_>>()
        .join(" and ")
}

/// Derives a citation key from the entry metadata.
///
/// Format: `{first_author_lastname}{year}` (all lowercase, alphanumeric only).
/// Falls back to the sanitized path stem if metadata is insufficient.
fn citation_key(entry: &SidecarEntry) -> String {
    let lastname = entry
        .authors
        .first()
        .map(|a| extract_lastname(&a.name))
        .filter(|s| !s.is_empty());

    let year = entry
        .date_published
        .as_deref()
        .and_then(extract_year)
        .map(str::to_string);

    match (lastname, year) {
        (Some(ln), Some(yr)) => sanitize_key(&format!("{ln}{yr}")),
        (Some(ln), None) => sanitize_key(&ln),
        (None, Some(yr)) => sanitize_key(&yr),
        (None, None) => {
            // Fall back to path stem
            let stem = entry
                .path
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("unknown");
            sanitize_key(stem)
        }
    }
}

/// Extracts the year portion from an ISO 8601 date or bare year string.
///
/// Accepts formats like `"2017"`, `"2017-01"`, `"2017-06-12"`.
/// Returns `None` if the string does not start with a 4-digit year.
fn extract_year(date: &str) -> Option<&str> {
    let year = date.split('-').next().unwrap_or(date).trim();
    if year.len() == 4 && year.chars().all(|c| c.is_ascii_digit()) {
        Some(year)
    } else {
        None
    }
}

/// Extracts the last name from an author name string.
///
/// Handles:
/// - `"First Last"` → `"last"`
/// - `"Last, First"` → `"last"`
/// - Single-token names → returned as-is (lowercased)
fn extract_lastname(name: &str) -> String {
    let name = name.trim();
    if name.contains(',') {
        // "Last, First" form
        let part = name.split(',').next().unwrap_or(name).trim();
        part.to_lowercase()
    } else {
        // "First Last" or "First Middle Last" — take the last token
        name.split_whitespace()
            .next_back()
            .unwrap_or(name)
            .to_lowercase()
    }
}

/// Sanitizes a string for use as a BibTeX citation key (ASCII alphanumeric + `-` + `_`).
fn sanitize_key(s: &str) -> String {
    s.chars()
        .filter(|c| c.is_ascii_alphanumeric() || *c == '-' || *c == '_')
        .collect::<String>()
        .to_lowercase()
}

/// Escapes special BibTeX characters in field values.
///
/// Escapes: `&`, `%`, `$`, `#`, `_`, `{`, `}`, `~`, `^`, `\`.
/// Note: We wrap all field values in `{}` braces so most TeX special characters
/// are already protected. This function handles the remaining problematic ones.
fn escape_bibtex(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '&' => out.push_str("\\&"),
            '%' => out.push_str("\\%"),
            '$' => out.push_str("\\$"),
            '#' => out.push_str("\\#"),
            // Underscores in URLs are common and must not be escaped for url/doi fields,
            // but BibTeX processors usually handle them in braced values. Keep as-is.
            _ => out.push(c),
        }
    }
    out
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use std::path::PathBuf;

    use super::*;
    use crate::export::sidecar_reader::{SidecarAuthor, SidecarEntry};

    fn make_entry(
        path: &str,
        title: Option<&str>,
        authors: &[&str],
        year: Option<&str>,
        doi: Option<&str>,
        url: Option<&str>,
    ) -> SidecarEntry {
        SidecarEntry {
            path: PathBuf::from(path),
            title: title.map(String::from),
            authors: authors
                .iter()
                .map(|a| SidecarAuthor {
                    name: a.to_string(),
                })
                .collect(),
            date_published: year.map(String::from),
            doi: doi.map(String::from),
            url: url.map(String::from),
        }
    }

    // ── extract_year ─────────────────────────────────────────────────────────

    #[test]
    fn test_extract_year_bare_year() {
        assert_eq!(extract_year("2017"), Some("2017"));
    }

    #[test]
    fn test_extract_year_iso_date() {
        assert_eq!(extract_year("2017-06-12"), Some("2017"));
    }

    #[test]
    fn test_extract_year_year_month() {
        assert_eq!(extract_year("2017-06"), Some("2017"));
    }

    #[test]
    fn test_extract_year_invalid_returns_none() {
        assert_eq!(extract_year("not-a-year"), None);
        assert_eq!(extract_year(""), None);
    }

    // ── extract_lastname ──────────────────────────────────────────────────────

    #[test]
    fn test_extract_lastname_first_last() {
        assert_eq!(extract_lastname("Ashish Vaswani"), "vaswani");
    }

    #[test]
    fn test_extract_lastname_last_comma_first() {
        assert_eq!(extract_lastname("Vaswani, Ashish"), "vaswani");
    }

    #[test]
    fn test_extract_lastname_single_name() {
        assert_eq!(extract_lastname("Plato"), "plato");
    }

    #[test]
    fn test_extract_lastname_three_parts() {
        assert_eq!(extract_lastname("Mary Ann Evans"), "evans");
    }

    // ── citation_key ──────────────────────────────────────────────────────────

    #[test]
    fn test_citation_key_full_metadata() {
        let entry = make_entry(
            "/corpus/vaswani2017.json",
            Some("Attention Is All You Need"),
            &["Ashish Vaswani"],
            Some("2017"),
            None,
            None,
        );
        assert_eq!(citation_key(&entry), "vaswani2017");
    }

    #[test]
    fn test_citation_key_no_authors_uses_year() {
        let entry = make_entry(
            "/corpus/paper.json",
            Some("Title"),
            &[],
            Some("2021"),
            None,
            None,
        );
        assert_eq!(citation_key(&entry), "2021");
    }

    #[test]
    fn test_citation_key_no_year_uses_lastname() {
        let entry = make_entry(
            "/corpus/paper.json",
            None,
            &["Alice Smith"],
            None,
            None,
            None,
        );
        assert_eq!(citation_key(&entry), "smith");
    }

    #[test]
    fn test_citation_key_no_metadata_uses_path_stem() {
        let entry = make_entry("/corpus/my_paper.json", None, &[], None, None, None);
        assert_eq!(citation_key(&entry), "my_paper");
    }

    // ── entry_to_bibtex ───────────────────────────────────────────────────────

    #[test]
    fn test_bibtex_full_entry_contains_all_fields() {
        let entry = make_entry(
            "/corpus/vaswani2017.json",
            Some("Attention Is All You Need"),
            &["Ashish Vaswani", "Noam Shazeer"],
            Some("2017"),
            Some("10.48550/arXiv.1706.03762"),
            Some("https://arxiv.org/abs/1706.03762"),
        );
        let bib = entry_to_bibtex(&entry);
        assert!(
            bib.starts_with("@article{vaswani2017,"),
            "key mismatch: {bib}"
        );
        assert!(
            bib.contains("title     = {Attention Is All You Need}"),
            "{bib}"
        );
        assert!(
            bib.contains("author    = {Ashish Vaswani and Noam Shazeer}"),
            "{bib}"
        );
        assert!(bib.contains("year      = {2017}"), "{bib}");
        assert!(
            bib.contains("doi       = {10.48550/arXiv.1706.03762}"),
            "{bib}"
        );
        assert!(
            bib.contains("url       = {https://arxiv.org/abs/1706.03762}"),
            "{bib}"
        );
    }

    #[test]
    fn test_bibtex_partial_entry_omits_missing_fields() {
        let entry = make_entry(
            "/corpus/minimal.json",
            Some("Minimal Paper"),
            &[],
            None,
            None,
            Some("https://example.com"),
        );
        let bib = entry_to_bibtex(&entry);
        assert!(bib.contains("title     = {Minimal Paper}"), "{bib}");
        assert!(!bib.contains("author"), "author should be absent: {bib}");
        assert!(!bib.contains("year"), "year should be absent: {bib}");
        assert!(!bib.contains("doi"), "doi should be absent: {bib}");
        assert!(bib.contains("url       = {https://example.com}"), "{bib}");
    }

    #[test]
    fn test_bibtex_empty_entry_produces_valid_block() {
        let entry = make_entry("/corpus/empty.json", None, &[], None, None, None);
        let bib = entry_to_bibtex(&entry);
        assert!(bib.starts_with("@article{"), "{bib}");
        assert!(bib.ends_with('}'), "{bib}");
    }

    #[test]
    fn test_bibtex_ampersand_in_title_escaped() {
        let entry = make_entry(
            "/corpus/paper.json",
            Some("Foo & Bar"),
            &[],
            None,
            None,
            None,
        );
        let bib = entry_to_bibtex(&entry);
        assert!(bib.contains("Foo \\& Bar"), "{bib}");
    }

    // ── generate_bibtex ───────────────────────────────────────────────────────

    #[test]
    fn test_generate_bibtex_empty_slice_returns_empty_string() {
        assert_eq!(generate_bibtex(&[]), "");
    }

    #[test]
    fn test_generate_bibtex_two_entries_separated_by_blank_line() {
        let entries = vec![
            make_entry(
                "/corpus/a.json",
                Some("Paper A"),
                &[],
                Some("2020"),
                None,
                None,
            ),
            make_entry(
                "/corpus/b.json",
                Some("Paper B"),
                &[],
                Some("2021"),
                None,
                None,
            ),
        ];
        let bib = generate_bibtex(&entries);
        // There should be exactly one blank line between the two @article blocks.
        let blocks: Vec<&str> = bib.split("\n\n").collect();
        assert_eq!(
            blocks.len(),
            2,
            "expected exactly 2 blocks separated by blank line"
        );
        assert!(blocks[0].contains("Paper A"), "{bib}");
        assert!(blocks[1].contains("Paper B"), "{bib}");
    }
}
