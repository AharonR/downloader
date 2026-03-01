//! Project folder utilities shared between the CLI and desktop app.
//!
//! Provides path sanitisation, output directory resolution, and helpers for
//! generating per-project artefacts (index.md, download.log, JSON-LD sidecars).

use std::collections::HashSet;
use std::fmt::Write as _;
use std::fs;
use std::hash::BuildHasher;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use tracing::{info, warn};

use crate::{
    DownloadAttempt, DownloadAttemptQuery, DownloadAttemptStatus, Queue, QueueError, QueueItem,
    QueueStatus, generate_sidecar, normalize_topics,
};

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

/// Maximum characters per project folder segment (avoids overly long paths).
pub const MAX_PROJECT_FOLDER_CHARS: usize = 80;

/// Maximum nesting depth for project path segments (e.g. "A/B/C" = 3).
pub const MAX_PROJECT_SEGMENTS: usize = 10;

/// Page size for paged history queries in [`append_project_download_log`].
pub const PROJECT_LOG_QUERY_PAGE_SIZE: usize = 10_000;

// Process-lifetime counter: ensures session labels are unique even when two
// sessions complete within the same wall-clock second.
static SESSION_SEQ: AtomicU64 = AtomicU64::new(0);

/// Returns a unique session label combining Unix seconds and a per-process counter.
///
/// `SystemTime::now()` is the single wall-clock access point in this module.
/// Replace with a `Clock` trait injection if test-injectable time becomes needed.
fn make_session_label() -> String {
    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let seq = SESSION_SEQ.fetch_add(1, Ordering::Relaxed);
    format!("unix-{secs}-{seq}")
}

// ---------------------------------------------------------------------------
// Error type
// ---------------------------------------------------------------------------

/// Errors produced by project path operations.
#[derive(Debug, thiserror::Error)]
pub enum ProjectError {
    /// The project name or segment is empty.
    #[error("project name is empty")]
    EmptyName,
    /// A segment produced no usable folder-name characters after sanitisation.
    #[error("project segment '{0}' contains no usable characters")]
    InvalidSegment(String),
    /// The nesting depth exceeds [`MAX_PROJECT_SEGMENTS`].
    #[error("project nesting depth {0} exceeds max {1}")]
    TooDeep(usize, usize),
    /// A segment that would escape the base directory was rejected.
    #[error("project path traversal rejected: '{0}'")]
    PathTraversal(String),
    /// An underlying I/O error (e.g. reading/writing index.md or download.log).
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    /// An underlying queue/database error.
    #[error(transparent)]
    Queue(#[from] QueueError),
}

// ---------------------------------------------------------------------------
// Path resolution
// ---------------------------------------------------------------------------

/// Resolves the output directory for a run, optionally under a sanitised project subpath.
///
/// If `project` is `None`, returns `base_output_dir`. Otherwise splits `project` on `/`,
/// sanitises each segment, and returns `base_output_dir/seg1/seg2/…`.
///
/// # Errors
///
/// Returns [`ProjectError::EmptyName`] if the project name or any segment is empty,
/// [`ProjectError::TooDeep`] if the depth exceeds [`MAX_PROJECT_SEGMENTS`], or
/// [`ProjectError::PathTraversal`] if a segment is `.` or `..`.
pub fn resolve_project_output_dir(
    base_output_dir: &Path,
    project: Option<&str>,
) -> Result<PathBuf, ProjectError> {
    let Some(raw_project) = project else {
        return Ok(base_output_dir.to_path_buf());
    };

    let trimmed = raw_project.trim();
    if trimmed.is_empty() {
        return Err(ProjectError::EmptyName);
    }
    let normalized = trimmed.replace('\\', "/");
    let raw_segments: Vec<&str> = normalized.split('/').collect();
    if raw_segments.is_empty() {
        return Err(ProjectError::EmptyName);
    }
    if raw_segments.len() > MAX_PROJECT_SEGMENTS {
        return Err(ProjectError::TooDeep(
            raw_segments.len(),
            MAX_PROJECT_SEGMENTS,
        ));
    }

    let mut output_dir = base_output_dir.to_path_buf();
    for segment in raw_segments {
        let clean = sanitize_project_segment(segment)?;
        output_dir.push(clean);
    }

    Ok(output_dir)
}

/// Returns a stable key for the project used for history DB lookups.
#[must_use]
pub fn project_history_key(output_dir: &Path) -> String {
    std::fs::canonicalize(output_dir)
        .unwrap_or_else(|e| {
            warn!(
                path = %output_dir.display(),
                error = %e,
                "canonicalize failed; using raw path as project key — DB lookups may miss prior history"
            );
            output_dir.to_path_buf()
        })
        .to_string_lossy()
        .to_string()
}

// ---------------------------------------------------------------------------
// Name sanitisation
// ---------------------------------------------------------------------------

/// Sanitises a string for use as a single path segment (no slashes, no reserved names).
#[must_use]
pub fn sanitize_project_name(name: &str) -> String {
    let mut sanitized = String::new();
    let mut previous_dash = false;

    for ch in name.trim().chars() {
        let mapped = match ch {
            '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|' => '-',
            c if c.is_whitespace() || c.is_control() => '-',
            c => c,
        };

        if mapped == '-' {
            if !previous_dash {
                sanitized.push('-');
                previous_dash = true;
            }
        } else {
            sanitized.push(mapped);
            previous_dash = false;
        }
    }

    sanitized.trim_matches('-').to_string()
}

/// Sanitises a single project path segment and rejects empty/traversal/reserved names.
///
/// # Errors
///
/// Returns [`ProjectError::EmptyName`] if the segment is empty,
/// [`ProjectError::PathTraversal`] if the segment is `.` or `..`, or
/// [`ProjectError::InvalidSegment`] if sanitisation leaves no usable characters.
pub fn sanitize_project_segment(segment: &str) -> Result<String, ProjectError> {
    let trimmed = segment.trim();
    if trimmed.is_empty() {
        return Err(ProjectError::EmptyName);
    }
    if trimmed == "." || trimmed == ".." {
        return Err(ProjectError::PathTraversal(trimmed.to_string()));
    }

    let mut sanitized = sanitize_project_name(trimmed).trim_matches('.').to_string();
    if is_windows_reserved_name(&sanitized) {
        sanitized.push_str("-project");
    }
    if sanitized.chars().count() > MAX_PROJECT_FOLDER_CHARS {
        sanitized = sanitized.chars().take(MAX_PROJECT_FOLDER_CHARS).collect();
        sanitized = sanitized.trim_matches('-').to_string();
    }
    if sanitized.is_empty() {
        return Err(ProjectError::InvalidSegment(segment.to_string()));
    }

    Ok(sanitized)
}

/// Returns `true` if the name is a Windows reserved device name
/// (CON, PRN, AUX, NUL, COM1–9, LPT1–9).
#[must_use]
pub fn is_windows_reserved_name(name: &str) -> bool {
    let upper = name.to_uppercase();
    matches!(
        upper.as_str(),
        "CON"
            | "PRN"
            | "AUX"
            | "NUL"
            | "COM1"
            | "COM2"
            | "COM3"
            | "COM4"
            | "COM5"
            | "COM6"
            | "COM7"
            | "COM8"
            | "COM9"
            | "LPT1"
            | "LPT2"
            | "LPT3"
            | "LPT4"
            | "LPT5"
            | "LPT6"
            | "LPT7"
            | "LPT8"
            | "LPT9"
    )
}

// ---------------------------------------------------------------------------
// Formatting helpers
// ---------------------------------------------------------------------------

/// Truncates a log/display field to at most `max` characters, appending `…` when truncated.
#[must_use]
pub fn truncate_field(value: &str, max: usize) -> String {
    if value.chars().count() <= max {
        return value.to_string();
    }
    let mut truncated: String = value.chars().take(max.saturating_sub(1)).collect();
    truncated.push('…');
    truncated
}

/// Escapes markdown table cell characters (pipe, backtick, newlines).
#[must_use]
pub fn escape_markdown_cell(value: &str) -> String {
    value
        .replace('|', "\\|")
        .replace('`', "\\`")
        .replace(['\n', '\r'], " ")
}

// ---------------------------------------------------------------------------
// Private helpers for download log entries
// ---------------------------------------------------------------------------

fn download_log_filename(attempt: &DownloadAttempt) -> String {
    attempt
        .file_path
        .as_deref()
        .and_then(|path| Path::new(path).file_name().and_then(|name| name.to_str()))
        .map(ToString::to_string)
        .or_else(|| attempt.title.clone())
        .unwrap_or_else(|| "n/a".to_string())
}

fn download_log_source(attempt: &DownloadAttempt) -> &str {
    attempt
        .original_input
        .as_deref()
        .unwrap_or(attempt.url.as_str())
}

// ---------------------------------------------------------------------------
// Project artefact generators
// ---------------------------------------------------------------------------

/// Appends a new session section to the project's `download.log` file.
///
/// Queries `queue` for all [`DownloadAttempt`]s belonging to the project derived from
/// `output_dir`, filtering to rows with `id > history_start_id` (pass `None` for all rows).
///
/// # Errors
///
/// Returns [`ProjectError::Queue`] on database errors or
/// [`ProjectError::Io`] on filesystem errors.
pub async fn append_project_download_log(
    queue: &Queue,
    output_dir: &Path,
    history_start_id: Option<i64>,
) -> Result<(), ProjectError> {
    let project_key = project_history_key(output_dir);
    let mut attempts = Vec::new();
    let mut page_count = 0usize;
    let mut before_id = None;
    loop {
        let query = DownloadAttemptQuery {
            project: Some(project_key.clone()),
            after_id: history_start_id,
            before_id,
            limit: PROJECT_LOG_QUERY_PAGE_SIZE,
            ..DownloadAttemptQuery::default()
        };
        let mut page = queue.query_download_attempts(&query).await?;
        if page.is_empty() {
            break;
        }
        before_id = page.last().map(|attempt| attempt.id);
        page_count = page_count.saturating_add(1);
        attempts.append(&mut page);
    }

    if attempts.is_empty() {
        return Ok(());
    }
    attempts.sort_by_key(|attempt| attempt.id);

    if page_count > 1 {
        info!(
            page_count,
            entries = attempts.len(),
            page_size = PROJECT_LOG_QUERY_PAGE_SIZE,
            "Processed new history rows across multiple pages"
        );
    }

    let session_label = make_session_label();
    let section = render_project_download_log_section(&session_label, &attempts);
    let log_path = output_dir.join("download.log");

    let mut content = if log_path.exists() {
        fs::read_to_string(&log_path)?
    } else {
        "# Project Download Log\n\n# References `.downloader/queue.db` table `download_log`.\n"
            .to_string()
    };
    if !content.ends_with('\n') {
        content.push('\n');
    }
    content.push('\n');
    content.push_str(&section);
    fs::write(&log_path, content)?;

    info!(
        path = %log_path.display(),
        entries = attempts.len(),
        page_count,
        "Project download log updated"
    );
    Ok(())
}

/// Renders a single session block for the `download.log` file.
#[must_use]
pub fn render_project_download_log_section(
    session_label: &str,
    attempts: &[DownloadAttempt],
) -> String {
    let mut out = String::new();
    let _ = write!(
        out,
        "## Session {session_label} ({} attempts)\n\n",
        attempts.len()
    );
    for attempt in attempts {
        let _ = writeln!(out, "{}", render_project_download_log_entry(attempt));
    }
    out.push('\n');
    out
}

fn render_project_download_log_entry(attempt: &DownloadAttempt) -> String {
    let timestamp = &attempt.started_at;
    let status = attempt.status().as_str().to_ascii_uppercase();
    let filename = truncate_field(&download_log_filename(attempt), 48);
    let source = truncate_field(download_log_source(attempt), 72);
    let row_ref = format!("history#{}", attempt.id);

    if attempt.status() == DownloadAttemptStatus::Failed {
        let reason_text = attempt
            .error_message
            .as_deref()
            .unwrap_or("unknown failure");
        let reason_first_line = reason_text.lines().next().unwrap_or(reason_text);
        let reason = truncate_field(reason_first_line, 96);
        format!(
            "- {timestamp} | {status} | file={filename} | source={source} | reason={reason} | ref={row_ref}"
        )
    } else {
        format!("- {timestamp} | {status} | file={filename} | source={source} | ref={row_ref}")
    }
}

/// Appends a new session section to the project's `index.md` file.
///
/// Only items whose `id` is not in `completed_before` are included.
///
/// # Errors
///
/// Returns [`ProjectError::Queue`] on database errors or
/// [`ProjectError::Io`] on filesystem errors.
pub async fn append_project_index<S: BuildHasher>(
    queue: &Queue,
    output_dir: &Path,
    completed_before: &HashSet<i64, S>,
) -> Result<(), ProjectError> {
    let mut new_items: Vec<_> = queue
        .list_by_status(QueueStatus::Completed)
        .await?
        .into_iter()
        .filter(|item| !completed_before.contains(&item.id))
        .collect();

    if new_items.is_empty() {
        return Ok(());
    }

    new_items.sort_by_key(|item| item.id);
    let session_label = make_session_label();

    let section = render_project_index_section(&session_label, &new_items);
    let index_path = output_dir.join("index.md");

    let mut content = if index_path.exists() {
        fs::read_to_string(&index_path)?
    } else {
        "# Project Index\n".to_string()
    };
    if !content.ends_with('\n') {
        content.push('\n');
    }
    content.push('\n');
    content.push_str(&section);
    fs::write(&index_path, content)?;

    info!(
        path = %index_path.display(),
        entries = new_items.len(),
        "Project index updated"
    );
    Ok(())
}

/// Generates JSON-LD sidecars for completed items not yet seen before this run.
///
/// Returns the number of sidecars successfully created.
pub async fn generate_sidecars_for_completed<S: BuildHasher>(
    queue: &Queue,
    completed_before: &HashSet<i64, S>,
) -> usize {
    let items = match queue.list_by_status(QueueStatus::Completed).await {
        Ok(items) => items,
        Err(err) => {
            warn!(
                ?err,
                "Failed to query completed queue items for sidecar generation"
            );
            return 0;
        }
    };

    let mut created = 0usize;
    for item in items
        .into_iter()
        .filter(|item| item.saved_path.is_some() && !completed_before.contains(&item.id))
    {
        match generate_sidecar(&item) {
            Ok(Some(_)) => created += 1,
            Ok(None) => {}
            Err(err) => {
                warn!(
                    item_id = item.id,
                    ?err,
                    "Sidecar generation failed, continuing"
                );
            }
        }
    }
    created
}

/// Renders a session section for the `index.md` file.
#[must_use]
pub fn render_project_index_section(session_label: &str, items: &[QueueItem]) -> String {
    let mut out = String::new();
    let _ = write!(out, "## Session {session_label}\n\n");

    let all_topics: Vec<String> = items.iter().flat_map(QueueItem::parse_topics).collect();
    let unique_topics = normalize_topics(all_topics);
    if !unique_topics.is_empty() {
        let _ = write!(
            out,
            "**Topics detected:** {} | {}\n\n",
            unique_topics.len(),
            unique_topics.join(", ")
        );
    }

    out.push_str("| Filename | Title | Authors | Source URL |\n");
    out.push_str("| --- | --- | --- | --- |\n");

    for item in items {
        let filename = item
            .saved_path
            .as_deref()
            .and_then(|path| Path::new(path).file_name().and_then(|name| name.to_str()))
            .unwrap_or("unknown");
        let title = item.meta_title.as_deref().unwrap_or("n/a");
        let authors = item.meta_authors.as_deref().unwrap_or("n/a");

        let _ = writeln!(
            out,
            "| `{}` | {} | {} | <{}> |",
            escape_markdown_cell(filename),
            escape_markdown_cell(title),
            escape_markdown_cell(authors),
            escape_markdown_cell(item.url.as_str())
        );
    }
    out.push('\n');
    out
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sanitize_project_segment_rejects_empty() {
        let err = sanitize_project_segment("").unwrap_err();
        assert!(err.to_string().contains("empty"));
    }

    #[test]
    fn test_sanitize_project_segment_rejects_dot() {
        assert!(sanitize_project_segment(".").is_err());
        assert!(sanitize_project_segment("..").is_err());
    }

    #[test]
    fn test_sanitize_project_segment_normal_segment() {
        assert_eq!(
            sanitize_project_segment("My Project").unwrap(),
            "My-Project"
        );
        assert_eq!(sanitize_project_segment("  valid  ").unwrap(), "valid");
    }

    #[test]
    fn test_sanitize_project_segment_reserved_name_gets_suffix() {
        let out = sanitize_project_segment("CON").unwrap();
        assert_eq!(out, "CON-project");
    }

    #[test]
    fn test_is_windows_reserved_name() {
        assert!(is_windows_reserved_name("CON"));
        assert!(is_windows_reserved_name("con"));
        assert!(is_windows_reserved_name("PRN"));
        assert!(is_windows_reserved_name("LPT1"));
        assert!(!is_windows_reserved_name("docs"));
        assert!(!is_windows_reserved_name("CON-project"));
    }

    #[test]
    fn test_resolve_project_output_dir_creates_subpath() {
        let base = PathBuf::from("/tmp/output");
        let result = resolve_project_output_dir(&base, Some("Climate Research")).unwrap();
        assert_eq!(result, PathBuf::from("/tmp/output/Climate-Research"));
    }

    #[test]
    fn test_resolve_project_output_dir_none_returns_base() {
        let base = PathBuf::from("/tmp/output");
        let result = resolve_project_output_dir(&base, None).unwrap();
        assert_eq!(result, base);
    }

    #[test]
    fn test_resolve_project_output_dir_nested() {
        let base = PathBuf::from("/tmp/output");
        let result = resolve_project_output_dir(&base, Some("Climate/Emissions/2024")).unwrap();
        assert_eq!(result, PathBuf::from("/tmp/output/Climate/Emissions/2024"));
    }

    #[test]
    fn test_resolve_project_output_dir_rejects_traversal() {
        let base = PathBuf::from("/tmp/output");
        let err = resolve_project_output_dir(&base, Some("../secret")).unwrap_err();
        assert!(matches!(err, ProjectError::PathTraversal(_)));
    }

    #[test]
    fn test_resolve_project_output_dir_rejects_empty_segment() {
        let base = PathBuf::from("/tmp/output");
        let err = resolve_project_output_dir(&base, Some("Climate//2024")).unwrap_err();
        assert!(matches!(err, ProjectError::EmptyName));
    }

    #[test]
    fn test_truncate_field_exact_fit() {
        assert_eq!(truncate_field("hello", 5), "hello");
        assert_eq!(truncate_field("hello", 10), "hello");
    }

    #[test]
    fn test_truncate_field_truncates_with_ellipsis() {
        assert_eq!(truncate_field("1234567890", 6), "12345…");
    }

    #[test]
    fn test_truncate_field_empty_string() {
        assert_eq!(truncate_field("", 10), "");
        assert_eq!(truncate_field("", 0), "");
    }

    #[test]
    fn test_truncate_field_max_zero_appends_ellipsis() {
        // max=0: take 0 chars (saturating_sub(1) = 0), then push '…' → "…"
        assert_eq!(truncate_field("any", 0), "…");
    }

    #[test]
    fn test_truncate_field_single_char_max() {
        // max=1: take 0 chars, push '…' → "…"
        assert_eq!(truncate_field("ab", 1), "…");
    }

    #[test]
    fn test_escape_markdown_cell_replaces_pipes_and_backticks() {
        assert_eq!(escape_markdown_cell("a|b"), "a\\|b");
        assert_eq!(escape_markdown_cell("a`b"), "a\\`b");
        assert_eq!(escape_markdown_cell("a\nb"), "a b");
        assert_eq!(
            escape_markdown_cell("A|B\nline`one\rline2"),
            "A\\|B line\\`one line2"
        );
    }

    fn make_test_attempt(id: i64, status_str: &str, file_path: Option<&str>) -> DownloadAttempt {
        DownloadAttempt {
            id,
            url: "https://example.com/paper.pdf".to_string(),
            status_str: status_str.to_string(),
            file_path: file_path.map(ToString::to_string),
            title: Some("Test Paper".to_string()),
            authors: None,
            doi: None,
            parse_confidence: None,
            parse_confidence_factors: None,
            project: None,
            started_at: "2026-02-28T10:00:00Z".to_string(),
            error_message: None,
            error_type: None,
            retry_count: 0,
            last_retry_at: None,
            original_input: Some("https://example.com/paper.pdf".to_string()),
            http_status: None,
            duration_ms: None,
        }
    }

    fn make_test_item(id: i64, topics: Option<&str>) -> QueueItem {
        QueueItem {
            id,
            url: "https://example.com/paper.pdf".to_string(),
            source_type: "direct_url".to_string(),
            original_input: None,
            status_str: "completed".to_string(),
            priority: 0,
            retry_count: 0,
            last_error: None,
            suggested_filename: None,
            meta_title: Some("Test Paper".to_string()),
            meta_authors: Some("Smith, J.".to_string()),
            meta_year: None,
            meta_doi: None,
            topics: topics.map(ToString::to_string),
            parse_confidence: None,
            parse_confidence_factors: None,
            saved_path: Some("/tmp/Climate-Research/paper.pdf".to_string()),
            bytes_downloaded: 0,
            content_length: None,
            created_at: "2026-02-28T10:00:00Z".to_string(),
            updated_at: "2026-02-28T10:00:00Z".to_string(),
        }
    }

    #[test]
    fn test_render_project_download_log_section_structure() {
        let attempt = make_test_attempt(42, "success", Some("/tmp/paper.pdf"));
        let output = render_project_download_log_section("unix-1234567890", &[attempt]);

        assert!(
            output.contains("## Session unix-1234567890"),
            "missing session header"
        );
        assert!(output.contains("(1 attempts)"), "missing attempt count");
        assert!(output.contains("SUCCESS"), "missing status");
        assert!(output.contains("paper.pdf"), "missing filename");
        assert!(output.contains("ref=history#42"), "missing row ref");
    }

    #[test]
    fn test_render_project_download_log_section_empty() {
        let output = render_project_download_log_section("unix-0", &[]);
        assert!(output.contains("## Session unix-0"), "missing header");
        assert!(output.contains("(0 attempts)"), "missing zero count");
    }

    #[test]
    fn test_render_project_download_log_section_failed_includes_reason() {
        let mut attempt = make_test_attempt(99, "failed", None);
        attempt.error_message = Some("Connection refused".to_string());
        let output = render_project_download_log_section("unix-0", &[attempt]);

        assert!(output.contains("FAILED"), "missing FAILED status");
        assert!(
            output.contains("reason=Connection refused"),
            "missing failure reason"
        );
    }

    #[test]
    fn test_render_project_index_section_structure() {
        let item = make_test_item(7, None);
        let output = render_project_index_section("unix-1234567890", &[item]);

        assert!(
            output.contains("## Session unix-1234567890"),
            "missing session header"
        );
        assert!(output.contains("paper.pdf"), "missing filename");
        assert!(output.contains("Test Paper"), "missing title");
        assert!(output.contains("Smith, J."), "missing authors");
        assert!(output.contains("| --- |"), "missing table separator");
    }

    #[test]
    fn test_render_project_index_section_topics_line_present_when_nonempty() {
        let item = make_test_item(1, Some(r#"["machine learning","neural networks"]"#));
        let output = render_project_index_section("unix-0", &[item]);
        assert!(
            output.contains("**Topics detected:**"),
            "expected topics line"
        );
    }

    #[test]
    fn test_render_project_index_section_no_topics_line_when_empty() {
        let item = make_test_item(2, None);
        let output = render_project_index_section("unix-0", &[item]);
        assert!(
            !output.contains("**Topics detected:**"),
            "topics line should be absent"
        );
    }

    #[test]
    fn test_make_session_label_unique_across_rapid_calls() {
        // Session labels must be distinct even if the wall clock doesn't advance.
        let labels: Vec<String> = (0..5).map(|_| make_session_label()).collect();
        let unique: std::collections::HashSet<_> = labels.iter().collect();
        assert_eq!(
            unique.len(),
            labels.len(),
            "session labels must all be unique: {:?}",
            labels
        );
    }

    #[test]
    fn test_make_session_label_has_expected_prefix() {
        let label = make_session_label();
        assert!(
            label.starts_with("unix-"),
            "session label should start with 'unix-', got: {label}"
        );
    }

    #[test]
    fn test_render_project_index_section_escapes_url_pipe_chars() {
        // URLs with '|' in query strings must not break the markdown table.
        let mut item = make_test_item(10, None);
        item.url = "https://example.com/?a=1|b=2".to_string();
        let output = render_project_index_section("unix-0", &[item]);
        // The pipe in the URL must be escaped so it doesn't act as a column delimiter.
        assert!(
            output.contains("\\|"),
            "pipe in URL should be escaped in markdown table"
        );
    }
}
