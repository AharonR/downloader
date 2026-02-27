//! Project folder and history path utilities.
//!
//! Resolves project output directories, sanitizes segment names, and discovers
//! history database paths under a base output directory.

use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Result, bail};
use downloader_core::{
    DownloadAttempt, Queue, QueueItem, QueueStatus, generate_sidecar, normalize_topics,
};
use tracing::{info, warn};

use crate::output;

/// Maximum characters per project folder segment (avoids overly long paths).
pub const MAX_PROJECT_FOLDER_CHARS: usize = 80;

/// Maximum nesting depth for project path segments (e.g. "A/B/C" = 3).
pub const MAX_PROJECT_SEGMENTS: usize = 10;
pub(crate) const PROJECT_LOG_QUERY_PAGE_SIZE: usize = 10_000;

/// Resolves the output directory for a run, optionally under a sanitized project subpath.
///
/// If `project` is `None`, returns `base_output_dir`. Otherwise splits `project` on `/`,
/// sanitizes each segment, and returns `base_output_dir/seg1/seg2/...`.
pub fn resolve_project_output_dir(
    base_output_dir: &Path,
    project: Option<&str>,
) -> Result<PathBuf> {
    let Some(raw_project) = project else {
        return Ok(base_output_dir.to_path_buf());
    };

    let trimmed = raw_project.trim();
    if trimmed.is_empty() {
        bail!("--project cannot be empty");
    }
    let normalized = trimmed.replace('\\', "/");
    let raw_segments: Vec<&str> = normalized.split('/').collect();
    if raw_segments.is_empty() {
        bail!("--project cannot be empty");
    }
    if raw_segments.len() > MAX_PROJECT_SEGMENTS {
        bail!(
            "--project nesting depth {} exceeds max {}",
            raw_segments.len(),
            MAX_PROJECT_SEGMENTS
        );
    }

    let mut output_dir = base_output_dir.to_path_buf();
    for segment in raw_segments {
        let clean = sanitize_project_segment(segment)?;
        output_dir.push(clean);
    }

    Ok(output_dir)
}

/// Sanitizes a string for use as a single path segment (no slashes, no reserved names).
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

/// Sanitizes a single project path segment and rejects empty/traversal/reserved names.
pub fn sanitize_project_segment(segment: &str) -> Result<String> {
    let trimmed = segment.trim();
    if trimmed.is_empty() {
        bail!("--project contains an empty path segment");
    }
    if trimmed == "." || trimmed == ".." {
        bail!("--project cannot contain '.' or '..' path segments");
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
        bail!(
            "--project segment '{}' contains no usable folder-name characters",
            segment
        );
    }

    Ok(sanitized)
}

/// Returns a stable key for the project (used for history DB lookups).
pub fn project_history_key(output_dir: &Path) -> String {
    std::fs::canonicalize(output_dir)
        .unwrap_or_else(|_| output_dir.to_path_buf())
        .to_string_lossy()
        .to_string()
}

/// Discovers all `.downloader/queue.db` paths under `base_output_dir` (recursive).
pub fn discover_history_db_paths(base_output_dir: &Path) -> Result<Vec<PathBuf>> {
    if !base_output_dir.exists() {
        return Ok(Vec::new());
    }

    let mut db_paths = Vec::new();
    let mut stack = vec![base_output_dir.to_path_buf()];

    while let Some(dir) = stack.pop() {
        let entries = match fs::read_dir(&dir) {
            Ok(entries) => entries,
            Err(error) => {
                warn!(
                    path = %dir.display(),
                    error = %error,
                    "Skipping unreadable directory while discovering history databases"
                );
                continue;
            }
        };

        for entry in entries {
            let entry = match entry {
                Ok(entry) => entry,
                Err(error) => {
                    warn!(error = %error, "Skipping unreadable directory entry");
                    continue;
                }
            };
            let file_type = match entry.file_type() {
                Ok(file_type) => file_type,
                Err(error) => {
                    warn!(
                        path = %entry.path().display(),
                        error = %error,
                        "Skipping entry with unreadable file type"
                    );
                    continue;
                }
            };
            if !file_type.is_dir() {
                continue;
            }

            let path = entry.path();
            if entry.file_name() == ".downloader" {
                let db_path = path.join("queue.db");
                if db_path.exists() {
                    db_paths.push(db_path);
                }
                continue;
            }

            stack.push(path);
        }
    }

    db_paths.sort();
    db_paths.dedup();
    Ok(db_paths)
}

/// Returns true if the name is a Windows reserved name (CON, PRN, AUX, NUL, COM1–9, LPT1–9).
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

pub(crate) async fn append_project_download_log(
    queue: &Queue,
    output_dir: &Path,
    history_start_id: Option<i64>,
) -> Result<()> {
    let project_key = project_history_key(output_dir);
    let mut attempts = Vec::new();
    let mut page_count = 0usize;
    let mut before_id = None;
    loop {
        let query = downloader_core::DownloadAttemptQuery {
            project: Some(project_key.clone()),
            after_id: history_start_id,
            before_id,
            limit: PROJECT_LOG_QUERY_PAGE_SIZE,
            ..downloader_core::DownloadAttemptQuery::default()
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

    let session_label = format!(
        "unix-{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|duration| duration.as_secs())
            .unwrap_or(0)
    );
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

pub(crate) fn render_project_download_log_section(
    session_label: &str,
    attempts: &[DownloadAttempt],
) -> String {
    let mut output_text = String::new();
    output_text.push_str(&format!(
        "## Session {session_label} ({} attempts)\n\n",
        attempts.len()
    ));
    for attempt in attempts {
        output_text.push_str(&format!("{}\n", render_project_download_log_entry(attempt)));
    }
    output_text.push('\n');
    output_text
}

fn render_project_download_log_entry(attempt: &DownloadAttempt) -> String {
    let timestamp = &attempt.started_at;
    let status = attempt.status().as_str().to_ascii_uppercase();
    let filename = output::truncate_log_field(&output::download_log_filename(attempt), 48);
    let source = output::truncate_log_field(output::download_log_source(attempt), 72);
    let row_ref = format!("history#{}", attempt.id);

    if attempt.status() == downloader_core::DownloadAttemptStatus::Failed {
        let reason_text = attempt
            .error_message
            .as_deref()
            .unwrap_or("unknown failure");
        let reason_first_line = reason_text.lines().next().unwrap_or(reason_text);
        let reason = output::truncate_log_field(reason_first_line, 96);
        format!(
            "- {timestamp} | {status} | file={filename} | source={source} | reason={reason} | ref={row_ref}"
        )
    } else {
        format!("- {timestamp} | {status} | file={filename} | source={source} | ref={row_ref}")
    }
}

pub(crate) async fn append_project_index(
    queue: &Queue,
    output_dir: &Path,
    completed_before: &std::collections::HashSet<i64>,
) -> Result<()> {
    let mut new_items: Vec<_> = queue
        .list_by_status(downloader_core::QueueStatus::Completed)
        .await?
        .into_iter()
        .filter(|item| !completed_before.contains(&item.id))
        .collect();

    if new_items.is_empty() {
        return Ok(());
    }

    new_items.sort_by_key(|item| item.id);
    let session_label = format!(
        "unix-{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|duration| duration.as_secs())
            .unwrap_or(0)
    );

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

pub(crate) async fn generate_sidecars_for_completed(
    queue: &Queue,
    completed_before: &std::collections::HashSet<i64>,
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

pub(crate) fn render_project_index_section(session_label: &str, items: &[QueueItem]) -> String {
    let mut output_text = String::new();
    output_text.push_str(&format!("## Session {session_label}\n\n"));

    let all_topics: Vec<String> = items.iter().flat_map(|item| item.parse_topics()).collect();
    let unique_topics = normalize_topics(all_topics);
    if !unique_topics.is_empty() {
        output_text.push_str(&format!(
            "**Topics detected:** {} | {}\n\n",
            unique_topics.len(),
            unique_topics.join(", ")
        ));
    }

    output_text.push_str("| Filename | Title | Authors | Source URL |\n");
    output_text.push_str("| --- | --- | --- | --- |\n");

    for item in items {
        let filename = item
            .saved_path
            .as_deref()
            .and_then(|path| Path::new(path).file_name().and_then(|name| name.to_str()))
            .unwrap_or("unknown");
        let title = item.meta_title.as_deref().unwrap_or("n/a");
        let authors = item.meta_authors.as_deref().unwrap_or("n/a");

        output_text.push_str(&format!(
            "| `{}` | {} | {} | <{}> |\n",
            output::escape_markdown_cell(filename),
            output::escape_markdown_cell(title),
            output::escape_markdown_cell(authors),
            item.url
        ));
    }
    output_text.push('\n');
    output_text
}

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
}
