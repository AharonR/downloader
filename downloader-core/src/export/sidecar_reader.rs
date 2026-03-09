//! Reads and deserializes Schema.org JSON-LD sidecar files from a corpus directory.

use std::path::{Path, PathBuf};

use serde::Deserialize;
use tracing::{debug, instrument, warn};

use super::error::ExportError;

/// A Schema.org `Person` (or `Organization`) author entry.
#[derive(Debug, Deserialize)]
pub struct SidecarAuthor {
    /// Display name of the author.
    pub name: String,
}

/// A Schema.org `PropertyValue` identifier (e.g. a DOI).
#[derive(Debug, Deserialize)]
pub struct SidecarIdentifier {
    /// The identifier string value (e.g. `"10.1234/example"`).
    pub value: String,
}

/// Parsed representation of a Schema.org `ScholarlyArticle` sidecar JSON-LD file.
///
/// All fields are optional because any of them may be absent in a partially-enriched sidecar.
/// The `@type` field is validated during deserialization via [`SidecarRecord`].
#[derive(Debug)]
pub struct SidecarEntry {
    /// File system path to the sidecar `.json` file.
    pub path: PathBuf,
    /// Article title (`name` field in JSON-LD).
    pub title: Option<String>,
    /// List of authors in declaration order.
    pub authors: Vec<SidecarAuthor>,
    /// ISO 8601 date or bare year string (`datePublished`).
    pub date_published: Option<String>,
    /// DOI identifier if present (`identifier.value`).
    pub doi: Option<String>,
    /// Canonical URL of the document.
    pub url: Option<String>,
}

// ── private deserialization glue ─────────────────────────────────────────────

/// Raw deserialization target for the sidecar JSON. Validated via [`SidecarRecord::into_entry`].
#[derive(Debug, Deserialize)]
struct SidecarRecord {
    #[serde(rename = "@type")]
    type_: String,
    name: Option<String>,
    #[serde(default)]
    author: Vec<SidecarAuthor>,
    #[serde(rename = "datePublished")]
    date_published: Option<String>,
    identifier: Option<SidecarIdentifier>,
    url: Option<String>,
}

impl SidecarRecord {
    /// Returns `Some(SidecarEntry)` if the record is a `ScholarlyArticle`, otherwise `None`.
    fn into_entry(self, path: PathBuf) -> Option<SidecarEntry> {
        if self.type_ != "ScholarlyArticle" {
            return None;
        }
        Some(SidecarEntry {
            path,
            title: self.name,
            authors: self.author,
            date_published: self.date_published,
            doi: self.identifier.map(|id| id.value),
            url: self.url,
        })
    }
}

// ── public API ────────────────────────────────────────────────────────────────

/// Scans `corpus_dir` for `.json` sidecar files and returns all valid `ScholarlyArticle` entries.
///
/// Files that are not valid JSON or whose `@type` is not `ScholarlyArticle` are silently skipped
/// with a `debug!` log. I/O errors at the directory-scan level are returned as [`ExportError`].
///
/// # Errors
///
/// Returns [`ExportError::CorpusNotFound`] if `corpus_dir` does not exist or is not a directory.
/// Returns [`ExportError::Io`] if the directory cannot be read.
#[instrument(fields(corpus_dir = %corpus_dir.display()))]
pub fn scan_corpus(corpus_dir: &Path) -> Result<Vec<SidecarEntry>, ExportError> {
    if !corpus_dir.is_dir() {
        return Err(ExportError::CorpusNotFound {
            path: corpus_dir.display().to_string(),
        });
    }

    let mut entries = Vec::new();
    let read_dir = std::fs::read_dir(corpus_dir)?;

    for dir_entry in read_dir {
        let dir_entry = dir_entry?;
        let path = dir_entry.path();

        if path.extension().and_then(|e| e.to_str()) != Some("json") {
            continue;
        }

        match try_parse_sidecar(&path) {
            Some(entry) => {
                debug!(path = %path.display(), "Loaded sidecar entry");
                entries.push(entry);
            }
            None => {
                debug!(path = %path.display(), "Skipping non-sidecar or invalid JSON file");
            }
        }
    }

    // Sort by path for deterministic output order across platforms.
    entries.sort_by(|a, b| a.path.cmp(&b.path));

    debug!(count = entries.len(), "Sidecar scan complete");
    Ok(entries)
}

/// Attempts to parse a single `.json` file as a `ScholarlyArticle` sidecar.
///
/// Returns `None` (with a `debug!` log) on any parse or type mismatch error.
fn try_parse_sidecar(path: &Path) -> Option<SidecarEntry> {
    let content = match std::fs::read_to_string(path) {
        Ok(s) => s,
        Err(err) => {
            warn!(path = %path.display(), error = %err, "Could not read sidecar file");
            return None;
        }
    };

    let record: SidecarRecord = match serde_json::from_str(&content) {
        Ok(r) => r,
        Err(err) => {
            debug!(path = %path.display(), error = %err, "JSON parse failed; skipping");
            return None;
        }
    };

    record.into_entry(path.to_path_buf())
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use std::fs;

    fn write_file(dir: &Path, name: &str, content: impl AsRef<[u8]>) -> PathBuf {
        let p = dir.join(name);
        fs::write(&p, content.as_ref()).unwrap();
        p
    }

    fn scholarly_article_json(
        title: Option<&str>,
        authors: &[&str],
        year: Option<&str>,
        doi: Option<&str>,
        url: Option<&str>,
    ) -> String {
        let authors_json: Vec<String> = authors
            .iter()
            .map(|a| format!(r#"{{"@type":"Person","name":"{a}"}}"#))
            .collect();
        let authors_str = authors_json.join(",");

        let title_field = title.map_or_else(String::new, |t| format!(r#","name":"{t}""#));
        let year_field = year.map_or_else(String::new, |y| format!(r#","datePublished":"{y}""#));
        let doi_field = doi.map_or_else(String::new, |d| {
            format!(r#","identifier":{{"@type":"PropertyValue","propertyID":"DOI","value":"{d}"}}"#)
        });
        let url_field = url.map_or_else(String::new, |u| format!(r#","url":"{u}""#));

        format!(
            r#"{{"@context":"https://schema.org","@type":"ScholarlyArticle","author":[{authors_str}]{title_field}{year_field}{doi_field}{url_field}}}"#
        )
    }

    #[test]
    fn test_scan_corpus_returns_error_for_missing_dir() {
        let result = scan_corpus(Path::new("/nonexistent/corpus/dir"));
        assert!(
            matches!(result, Err(ExportError::CorpusNotFound { .. })),
            "expected CorpusNotFound error"
        );
    }

    #[test]
    fn test_scan_corpus_empty_dir_returns_empty_vec() {
        let tmp = tempfile::TempDir::new().unwrap();
        let entries = scan_corpus(tmp.path()).unwrap();
        assert!(entries.is_empty());
    }

    #[test]
    fn test_scan_corpus_ignores_non_json_files() {
        let tmp = tempfile::TempDir::new().unwrap();
        write_file(tmp.path(), "paper.pdf", b"fake pdf");
        write_file(tmp.path(), "readme.txt", b"text");
        let entries = scan_corpus(tmp.path()).unwrap();
        assert!(entries.is_empty());
    }

    #[test]
    fn test_scan_corpus_skips_invalid_json() {
        let tmp = tempfile::TempDir::new().unwrap();
        write_file(tmp.path(), "broken.json", b"not json at all");
        let entries = scan_corpus(tmp.path()).unwrap();
        assert!(entries.is_empty());
    }

    #[test]
    fn test_scan_corpus_skips_non_scholarly_article_type() {
        let tmp = tempfile::TempDir::new().unwrap();
        write_file(
            tmp.path(),
            "other.json",
            br#"{"@context":"https://schema.org","@type":"Book","name":"Some Book"}"#,
        );
        let entries = scan_corpus(tmp.path()).unwrap();
        assert!(entries.is_empty());
    }

    #[test]
    fn test_scan_corpus_parses_full_sidecar() {
        let tmp = tempfile::TempDir::new().unwrap();
        let json = scholarly_article_json(
            Some("Attention Is All You Need"),
            &["Ashish Vaswani", "Noam Shazeer"],
            Some("2017"),
            Some("10.48550/arXiv.1706.03762"),
            Some("https://arxiv.org/abs/1706.03762"),
        );
        write_file(tmp.path(), "vaswani2017.json", json.as_bytes());

        let entries = scan_corpus(tmp.path()).unwrap();
        assert_eq!(entries.len(), 1);
        let e = &entries[0];
        assert_eq!(e.title.as_deref(), Some("Attention Is All You Need"));
        assert_eq!(e.authors.len(), 2);
        assert_eq!(e.authors[0].name, "Ashish Vaswani");
        assert_eq!(e.authors[1].name, "Noam Shazeer");
        assert_eq!(e.date_published.as_deref(), Some("2017"));
        assert_eq!(e.doi.as_deref(), Some("10.48550/arXiv.1706.03762"));
        assert_eq!(e.url.as_deref(), Some("https://arxiv.org/abs/1706.03762"));
    }

    #[test]
    fn test_scan_corpus_parses_minimal_sidecar() {
        let tmp = tempfile::TempDir::new().unwrap();
        let json = r#"{"@context":"https://schema.org","@type":"ScholarlyArticle","url":"https://example.com/paper.pdf"}"#;
        write_file(tmp.path(), "minimal.json", json.as_bytes());

        let entries = scan_corpus(tmp.path()).unwrap();
        assert_eq!(entries.len(), 1);
        let e = &entries[0];
        assert!(e.title.is_none());
        assert!(e.authors.is_empty());
        assert!(e.date_published.is_none());
        assert!(e.doi.is_none());
        assert_eq!(e.url.as_deref(), Some("https://example.com/paper.pdf"));
    }

    #[test]
    fn test_scan_corpus_returns_multiple_entries_sorted_by_path() {
        let tmp = tempfile::TempDir::new().unwrap();
        let json_a = scholarly_article_json(Some("Paper A"), &[], Some("2020"), None, None);
        let json_b = scholarly_article_json(Some("Paper B"), &[], Some("2021"), None, None);
        write_file(tmp.path(), "b_paper.json", json_b.as_bytes());
        write_file(tmp.path(), "a_paper.json", json_a.as_bytes());

        let entries = scan_corpus(tmp.path()).unwrap();
        assert_eq!(entries.len(), 2);
        // Sorted by path: a_paper.json before b_paper.json
        assert_eq!(entries[0].title.as_deref(), Some("Paper A"));
        assert_eq!(entries[1].title.as_deref(), Some("Paper B"));
    }
}
