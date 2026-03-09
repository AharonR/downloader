//! RIS bibliography generation from sidecar entries.
//!
//! Maps Schema.org `ScholarlyArticle` fields to RIS records following the
//! [RIS format specification](https://en.wikipedia.org/wiki/RIS_(file_format)).
//! The output is importable by Zotero, Mendeley, `EndNote`, and most other reference managers.

use tracing::instrument;

use super::sidecar_reader::SidecarEntry;

/// Generates a RIS bibliography string from a slice of sidecar entries.
///
/// Each entry maps to one RIS record terminated by `ER  - `. Records are separated
/// by a blank line for readability.
#[must_use]
#[instrument(skip(entries), fields(entry_count = entries.len()))]
pub fn generate_ris(entries: &[SidecarEntry]) -> String {
    let mut output = String::new();
    for entry in entries {
        if !output.is_empty() {
            output.push('\n');
        }
        output.push_str(&entry_to_ris(entry));
    }
    output
}

/// Converts a single [`SidecarEntry`] to a RIS record string.
fn entry_to_ris(entry: &SidecarEntry) -> String {
    let mut lines: Vec<String> = Vec::new();

    lines.push("TY  - JOUR".to_string());

    if let Some(title) = &entry.title {
        lines.push(format!("TI  - {title}"));
    }

    for author in &entry.authors {
        lines.push(format!("AU  - {}", author.name));
    }

    if let Some(year) = entry.date_published.as_deref().and_then(extract_year) {
        lines.push(format!("PY  - {year}"));
    }

    if let Some(doi) = &entry.doi {
        lines.push(format!("DO  - {doi}"));
    }

    if let Some(url) = &entry.url {
        lines.push(format!("UR  - {url}"));
    }

    lines.push("ER  - ".to_string());

    let mut record = lines.join("\n");
    record.push('\n');
    record
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
    fn test_extract_year_bare_year_ris() {
        assert_eq!(extract_year("2017"), Some("2017"));
    }

    #[test]
    fn test_extract_year_iso_date_ris() {
        assert_eq!(extract_year("2017-06-12"), Some("2017"));
    }

    #[test]
    fn test_extract_year_invalid_returns_none_ris() {
        assert_eq!(extract_year("invalid"), None);
    }

    // ── entry_to_ris ──────────────────────────────────────────────────────────

    #[test]
    fn test_ris_full_entry_contains_all_fields() {
        let entry = make_entry(
            "/corpus/vaswani2017.json",
            Some("Attention Is All You Need"),
            &["Ashish Vaswani", "Noam Shazeer"],
            Some("2017"),
            Some("10.48550/arXiv.1706.03762"),
            Some("https://arxiv.org/abs/1706.03762"),
        );
        let ris = entry_to_ris(&entry);
        assert!(ris.contains("TY  - JOUR\n"), "TY missing: {ris}");
        assert!(ris.contains("TI  - Attention Is All You Need\n"), "{ris}");
        assert!(ris.contains("AU  - Ashish Vaswani\n"), "{ris}");
        assert!(ris.contains("AU  - Noam Shazeer\n"), "{ris}");
        assert!(ris.contains("PY  - 2017\n"), "{ris}");
        assert!(ris.contains("DO  - 10.48550/arXiv.1706.03762\n"), "{ris}");
        assert!(
            ris.contains("UR  - https://arxiv.org/abs/1706.03762\n"),
            "{ris}"
        );
        assert!(ris.contains("ER  - \n"), "{ris}");
    }

    #[test]
    fn test_ris_partial_entry_omits_missing_fields() {
        let entry = make_entry(
            "/corpus/minimal.json",
            Some("Minimal Paper"),
            &[],
            None,
            None,
            Some("https://example.com"),
        );
        let ris = entry_to_ris(&entry);
        assert!(ris.contains("TI  - Minimal Paper"), "{ris}");
        assert!(!ris.contains("AU  -"), "AU should be absent: {ris}");
        assert!(!ris.contains("PY  -"), "PY should be absent: {ris}");
        assert!(!ris.contains("DO  -"), "DO should be absent: {ris}");
        assert!(ris.contains("UR  - https://example.com"), "{ris}");
    }

    #[test]
    fn test_ris_minimal_entry_has_ty_and_er() {
        let entry = make_entry("/corpus/empty.json", None, &[], None, None, None);
        let ris = entry_to_ris(&entry);
        assert!(ris.starts_with("TY  - JOUR"), "{ris}");
        assert!(ris.trim_end().ends_with("ER  -"), "{ris}");
    }

    #[test]
    fn test_ris_multiple_authors_each_on_own_line() {
        let entry = make_entry(
            "/corpus/paper.json",
            None,
            &["Author One", "Author Two", "Author Three"],
            None,
            None,
            None,
        );
        let ris = entry_to_ris(&entry);
        let au_count = ris.lines().filter(|l| l.starts_with("AU  -")).count();
        assert_eq!(au_count, 3, "expected 3 AU lines: {ris}");
    }

    // ── generate_ris ──────────────────────────────────────────────────────────

    #[test]
    fn test_generate_ris_empty_slice_returns_empty_string() {
        assert_eq!(generate_ris(&[]), "");
    }

    #[test]
    fn test_generate_ris_two_entries_separated_by_blank_line() {
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
        let ris = generate_ris(&entries);
        // Each record ends with "ER  - \n" and records are separated by "\n"
        assert!(ris.contains("Paper A"), "{ris}");
        assert!(ris.contains("Paper B"), "{ris}");
        // Two ER markers
        let er_count = ris.matches("ER  -").count();
        assert_eq!(er_count, 2, "expected 2 ER markers: {ris}");
    }
}
