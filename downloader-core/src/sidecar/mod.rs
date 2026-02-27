//! JSON-LD sidecar file generation for downloaded documents.
//!
//! Writes machine-readable metadata files (`.json`) alongside downloaded files
//! following the Schema.org/ScholarlyArticle vocabulary.
//!
//! # Module structure note
//!
//! This module is intentionally a single file (`mod.rs`-only), mirroring the
//! `src/topics/mod.rs` pattern. The feature scope is small enough to not warrant
//! sub-files, making this an approved exception to the architecture guideline that
//! `mod.rs` should contain only declarations.

use std::fs;
use std::io::{BufWriter, ErrorKind};
use std::path::{Path, PathBuf};

use serde::Serialize;
use thiserror::Error;
use tracing::{debug, instrument};

use crate::queue::QueueItem;

/// Errors produced by sidecar generation.
#[derive(Debug, Error)]
pub enum SidecarError {
    /// I/O error writing the sidecar file to disk.
    #[error("I/O error writing sidecar: {0}")]
    Io(#[from] std::io::Error),
    /// JSON serialization error (shouldn't occur for well-formed structs).
    #[error("JSON serialization error: {0}")]
    Serialize(#[from] serde_json::Error),
}

/// Configuration for sidecar generation behaviour.
///
/// Used by callers in `main.rs` to check the enabled flag before calling
/// `generate_sidecar()`. The function itself does not take a `SidecarConfig`
/// parameter — the enabled check is the caller's responsibility.
#[derive(Debug, Clone)]
pub struct SidecarConfig {
    /// Whether sidecar generation is active for this run.
    pub enabled: bool,
}

/// Schema.org/ScholarlyArticle JSON-LD document root.
#[derive(Debug, Serialize)]
struct ScholarlyArticle {
    #[serde(rename = "@context")]
    context: &'static str,
    #[serde(rename = "@type")]
    type_: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    author: Option<Vec<Author>>,
    #[serde(rename = "datePublished", skip_serializing_if = "Option::is_none")]
    date_published: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    identifier: Option<DoiIdentifier>,
    #[serde(skip_serializing_if = "Option::is_none")]
    url: Option<String>,
}

/// A single author entry in the JSON-LD document.
#[derive(Debug, Serialize)]
struct Author {
    #[serde(rename = "@type")]
    type_: &'static str,
    name: String,
}

/// DOI expressed as a Schema.org `PropertyValue`.
#[derive(Debug, Serialize)]
struct DoiIdentifier {
    #[serde(rename = "@type")]
    type_: &'static str,
    #[serde(rename = "propertyID")]
    property_id: &'static str,
    value: String,
}

/// Generates a JSON-LD sidecar file alongside the downloaded file for `item`.
///
/// Returns `None` (with a `debug!` log) if:
/// - `item.saved_path` is `None` (download location unknown)
/// - the sidecar file already exists on disk (idempotent by design)
///
/// Returns `Some(sidecar_path)` on success.
///
/// Callers MUST check `SidecarConfig::enabled` before calling this function.
///
/// # Errors
///
/// Returns [`SidecarError`] on I/O or serialization failure.
#[instrument(fields(item_id = item.id, saved_path = ?item.saved_path))]
pub fn generate_sidecar(item: &QueueItem) -> Result<Option<PathBuf>, SidecarError> {
    let Some(ref saved_path_str) = item.saved_path else {
        debug!("No saved_path, skipping sidecar generation");
        return Ok(None);
    };

    let saved_path = Path::new(saved_path_str);
    if !saved_path.exists() {
        debug!(
            path = %saved_path.display(),
            "Downloaded file missing, skipping sidecar generation"
        );
        return Ok(None);
    }
    let sidecar_path = derive_sidecar_path(saved_path);

    let article = build_scholarly_article(item);
    let file = match fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&sidecar_path)
    {
        Ok(file) => file,
        Err(err) if err.kind() == ErrorKind::AlreadyExists => {
            debug!(
                path = %sidecar_path.display(),
                "Sidecar already exists, skipping"
            );
            return Ok(None);
        }
        Err(err) => return Err(err.into()),
    };
    let write_result = {
        let writer = BufWriter::new(file);
        serde_json::to_writer_pretty(writer, &article)
    };
    if let Err(err) = write_result {
        // Best-effort cleanup so a partially written file does not block retries.
        let _ = fs::remove_file(&sidecar_path);
        return Err(err.into());
    }

    debug!(path = %sidecar_path.display(), "Sidecar created");
    Ok(Some(sidecar_path))
}

/// Derives the sidecar `.json` path from a downloaded file path.
///
/// Examples:
/// - `paper.pdf` → `paper.json`
/// - `article.html` → `article.json`
/// - `no_extension` → `no_extension.json`
fn derive_sidecar_path(downloaded_path: &Path) -> PathBuf {
    let mut p = downloaded_path.to_path_buf();
    p.set_extension("json");
    p
}

/// Builds a `ScholarlyArticle` from `QueueItem` metadata.
fn build_scholarly_article(item: &QueueItem) -> ScholarlyArticle {
    let author = item.meta_authors.as_deref().and_then(|s| {
        let authors = parse_authors(s);
        if authors.is_empty() {
            None
        } else {
            Some(authors)
        }
    });

    let identifier = item
        .meta_doi
        .as_deref()
        .filter(|doi| !doi.is_empty())
        .map(|doi| DoiIdentifier {
            type_: "PropertyValue",
            property_id: "DOI",
            value: doi.to_string(),
        });

    ScholarlyArticle {
        context: "https://schema.org",
        type_: "ScholarlyArticle",
        name: item.meta_title.clone().filter(|s| !s.is_empty()),
        author,
        date_published: item.meta_year.clone().filter(|s| !s.is_empty()),
        identifier,
        url: Some(item.url.clone()),
    }
}

/// Parses a metadata author string into individual `Author` entries.
///
/// Strategy (per audit recommendation QA-2):
/// 1. If the string contains `';'`, split by `';'` first.
/// 2. Otherwise, split by `','` only when each token looks like a full name.
/// 3. Fallback: keep the entire input as a single author to avoid mis-splitting
///    family/given name forms like `"Smith, John"`.
///
/// Each token is trimmed of whitespace. Empty tokens are discarded.
fn parse_authors(authors_str: &str) -> Vec<Author> {
    let normalized = authors_str.trim();
    if normalized.is_empty() {
        return Vec::new();
    }

    let tokens: Vec<&str> = if normalized.contains(';') {
        normalized.split(';').collect()
    } else {
        let comma_tokens: Vec<&str> = normalized
            .split(',')
            .map(str::trim)
            .filter(|t| !t.is_empty())
            .collect();
        if comma_tokens.len() > 1 && comma_tokens.iter().all(|t| looks_like_full_name(t)) {
            comma_tokens
        } else {
            vec![normalized]
        }
    };

    tokens
        .into_iter()
        .map(str::trim)
        .filter(|t| !t.is_empty())
        .map(|name| Author {
            type_: "Person",
            name: name.to_string(),
        })
        .collect()
}

fn looks_like_full_name(token: &str) -> bool {
    token.split_whitespace().count() >= 2
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use std::sync::{Arc, Mutex};

    use tracing::field::{Field, Visit};
    use tracing::{Event, Subscriber};
    use tracing_subscriber::layer::{Context, Layer};
    use tracing_subscriber::prelude::*;
    use tracing_subscriber::registry::LookupSpan;

    #[derive(Debug, Default)]
    struct CapturedEvent {
        fields: HashMap<String, String>,
    }

    #[derive(Default)]
    struct EventFieldVisitor {
        fields: HashMap<String, String>,
    }

    impl EventFieldVisitor {
        fn into_event(self) -> CapturedEvent {
            CapturedEvent {
                fields: self.fields,
            }
        }
    }

    impl Visit for EventFieldVisitor {
        fn record_str(&mut self, field: &Field, value: &str) {
            self.fields
                .insert(field.name().to_string(), value.to_string());
        }

        fn record_debug(&mut self, field: &Field, value: &dyn std::fmt::Debug) {
            self.fields
                .insert(field.name().to_string(), format!("{value:?}"));
        }
    }

    #[derive(Clone)]
    struct EventCaptureLayer {
        events: Arc<Mutex<Vec<CapturedEvent>>>,
    }

    impl<S> Layer<S> for EventCaptureLayer
    where
        S: Subscriber + for<'lookup> LookupSpan<'lookup>,
    {
        fn on_event(&self, event: &Event<'_>, _ctx: Context<'_, S>) {
            let mut visitor = EventFieldVisitor::default();
            event.record(&mut visitor);
            self.events.lock().unwrap().push(visitor.into_event());
        }
    }

    // ───── helpers ─────────────────────────────────────────────────────────────

    fn make_item(
        saved_path: Option<&str>,
        title: Option<&str>,
        authors: Option<&str>,
        year: Option<&str>,
        doi: Option<&str>,
        url: &str,
    ) -> QueueItem {
        QueueItem {
            id: 1,
            url: url.to_string(),
            source_type: "direct_url".to_string(),
            original_input: None,
            status_str: "completed".to_string(),
            priority: 0,
            retry_count: 0,
            last_error: None,
            suggested_filename: None,
            meta_title: title.map(String::from),
            meta_authors: authors.map(String::from),
            meta_year: year.map(String::from),
            meta_doi: doi.map(String::from),
            topics: None,
            parse_confidence: None,
            parse_confidence_factors: None,
            saved_path: saved_path.map(String::from),
            bytes_downloaded: 0,
            content_length: None,
            created_at: "2026-01-01".to_string(),
            updated_at: "2026-01-01".to_string(),
        }
    }

    // ───── derive_sidecar_path ──────────────────────────────────────────────

    #[test]
    fn test_sidecar_path_replaces_pdf_extension() {
        let path = Path::new("/tmp/paper.pdf");
        assert_eq!(derive_sidecar_path(path), PathBuf::from("/tmp/paper.json"));
    }

    #[test]
    fn test_sidecar_path_replaces_html_extension() {
        let path = Path::new("/tmp/article.html");
        assert_eq!(
            derive_sidecar_path(path),
            PathBuf::from("/tmp/article.json")
        );
    }

    #[test]
    fn test_sidecar_path_no_extension_appends_json() {
        let path = Path::new("/tmp/no_extension");
        assert_eq!(
            derive_sidecar_path(path),
            PathBuf::from("/tmp/no_extension.json")
        );
    }

    // ───── parse_authors ────────────────────────────────────────────────────

    #[test]
    fn test_parse_authors_comma_separated_returns_vec() {
        let authors = parse_authors("Alice Smith, Bob Doe");
        assert_eq!(authors.len(), 2);
        assert_eq!(authors[0].name, "Alice Smith");
        assert_eq!(authors[1].name, "Bob Doe");
    }

    #[test]
    fn test_parse_authors_semicolon_separated_returns_vec() {
        let authors = parse_authors("Smith, J.; Doe, J.");
        assert_eq!(authors.len(), 2);
        assert_eq!(authors[0].name, "Smith, J.");
        assert_eq!(authors[1].name, "Doe, J.");
    }

    #[test]
    fn test_parse_authors_single_author_no_separator() {
        let authors = parse_authors("Alice Smith");
        assert_eq!(authors.len(), 1);
        assert_eq!(authors[0].name, "Alice Smith");
    }

    #[test]
    fn test_parse_authors_comma_name_format_kept_single_author() {
        let authors = parse_authors("Smith, John");
        assert_eq!(authors.len(), 1);
        assert_eq!(authors[0].name, "Smith, John");
    }

    #[test]
    fn test_parse_authors_ambiguous_comma_initials_kept_single_author() {
        let authors = parse_authors("Vaswani, A., Shazeer, N.");
        assert_eq!(authors.len(), 1);
        assert_eq!(authors[0].name, "Vaswani, A., Shazeer, N.");
    }

    #[test]
    fn test_parse_authors_empty_string_returns_empty() {
        let authors = parse_authors("");
        assert!(authors.is_empty());
    }

    #[test]
    fn test_parse_authors_trims_whitespace() {
        let authors = parse_authors("  Alice Smith  ,  Bob Doe  ");
        assert_eq!(authors[0].name, "Alice Smith");
        assert_eq!(authors[1].name, "Bob Doe");
    }

    #[test]
    fn test_parse_authors_type_is_person() {
        let authors = parse_authors("Alice Smith");
        assert_eq!(authors[0].type_, "Person");
    }

    // ───── build_scholarly_article ──────────────────────────────────────────

    #[test]
    fn test_scholarly_article_full_metadata_serializes_correctly() {
        let item = make_item(
            Some("/tmp/paper.pdf"),
            Some("Attention Is All You Need"),
            Some("Ashish Vaswani, Noam Shazeer"),
            Some("2017"),
            Some("10.48550/arXiv.1706.03762"),
            "https://arxiv.org/pdf/1706.03762",
        );
        let article = build_scholarly_article(&item);
        let json = serde_json::to_value(&article).unwrap();

        assert_eq!(json["@context"], "https://schema.org");
        assert_eq!(json["@type"], "ScholarlyArticle");
        assert_eq!(json["name"], "Attention Is All You Need");
        assert_eq!(json["datePublished"], "2017");
        assert_eq!(json["identifier"]["@type"], "PropertyValue");
        assert_eq!(json["identifier"]["propertyID"], "DOI");
        assert_eq!(json["identifier"]["value"], "10.48550/arXiv.1706.03762");
        assert_eq!(json["url"], "https://arxiv.org/pdf/1706.03762");

        let authors = json["author"].as_array().unwrap();
        assert_eq!(authors.len(), 2);
        assert_eq!(authors[0]["name"], "Ashish Vaswani");
        assert_eq!(authors[1]["name"], "Noam Shazeer");
    }

    #[test]
    fn test_scholarly_article_missing_doi_omits_identifier_field() {
        let item = make_item(
            Some("/tmp/paper.pdf"),
            Some("Test Paper"),
            None,
            None,
            None,
            "https://example.com/paper.pdf",
        );
        let article = build_scholarly_article(&item);
        let json = serde_json::to_value(&article).unwrap();

        assert!(
            json.get("identifier").is_none(),
            "identifier should be absent when DOI is None"
        );
        assert!(
            json.get("author").is_none(),
            "author should be absent when authors is None"
        );
        assert!(
            json.get("datePublished").is_none(),
            "datePublished should be absent when year is None"
        );
    }

    #[test]
    fn test_scholarly_article_url_always_present() {
        let item = make_item(
            Some("/tmp/paper.pdf"),
            None,
            None,
            None,
            None,
            "https://example.com/paper.pdf",
        );
        let article = build_scholarly_article(&item);
        let json = serde_json::to_value(&article).unwrap();
        assert_eq!(json["url"], "https://example.com/paper.pdf");
    }

    // ───── generate_sidecar ─────────────────────────────────────────────────

    #[test]
    fn test_generate_sidecar_no_saved_path_returns_none() {
        let item = make_item(
            None,
            Some("Test Paper"),
            None,
            None,
            None,
            "https://example.com/paper.pdf",
        );
        let result = generate_sidecar(&item).unwrap();
        assert!(
            result.is_none(),
            "should return None when saved_path is absent"
        );
    }

    #[test]
    fn test_generate_sidecar_missing_download_file_returns_none() {
        let tmp = tempfile::TempDir::new().unwrap();
        let missing_path = tmp.path().join("missing.pdf");
        let item = make_item(
            Some(missing_path.to_str().unwrap()),
            Some("Test Paper"),
            None,
            None,
            None,
            "https://example.com/paper.pdf",
        );
        let result = generate_sidecar(&item).unwrap();
        assert!(
            result.is_none(),
            "should skip sidecar generation when downloaded file is missing"
        );
        assert!(
            !tmp.path().join("missing.json").exists(),
            "no sidecar should be created when source file is missing"
        );
    }

    #[test]
    fn test_generate_sidecar_creates_json_file_with_correct_content() {
        let tmp = tempfile::TempDir::new().unwrap();
        let pdf_path = tmp.path().join("paper.pdf");
        std::fs::write(&pdf_path, b"fake pdf content").unwrap();

        let item = make_item(
            Some(pdf_path.to_str().unwrap()),
            Some("Test Paper"),
            Some("Alice Smith, Bob Doe"),
            Some("2024"),
            Some("10.1234/test"),
            "https://example.com/paper.pdf",
        );

        let result = generate_sidecar(&item).unwrap();
        assert!(result.is_some(), "should return Some(path) on success");

        let sidecar_path = tmp.path().join("paper.json");
        assert!(sidecar_path.exists(), "sidecar file should exist on disk");

        let content = std::fs::read_to_string(&sidecar_path).unwrap();
        let value: serde_json::Value = serde_json::from_str(&content).unwrap();
        assert_eq!(value["@context"], "https://schema.org");
        assert_eq!(value["@type"], "ScholarlyArticle");
        assert_eq!(value["name"], "Test Paper");
        assert_eq!(value["datePublished"], "2024");
        assert_eq!(value["identifier"]["value"], "10.1234/test");

        let authors = value["author"].as_array().unwrap();
        assert_eq!(authors.len(), 2);
        assert_eq!(authors[0]["name"], "Alice Smith");
    }

    #[test]
    fn test_generate_sidecar_existing_sidecar_not_overwritten() {
        let tmp = tempfile::TempDir::new().unwrap();
        let pdf_path = tmp.path().join("paper.pdf");
        std::fs::write(&pdf_path, b"fake pdf").unwrap();

        // Write sentinel content to pre-existing sidecar
        let sidecar_path = tmp.path().join("paper.json");
        let sentinel = r#"{"sentinel": "original content"}"#;
        std::fs::write(&sidecar_path, sentinel).unwrap();

        let item = make_item(
            Some(pdf_path.to_str().unwrap()),
            Some("New Paper"),
            None,
            None,
            None,
            "https://example.com/paper.pdf",
        );

        let result = generate_sidecar(&item).unwrap();
        assert!(
            result.is_none(),
            "should return None when sidecar already exists"
        );

        // Original content must be preserved
        let content = std::fs::read_to_string(&sidecar_path).unwrap();
        assert_eq!(
            content, sentinel,
            "existing sidecar content should not be overwritten"
        );
    }

    #[test]
    fn test_generate_sidecar_existing_sidecar_logs_skip_with_path() {
        let tmp = tempfile::TempDir::new().unwrap();
        let pdf_path = tmp.path().join("paper.pdf");
        std::fs::write(&pdf_path, b"fake pdf").unwrap();

        let item = make_item(
            Some(pdf_path.to_str().unwrap()),
            Some("Test Paper"),
            None,
            None,
            None,
            "https://example.com/paper.pdf",
        );

        let events = Arc::new(Mutex::new(Vec::<CapturedEvent>::new()));
        let subscriber = tracing_subscriber::registry()
            .with(tracing_subscriber::filter::LevelFilter::DEBUG)
            .with(EventCaptureLayer {
                events: Arc::clone(&events),
            });

        tracing::subscriber::with_default(subscriber, || {
            let first = generate_sidecar(&item).unwrap();
            assert!(first.is_some(), "first sidecar creation should succeed");

            // Refresh interest cache to ensure our subscriber's interests
            // take precedence over any callsite registrations that parallel
            // tests may have made with the noop dispatcher (Interest::Never).
            tracing::callsite::rebuild_interest_cache();

            let second = generate_sidecar(&item).unwrap();
            assert!(
                second.is_none(),
                "second sidecar creation should skip existing file"
            );
        });

        let expected_path = tmp.path().join("paper.json");
        let expected_path_str = expected_path.to_string_lossy();
        let events = events.lock().unwrap();
        let skip_event = events.iter().find(|event| {
            event
                .fields
                .get("message")
                .is_some_and(|message| message.contains("Sidecar already exists, skipping"))
        });

        assert!(
            skip_event.is_some(),
            "debug log should include skip message; captured events: {events:?}"
        );
        let skip_event = skip_event.unwrap();
        let message_matches_path = skip_event
            .fields
            .get("message")
            .is_some_and(|message| message.contains(expected_path_str.as_ref()));
        let field_matches_path = skip_event
            .fields
            .get("path")
            .is_some_and(|path| path.contains(expected_path_str.as_ref()));
        assert!(
            message_matches_path || field_matches_path,
            "debug log should include skipped sidecar path via message or `path` field; event fields: {:?}",
            skip_event.fields
        );
    }

    #[test]
    fn test_generate_sidecar_returns_correct_path() {
        let tmp = tempfile::TempDir::new().unwrap();
        let pdf_path = tmp.path().join("paper.pdf");
        std::fs::write(&pdf_path, b"fake pdf").unwrap();

        let item = make_item(
            Some(pdf_path.to_str().unwrap()),
            None,
            None,
            None,
            None,
            "https://example.com/paper.pdf",
        );

        let result = generate_sidecar(&item).unwrap().unwrap();
        assert_eq!(result, tmp.path().join("paper.json"));
    }
}
