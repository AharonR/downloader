use std::path::Path;
use std::time::{Duration, Instant};

use tracing::{debug, info, warn};

use crate::generate_sidecar;
use crate::queue::{DownloadAttemptStatus, NewDownloadAttempt, QueueItem, QueueRepository};

use super::DownloadError;
use super::DownloadFileResult;
use super::DownloadStats;
use super::error_mapping::{
    build_actionable_error_message, classify_download_error_type, extract_http_status,
};

pub(super) async fn persist_download_success(
    queue: &impl QueueRepository,
    item: &QueueItem,
    download: &DownloadFileResult,
    project_key: &str,
    attempt_started: Instant,
    generate_sidecars: bool,
    stats: &DownloadStats,
) {
    if download.resume_attempted {
        info!(
            item_id = item.id,
            resumed = download.resumed,
            bytes = download.bytes_downloaded,
            "resume attempt recorded"
        );
    }
    debug!(
        item_id = item.id,
        path = %download.path.display(),
        "download completed"
    );
    if let Err(error) = queue
        .update_progress(
            item.id,
            i64::try_from(download.bytes_downloaded).unwrap_or(i64::MAX),
            download
                .content_length
                .and_then(|value| i64::try_from(value).ok()),
        )
        .await
    {
        warn!(
            item_id = item.id,
            error = %error,
            "failed to update progress metadata"
        );
    }
    if let Err(error) = queue
        .mark_completed_with_path(item.id, Some(&download.path))
        .await
    {
        warn!(
            item_id = item.id,
            error = %error,
            "failed to mark item completed"
        );
    }

    let doi = extract_attempt_doi(item);
    let saved_path = download.path.to_string_lossy().to_string();

    if generate_sidecars {
        let mut sidecar_item = item.clone();
        sidecar_item.saved_path = Some(saved_path.clone());
        if let Err(error) = generate_sidecar(&sidecar_item) {
            warn!(
                item_id = item.id,
                ?error,
                "Sidecar generation failed, continuing"
            );
        }
    }

    let original_input = item.original_input.as_deref().unwrap_or(item.url.as_str());
    let attempt = NewDownloadAttempt {
        url: &item.url,
        final_url: Some(&item.url),
        status: DownloadAttemptStatus::Success,
        file_path: Some(&saved_path),
        file_size: Some(i64::try_from(download.bytes_downloaded).unwrap_or(i64::MAX)),
        content_type: None,
        error_message: None,
        error_type: None,
        retry_count: 0,
        project: Some(project_key),
        original_input: Some(original_input),
        http_status: None,
        duration_ms: Some(elapsed_ms_i64(attempt_started.elapsed())),
        title: item.meta_title.as_deref(),
        authors: item.meta_authors.as_deref(),
        doi: doi.as_deref(),
        topics: item.topics.as_deref(),
        parse_confidence: item.parse_confidence.as_deref(),
        parse_confidence_factors: item.parse_confidence_factors.as_deref(),
    };
    if let Err(error) = queue.log_download_attempt(&attempt).await {
        warn!(
            item_id = item.id,
            error = %error,
            "failed to persist download history row"
        );
    }
    stats.increment_completed();
}

pub(super) async fn persist_download_failure(
    queue: &impl QueueRepository,
    item: &QueueItem,
    error: &DownloadError,
    attempts: u32,
    project_key: &str,
    attempt_started: Instant,
    stats: &DownloadStats,
) {
    let error_type = classify_download_error_type(error);
    let error_message = build_actionable_error_message(error, error_type);
    let doi = extract_attempt_doi(item);
    let original_input = item.original_input.as_deref().unwrap_or(item.url.as_str());

    warn!(
        item_id = item.id,
        url = %item.url,
        error = %error_message,
        attempts,
        "download failed after all attempts"
    );

    let retry_count = i64::from(attempts.saturating_sub(1));
    if let Err(queue_error) = queue
        .mark_failed(item.id, &error_message, retry_count)
        .await
    {
        warn!(
            item_id = item.id,
            error = %queue_error,
            "failed to mark item failed"
        );
    }

    let attempt = NewDownloadAttempt {
        url: &item.url,
        final_url: None,
        status: DownloadAttemptStatus::Failed,
        file_path: None,
        file_size: None,
        content_type: None,
        error_message: Some(&error_message),
        error_type: Some(error_type),
        retry_count,
        project: Some(project_key),
        original_input: Some(original_input),
        http_status: extract_http_status(error),
        duration_ms: Some(elapsed_ms_i64(attempt_started.elapsed())),
        title: item.meta_title.as_deref(),
        authors: item.meta_authors.as_deref(),
        doi: doi.as_deref(),
        topics: item.topics.as_deref(),
        parse_confidence: item.parse_confidence.as_deref(),
        parse_confidence_factors: item.parse_confidence_factors.as_deref(),
    };
    if let Err(history_error) = queue.log_download_attempt(&attempt).await {
        warn!(
            item_id = item.id,
            error = %history_error,
            "failed to persist download history row"
        );
    }
    stats.increment_failed();
}

pub(super) fn derive_project_key(output_dir: &Path) -> String {
    std::fs::canonicalize(output_dir)
        .unwrap_or_else(|_| output_dir.to_path_buf())
        .to_string_lossy()
        .to_string()
}

fn elapsed_ms_i64(duration: Duration) -> i64 {
    i64::try_from(duration.as_millis()).unwrap_or(i64::MAX)
}

fn extract_attempt_doi(item: &QueueItem) -> Option<String> {
    if let Some(doi) = item
        .meta_doi
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        return Some(doi.to_string());
    }

    if item.source_type != "doi" {
        return None;
    }

    let candidate = item.original_input.as_deref().unwrap_or(item.url.as_str());
    let normalized = normalize_doi_candidate(candidate);
    if normalized.is_empty() {
        None
    } else {
        Some(normalized)
    }
}

fn normalize_doi_candidate(raw: &str) -> String {
    let trimmed = raw.trim();
    let lower = trimmed.to_ascii_lowercase();

    for prefix in [
        "https://doi.org/",
        "http://doi.org/",
        "https://dx.doi.org/",
        "http://dx.doi.org/",
    ] {
        if lower.starts_with(prefix) {
            return trimmed[prefix.len()..].trim().to_string();
        }
    }

    if lower.starts_with("doi:") {
        return trimmed[4..].trim().to_string();
    }

    trimmed.to_string()
}
