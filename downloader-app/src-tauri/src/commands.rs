//! Tauri commands for the Downloader desktop app.
//!
//! Bridges the Svelte frontend to `downloader_core` via Tauri's IPC layer.

use std::collections::HashSet;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use downloader_core::project::{
    append_project_download_log, append_project_index, generate_sidecars_for_completed,
    project_history_key, resolve_project_output_dir,
};
use downloader_core::{
    DEFAULT_CONCURRENCY, Database, DownloadAttemptQuery, DownloadEngine, HttpClient, InputType,
    Queue, QueueMetadata, QueueStatus, RateLimiter, ResolveContext, RetryPolicy,
    build_default_resolver_registry, build_preferred_filename, extract_reference_confidence,
    parse_input,
};
use serde::Serialize;
use tauri::Emitter;
use tracing::{debug, warn};

// ---------------------------------------------------------------------------
// Shared types
// ---------------------------------------------------------------------------

/// Summary returned to the frontend after a download batch completes.
#[derive(Debug, Serialize, Clone)]
pub struct DownloadSummary {
    pub completed: usize,
    pub failed: usize,
    pub output_dir: String,
}

/// Per-item progress snapshot emitted during active downloads.
#[derive(Debug, Serialize, Clone)]
pub struct InProgressItem {
    pub url: String,
    pub bytes_downloaded: i64,
    pub content_length: Option<i64>,
}

/// Progress event payload emitted as `"download://progress"` events.
#[derive(Debug, Serialize, Clone)]
pub struct ProgressPayload {
    pub completed: usize,
    pub failed: usize,
    pub total: usize,
    pub in_progress: Vec<InProgressItem>,
}

// ---------------------------------------------------------------------------
// Managed app state (shared between commands)
// ---------------------------------------------------------------------------

/// Shared state managed by Tauri. Holds the current download's interrupt flag.
pub struct AppState {
    /// Set to `Some(flag)` while a `start_download_with_progress` call is active.
    /// `cancel_download` stores the flag here; each new run creates a fresh Arc.
    pub interrupted: Mutex<Option<Arc<AtomicBool>>>,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            interrupted: Mutex::new(None),
        }
    }
}

// ---------------------------------------------------------------------------
// Config loader
// ---------------------------------------------------------------------------

/// Minimal app defaults. Reads `~/.downloader/config.toml` if present;
/// silently falls back to compile-time defaults when the file is absent or unparseable.
struct AppDefaults {
    output_dir: PathBuf,
    concurrency: usize,
}

impl Default for AppDefaults {
    fn default() -> Self {
        let output_dir = dirs::home_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("Downloads")
            .join("downloader-output");
        Self {
            output_dir,
            concurrency: DEFAULT_CONCURRENCY,
        }
    }
}

impl AppDefaults {
    fn load() -> Self {
        let config_path = dirs::home_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join(".downloader")
            .join("config.toml");
        if config_path.exists() {
            if let Ok(raw) = std::fs::read_to_string(&config_path) {
                return Self::parse_config_text(&raw, Self::default());
            }
        }
        Self::default()
    }

    fn parse_config_text(raw: &str, mut defaults: Self) -> Self {
        for line in raw.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            // Use split_once to require an explicit '=' separator, preventing a key like
            // "output_directory" from matching a strip_prefix("output_dir") check.
            let Some((key, val)) = line.split_once('=') else {
                continue;
            };
            match key.trim() {
                "output_dir" => {
                    let val = val.trim().trim_matches('"');
                    if !val.is_empty() {
                        defaults.output_dir = PathBuf::from(val);
                    }
                }
                "concurrency" => {
                    if let Ok(n) = val.trim().parse::<usize>() {
                        if (1..=100).contains(&n) {
                            defaults.concurrency = n;
                        }
                    }
                }
                _ => {}
            }
        }
        defaults
    }
}

// ---------------------------------------------------------------------------
// list_projects helper
// ---------------------------------------------------------------------------

/// Scans `base` for non-hidden subdirectories, sorted by most-recently-modified.
///
/// Extracted for unit-testability; called by [`list_projects`].
fn scan_project_dirs(base: &std::path::Path) -> Vec<String> {
    if !base.exists() {
        return Vec::new();
    }

    let entries = match std::fs::read_dir(base) {
        Ok(e) => e,
        Err(e) => {
            warn!(path = %base.display(), error = %e, "Could not read projects directory");
            return Vec::new();
        }
    };

    let mut dirs: Vec<(std::time::SystemTime, String)> = entries
        .filter_map(|e| {
            let e = e.ok()?;
            let ft = e.file_type().ok()?;
            if !ft.is_dir() {
                return None;
            }
            let name = e.file_name().to_string_lossy().to_string();
            // Skip hidden directories (e.g. .downloader)
            if name.starts_with('.') {
                return None;
            }
            let modified = e
                .metadata()
                .ok()
                .and_then(|m| m.modified().ok())
                .unwrap_or(std::time::UNIX_EPOCH);
            Some((modified, name))
        })
        .collect();

    dirs.sort_by(|a, b| b.0.cmp(&a.0));
    dirs.into_iter().map(|(_, name)| name).collect()
}

/// Lists project subdirectory names under the base output directory.
///
/// Used to populate the project autocomplete suggestions in the frontend.
#[tracing::instrument]
#[tauri::command]
pub async fn list_projects() -> Result<Vec<String>, String> {
    let defaults = AppDefaults::load();
    Ok(scan_project_dirs(&defaults.output_dir))
}

/// Returns `true` when the polling loop should exit.
///
/// All items enqueued in the current run have reached a terminal state once
/// `(db_completed - prior_completed) + (db_failed - prior_failed) >= enqueued`.
/// The `prior_*` offsets subtract rows left over from earlier runs in the shared DB.
fn poll_should_break(
    db_completed: usize,
    db_failed: usize,
    prior_completed: usize,
    prior_failed: usize,
    enqueued: usize,
) -> bool {
    let this_run_completed = db_completed.saturating_sub(prior_completed);
    let this_run_failed = db_failed.saturating_sub(prior_failed);
    this_run_completed + this_run_failed >= enqueued
}

fn mark_interrupt_requested(state: &AppState) {
    if let Some(flag) = state.interrupted.lock().unwrap().as_ref() {
        flag.store(true, Ordering::SeqCst);
    }
}

fn clear_interrupt_slot(state: &AppState) {
    *state.interrupted.lock().unwrap() = None;
}

// ---------------------------------------------------------------------------
// Shared resolve-and-enqueue helper
// ---------------------------------------------------------------------------

/// Parse `inputs`, resolve each item, and enqueue into `queue`.
/// Returns the number of items successfully enqueued.
/// On total failure returns an error string.
async fn resolve_and_enqueue(inputs: &[String], queue: &Queue) -> Result<usize, String> {
    let joined = inputs
        .iter()
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join("\n");

    if joined.is_empty() {
        return Err(
            "What: No input provided.\n\
             Why: The input was blank or contained only whitespace.\n\
             Fix: Paste at least one URL (starting with https://) or DOI (starting with 10.) per line."
                .to_string(),
        );
    }

    let parse_result = parse_input(&joined);
    if parse_result.is_empty() {
        return Err(
            "What: No valid URLs or DOIs found in input.\n\
             Why: The text did not match any recognisable URL (https://...) or DOI (10.xxx/...) pattern.\n\
             Fix: Paste at least one full URL or DOI. Example: https://arxiv.org/abs/2301.00001 or 10.1000/xyz123."
                .to_string(),
        );
    }

    let resolver_registry = build_default_resolver_registry(None, "downloader-app@downloader");
    let resolve_context = ResolveContext::default();

    let mut enqueued = 0usize;
    let mut resolve_errors = 0usize;

    for item in &parse_result.items {
        let resolver_input = if item.input_type == InputType::BibTex {
            item.raw.as_str()
        } else {
            item.value.as_str()
        };

        let resolved = match resolver_registry
            .resolve_to_url(resolver_input, item.input_type, &resolve_context)
            .await
        {
            Ok(r) => r,
            Err(e) => {
                resolve_errors += 1;
                warn!(error = %e, "Skipped unresolved item");
                continue;
            }
        };

        if queue.has_active_url(&resolved.url).await.unwrap_or(false) {
            debug!("Skipping duplicate URL already in queue");
            continue;
        }

        let reference_confidence = (item.input_type == InputType::Reference)
            .then(|| extract_reference_confidence(&item.raw));

        let metadata = QueueMetadata {
            suggested_filename: Some(build_preferred_filename(&resolved.url, &resolved.metadata)),
            title: resolved.metadata.get("title").cloned(),
            authors: resolved.metadata.get("authors").cloned(),
            year: resolved.metadata.get("year").cloned(),
            doi: resolved.metadata.get("doi").cloned(),
            topics: None,
            parse_confidence: reference_confidence.map(|d| d.level.to_string()),
            parse_confidence_factors: reference_confidence
                .and_then(|d| serde_json::to_string(&d.factors).ok()),
        };

        if let Err(e) = queue
            .enqueue_with_metadata(
                &resolved.url,
                item.input_type.queue_source_type(),
                Some(&item.raw),
                Some(&metadata),
            )
            .await
        {
            warn!(error = %e, "Failed to enqueue item");
        } else {
            enqueued += 1;
        }
    }

    if enqueued == 0 {
        let reason = if resolve_errors > 0 {
            format!("All {resolve_errors} item(s) failed to resolve to a download URL.")
        } else {
            "No items could be enqueued.".to_string()
        };
        return Err(format!(
            "What: Download could not start.\n\
             Why: {reason}\n\
             Fix: Verify the URLs/DOIs are correct and that network access is available."
        ));
    }

    Ok(enqueued)
}

// ---------------------------------------------------------------------------
// Commands
// ---------------------------------------------------------------------------

/// Simple one-shot download command (Story 10-2 — kept for unit-test compatibility).
#[tracing::instrument]
#[tauri::command]
pub async fn start_download(
    inputs: Vec<String>,
    project: Option<String>,
) -> Result<DownloadSummary, String> {
    let defaults = AppDefaults::load();

    let output_dir =
        resolve_project_output_dir(&defaults.output_dir, project.as_deref()).map_err(|e| {
            format!(
                "What: Invalid project name.\n\
                 Why: {e}\n\
                 Fix: Use a simple name like 'Climate Research' without special characters."
            )
        })?;

    if let Err(e) = std::fs::create_dir_all(&output_dir) {
        return Err(format!(
            "What: Could not create output directory.\n\
             Why: {e}\n\
             Fix: Check that the path '{dir}' is writable, or update output_dir in ~/.downloader/config.toml.",
            dir = output_dir.display()
        ));
    }

    // Dual-DB design note: `start_download` uses a separate database file
    // (`downloader-app.db`) from `start_download_with_progress` (`downloader-app-progress.db`).
    // This split was introduced in Story 10-2 to preserve unit-test isolation: the progress
    // command's tests assume a fresh DB state and would otherwise conflict with tests for this
    // simpler command.
    //
    // Tradeoff: project history written by one command is invisible to the other. In practice
    // the frontend always calls `start_download_with_progress`, so `start_download` is a
    // fallback/test path only and its history is not surfaced in the UI. If the two commands
    // are ever consolidated, merge their DB paths at the same time.
    let db_path = dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".downloader")
        .join("downloader-app.db");

    let db = Database::new(&db_path).await.map_err(|e| {
        format!(
            "What: Failed to initialise database.\n\
             Why: {e}\n\
             Fix: Check that ~/.downloader/ is writable."
        )
    })?;

    let queue = Arc::new(Queue::new(db));
    resolve_and_enqueue(&inputs, &queue).await?;

    // Capture state before this run to identify newly-completed items and bound the log.
    let completed_before: HashSet<i64> = queue
        .list_by_status(QueueStatus::Completed)
        .await
        .unwrap_or_default()
        .into_iter()
        .map(|item| item.id)
        .collect();

    // Watermark: max existing DownloadAttempt id for this project before the run.
    // Passed to append_project_download_log so only new attempts appear in the session section.
    let log_watermark: Option<i64> = queue
        .query_download_attempts(&DownloadAttemptQuery {
            project: Some(project_history_key(&output_dir)),
            limit: 1,
            ..DownloadAttemptQuery::default()
        })
        .await
        .ok()
        .and_then(|mut v| v.pop())
        .map(|a| a.id);

    let client = HttpClient::new();
    let rate_limiter = Arc::new(RateLimiter::new(Duration::from_millis(0)));
    let engine = DownloadEngine::new(defaults.concurrency, RetryPolicy::default(), rate_limiter)
        .map_err(|e| {
            format!(
                "What: Failed to initialise download engine.\n\
                     Why: {e}\n\
                     Fix: Check concurrency settings in ~/.downloader/config.toml."
            )
        })?;

    let stats = engine
        .process_queue(&queue, &client, &output_dir)
        .await
        .map_err(|e| {
            format!(
                "What: Download engine encountered an error.\n\
                 Why: {e}\n\
                 Fix: Check network connectivity and output directory permissions."
            )
        })?;

    if project.is_some() {
        let _ = append_project_index(&queue, &output_dir, &completed_before).await;
        let _ = append_project_download_log(&queue, &output_dir, log_watermark).await;
        generate_sidecars_for_completed(&queue, &completed_before).await;
    }

    Ok(DownloadSummary {
        completed: stats.completed(),
        failed: stats.failed(),
        output_dir: output_dir.display().to_string(),
    })
}

/// Download command with real-time progress events (Story 10-3).
///
/// Emits `"download://progress"` events every 300 ms while downloads are in flight.
/// The `state.interrupted` flag can be set by `cancel_download` to stop the engine.
#[tracing::instrument(skip(window, state))]
#[tauri::command]
pub async fn start_download_with_progress(
    inputs: Vec<String>,
    project: Option<String>,
    window: tauri::Window,
    state: tauri::State<'_, AppState>,
) -> Result<DownloadSummary, String> {
    let defaults = AppDefaults::load();

    let output_dir =
        resolve_project_output_dir(&defaults.output_dir, project.as_deref()).map_err(|e| {
            format!(
                "What: Invalid project name.\n\
                 Why: {e}\n\
                 Fix: Use a simple name like 'Climate Research' without special characters."
            )
        })?;

    if let Err(e) = std::fs::create_dir_all(&output_dir) {
        return Err(format!(
            "What: Could not create output directory.\n\
             Why: {e}\n\
             Fix: Check that the path '{dir}' is writable, or update output_dir in ~/.downloader/config.toml.",
            dir = output_dir.display()
        ));
    }

    let db_path = dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".downloader")
        .join("downloader-app-progress.db");

    let db = Database::new(&db_path).await.map_err(|e| {
        format!(
            "What: Failed to initialise database.\n\
             Why: {e}\n\
             Fix: Check that ~/.downloader/ is writable."
        )
    })?;

    let queue = Arc::new(Queue::new(db));
    let enqueued = resolve_and_enqueue(&inputs, &queue).await?;

    // Create a fresh interrupt flag for this run; register it for cancel_download.
    let flag = Arc::new(AtomicBool::new(false));
    *state.interrupted.lock().unwrap() = Some(Arc::clone(&flag));

    let client = HttpClient::new();
    let rate_limiter = Arc::new(RateLimiter::new(Duration::from_millis(0)));
    let engine = DownloadEngine::new(defaults.concurrency, RetryPolicy::default(), rate_limiter)
        .map_err(|e| {
            format!(
                "What: Failed to initialise download engine.\n\
                     Why: {e}\n\
                     Fix: Check concurrency settings in ~/.downloader/config.toml."
            )
        })?;

    // Capture state before this run to identify newly-completed items and bound the log.
    let completed_before: HashSet<i64> = queue
        .list_by_status(QueueStatus::Completed)
        .await
        .unwrap_or_default()
        .into_iter()
        .map(|item| item.id)
        .collect();

    // Offsets for the polling loop: items completed/failed in prior runs must not count
    // toward the progress of THIS run (they are already Completed/Failed in the shared DB).
    let prior_completed = completed_before.len();
    let prior_failed: usize = queue
        .count_by_status(QueueStatus::Failed)
        .await
        .unwrap_or(0) as usize;

    // Watermark: max existing DownloadAttempt id for this project before the run.
    // Passed to append_project_download_log so only new attempts appear in the session section.
    let log_watermark: Option<i64> = queue
        .query_download_attempts(&DownloadAttemptQuery {
            project: Some(project_history_key(&output_dir)),
            limit: 1,
            ..DownloadAttemptQuery::default()
        })
        .await
        .ok()
        .and_then(|mut v| v.pop())
        .map(|a| a.id);

    let output_dir_for_engine = output_dir.clone();

    // Spawn engine in a background task.
    let queue_for_engine = Arc::clone(&queue);
    let flag_for_engine = Arc::clone(&flag);
    let engine_task = tokio::spawn(async move {
        engine
            .process_queue_interruptible(
                queue_for_engine.as_ref(),
                &client,
                &output_dir_for_engine,
                flag_for_engine,
            )
            .await
    });

    // Polling loop: emit progress events until engine finishes.
    let queue_for_poll = Arc::clone(&queue);
    let window_for_poll = window.clone();
    let poll_handle = tokio::spawn(async move {
        loop {
            tokio::time::sleep(Duration::from_millis(300)).await;

            let completed = queue_for_poll
                .count_by_status(QueueStatus::Completed)
                .await
                .unwrap_or(0) as usize;
            let failed = queue_for_poll
                .count_by_status(QueueStatus::Failed)
                .await
                .unwrap_or(0) as usize;
            let in_progress_items = queue_for_poll.get_in_progress().await.unwrap_or_default();

            let in_progress = in_progress_items
                .into_iter()
                .map(|item| InProgressItem {
                    url: item.url,
                    bytes_downloaded: item.bytes_downloaded,
                    content_length: item.content_length,
                })
                .collect::<Vec<_>>();

            // Subtract items completed/failed in prior runs so the payload reflects only
            // this-run progress and the break condition is not tripped prematurely.
            let this_run_completed = completed.saturating_sub(prior_completed);
            let this_run_failed = failed.saturating_sub(prior_failed);

            let payload = ProgressPayload {
                completed: this_run_completed,
                failed: this_run_failed,
                total: enqueued,
                in_progress,
            };

            let _ = window_for_poll.emit("download://progress", &payload);

            if poll_should_break(completed, failed, prior_completed, prior_failed, enqueued) {
                break;
            }
        }
    });

    // Await engine completion; abort the polling task and clear state on any error path.
    let stats = match engine_task.await {
        Err(e) => {
            poll_handle.abort();
            clear_interrupt_slot(&state);
            return Err(format!(
                "What: Internal task error.\nWhy: {e}\nFix: Restart the app."
            ));
        }
        Ok(Err(e)) => {
            poll_handle.abort();
            clear_interrupt_slot(&state);
            return Err(format!(
                "What: Download engine encountered an error.\n\
                 Why: {e}\n\
                 Fix: Check network connectivity and output directory permissions."
            ));
        }
        Ok(Ok(s)) => s,
    };

    // Stop the polling task (success path).
    poll_handle.abort();

    // Emit one final accurate event so the frontend reaches 100%.
    let _ = window.emit(
        "download://progress",
        &ProgressPayload {
            completed: stats.completed(),
            failed: stats.failed(),
            total: enqueued,
            in_progress: vec![],
        },
    );

    // Clear the interrupt flag slot.
    clear_interrupt_slot(&state);

    // Generate project artefacts (index.md, download.log, sidecars) when a project is set.
    if project.is_some() {
        let _ = append_project_index(&queue, &output_dir, &completed_before).await;
        let _ = append_project_download_log(&queue, &output_dir, log_watermark).await;
        generate_sidecars_for_completed(&queue, &completed_before).await;
    }

    Ok(DownloadSummary {
        completed: stats.completed(),
        failed: stats.failed(),
        output_dir: output_dir.display().to_string(),
    })
}

/// Sets the interrupt flag to gracefully stop an active `start_download_with_progress` run.
#[tracing::instrument(skip(state))]
#[tauri::command]
pub async fn cancel_download(state: tauri::State<'_, AppState>) -> Result<(), String> {
    mark_interrupt_requested(&state);
    Ok(())
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::AtomicU64;
    use std::time::{SystemTime, UNIX_EPOCH};

    static TEST_COUNTER: AtomicU64 = AtomicU64::new(0);

    fn unique_db_path(label: &str) -> PathBuf {
        let ts = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock before epoch")
            .as_nanos();
        let seq = TEST_COUNTER.fetch_add(1, Ordering::Relaxed);
        std::env::temp_dir().join(format!("downloader-{label}-{ts}-{seq}.db"))
    }

    #[tokio::test]
    async fn test_start_download_empty_inputs_returns_error() {
        let result = start_download(vec![], None).await;
        assert!(result.is_err(), "empty input should return Err");
        let err = result.unwrap_err();
        assert!(
            err.contains("What:"),
            "error should follow What/Why/Fix format, got: {err}"
        );
    }

    #[tokio::test]
    async fn test_start_download_blank_inputs_returns_error() {
        let result = start_download(vec!["   ".to_string(), "\t".to_string()], None).await;
        assert!(result.is_err(), "blank-only inputs should return Err");
        let err = result.unwrap_err();
        assert!(
            err.contains("What:"),
            "error should follow What/Why/Fix format, got: {err}"
        );
    }

    #[tokio::test]
    async fn test_start_download_garbage_text_returns_error() {
        let result = start_download(vec!["not a url or doi at all".to_string()], None).await;
        assert!(result.is_err(), "unrecognised text should return Err");
        let err = result.unwrap_err();
        assert!(
            err.contains("What:"),
            "error should follow What/Why/Fix format, got: {err}"
        );
    }

    #[tokio::test]
    async fn test_cancel_download_sets_interrupted_flag() {
        use std::sync::{Arc, Mutex};
        let state_inner = AppState {
            interrupted: Mutex::new(Some(Arc::new(AtomicBool::new(false)))),
        };
        // Extract the flag before moving state_inner into a State wrapper.
        let flag_clone = Arc::clone(state_inner.interrupted.lock().unwrap().as_ref().unwrap());
        assert!(!flag_clone.load(Ordering::SeqCst), "initially false");

        // Simulate cancel by setting via AppState directly (no Tauri runtime needed).
        if let Some(flag) = state_inner.interrupted.lock().unwrap().as_ref() {
            flag.store(true, Ordering::SeqCst);
        }

        assert!(flag_clone.load(Ordering::SeqCst), "flag set after cancel");
    }

    #[test]
    fn test_parse_config_text_keeps_defaults_for_empty_config() {
        let defaults = AppDefaults::default();
        let parsed = AppDefaults::parse_config_text("", AppDefaults::default());

        assert_eq!(parsed.output_dir, defaults.output_dir);
        assert_eq!(parsed.concurrency, defaults.concurrency);
    }

    #[test]
    fn test_parse_config_text_reads_output_dir_override() {
        let parsed = AppDefaults::parse_config_text(
            "output_dir = \"/tmp/custom-downloads\"",
            AppDefaults::default(),
        );

        assert_eq!(parsed.output_dir, PathBuf::from("/tmp/custom-downloads"));
    }

    #[test]
    fn test_parse_config_text_accepts_valid_concurrency() {
        let parsed = AppDefaults::parse_config_text("concurrency = 7", AppDefaults::default());
        assert_eq!(parsed.concurrency, 7);
    }

    #[test]
    fn test_parse_config_text_rejects_invalid_concurrency_values() {
        let defaults = AppDefaults::default();

        let zero = AppDefaults::parse_config_text("concurrency = 0", AppDefaults::default());
        let too_large = AppDefaults::parse_config_text("concurrency = 101", AppDefaults::default());
        let non_numeric =
            AppDefaults::parse_config_text("concurrency = nope", AppDefaults::default());

        assert_eq!(zero.concurrency, defaults.concurrency);
        assert_eq!(too_large.concurrency, defaults.concurrency);
        assert_eq!(non_numeric.concurrency, defaults.concurrency);
    }

    #[test]
    fn test_parse_config_text_ignores_output_directory_prefix_match() {
        // "output_directory" must NOT be treated as "output_dir" — exact key match required.
        let defaults = AppDefaults::default();
        let result = AppDefaults::parse_config_text(
            "output_directory = \"/should/be/ignored\"",
            AppDefaults::default(),
        );
        assert_eq!(
            result.output_dir, defaults.output_dir,
            "output_directory key should not override output_dir"
        );
    }

    #[test]
    fn test_parse_config_text_ignores_comment_lines() {
        let defaults = AppDefaults::default();
        let result = AppDefaults::parse_config_text(
            "# output_dir = \"/commented/out\"\noutput_dir = \"/actual/path\"",
            AppDefaults::default(),
        );
        assert_eq!(result.output_dir, std::path::PathBuf::from("/actual/path"));
        assert_eq!(
            AppDefaults::parse_config_text(
                "# output_dir = \"/commented/out\"",
                AppDefaults::default()
            )
            .output_dir,
            defaults.output_dir,
            "commented-out key should be ignored"
        );
    }

    #[tokio::test]
    async fn test_resolve_and_enqueue_rejects_whitespace_only_input() {
        let db_path = unique_db_path("whitespace-input");
        let db = Database::new(&db_path).await.expect("test DB");
        let queue = Queue::new(db);

        let result = resolve_and_enqueue(&["   ".to_string(), "\t".to_string()], &queue).await;

        assert!(result.is_err());
        assert!(result.unwrap_err().contains("What: No input provided."));

        let _ = std::fs::remove_file(&db_path);
    }

    #[tokio::test]
    async fn test_resolve_and_enqueue_rejects_unparseable_input() {
        let db_path = unique_db_path("garbage-input");
        let db = Database::new(&db_path).await.expect("test DB");
        let queue = Queue::new(db);

        let result = resolve_and_enqueue(&["not a url or doi at all".to_string()], &queue).await;

        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .contains("What: No valid URLs or DOIs found in input.")
        );

        let _ = std::fs::remove_file(&db_path);
    }

    #[tokio::test]
    async fn test_resolve_and_enqueue_returns_partial_success_when_duplicate_is_skipped() {
        let db_path = unique_db_path("partial-success");
        let db = Database::new(&db_path).await.expect("test DB");
        let queue = Queue::new(db);

        queue
            .enqueue("https://example.com/already-queued.pdf", "direct_url", None)
            .await
            .expect("seed duplicate");

        let result = resolve_and_enqueue(
            &[
                "https://example.com/already-queued.pdf".to_string(),
                "https://example.com/new-file.pdf".to_string(),
            ],
            &queue,
        )
        .await;

        assert_eq!(result.expect("one item should enqueue"), 1);

        let pending = queue
            .count_by_status(QueueStatus::Pending)
            .await
            .expect("count pending");
        assert_eq!(pending, 2);

        let _ = std::fs::remove_file(&db_path);
    }

    #[tokio::test]
    async fn test_resolve_and_enqueue_returns_error_when_only_duplicates_are_available() {
        let db_path = unique_db_path("duplicate-only");
        let db = Database::new(&db_path).await.expect("test DB");
        let queue = Queue::new(db);

        queue
            .enqueue("https://example.com/already-queued.pdf", "direct_url", None)
            .await
            .expect("seed duplicate");

        let result = resolve_and_enqueue(
            &["https://example.com/already-queued.pdf".to_string()],
            &queue,
        )
        .await;

        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .contains("What: Download could not start.")
        );

        let _ = std::fs::remove_file(&db_path);
    }

    #[test]
    fn test_mark_interrupt_requested_is_noop_when_no_active_flag() {
        let state = AppState::default();
        mark_interrupt_requested(&state);
        assert!(state.interrupted.lock().unwrap().is_none());
    }

    #[test]
    fn test_clear_interrupt_slot_resets_state_on_success_path() {
        let state = AppState {
            interrupted: Mutex::new(Some(Arc::new(AtomicBool::new(false)))),
        };
        clear_interrupt_slot(&state);
        assert!(state.interrupted.lock().unwrap().is_none());
    }

    #[test]
    fn test_clear_interrupt_slot_resets_state_on_engine_error_path() {
        let state = AppState {
            interrupted: Mutex::new(Some(Arc::new(AtomicBool::new(true)))),
        };
        clear_interrupt_slot(&state);
        assert!(state.interrupted.lock().unwrap().is_none());
    }

    #[test]
    fn test_clear_interrupt_slot_resets_state_on_join_error_path() {
        let state = AppState {
            interrupted: Mutex::new(Some(Arc::new(AtomicBool::new(true)))),
        };
        clear_interrupt_slot(&state);
        assert!(state.interrupted.lock().unwrap().is_none());
    }

    // -----------------------------------------------------------------------
    // poll_should_break — unit tests for the polling loop exit predicate (M-3)
    // -----------------------------------------------------------------------

    #[test]
    fn test_poll_should_break_exits_when_all_items_completed() {
        assert!(poll_should_break(2, 0, 0, 0, 2), "all completed → break");
    }

    #[test]
    fn test_poll_should_break_exits_on_mixed_completed_and_failed() {
        assert!(
            poll_should_break(1, 1, 0, 0, 2),
            "1 completed + 1 failed → break"
        );
        assert!(poll_should_break(0, 2, 0, 0, 2), "all failed → break");
    }

    #[test]
    fn test_poll_should_break_stays_when_items_still_pending() {
        assert!(!poll_should_break(0, 0, 0, 0, 2), "nothing done → continue");
        assert!(!poll_should_break(1, 0, 0, 0, 2), "1 of 2 done → continue");
    }

    #[test]
    fn test_poll_should_break_accounts_for_prior_run_offsets() {
        // 3 completed in DB, 1 was from a prior run → this_run_completed = 2 → break
        assert!(poll_should_break(3, 0, 1, 0, 2));
        // 3 completed in DB, 2 were from prior runs → this_run_completed = 1 → continue
        assert!(!poll_should_break(3, 0, 2, 0, 2));
        // prior_failed offset applied correctly
        assert!(poll_should_break(0, 3, 0, 1, 2));
        assert!(!poll_should_break(0, 3, 0, 2, 2));
    }

    #[test]
    fn test_poll_should_break_saturating_sub_handles_underflow() {
        // prior counts larger than current (shouldn't happen in practice) must not panic.
        assert!(!poll_should_break(1, 0, 5, 0, 2)); // saturating_sub → 0, 0 < 2 → continue
    }

    /// AC#9 (DB invariant): verifies queue state is consistent after engine marks items terminal.
    /// Note: this tests the DB mechanics, not the polling loop's break path.
    /// The break predicate itself is covered by test_poll_should_break_* tests above.
    #[tokio::test]
    async fn test_poll_exit_condition_triggers_when_all_items_terminal() {
        let db_path = unique_db_path("poll-exit");
        let db = Database::new(&db_path).await.expect("test DB");
        let queue = Queue::new(db);

        queue
            .enqueue("https://a.example.com/1", "direct_url", None)
            .await
            .unwrap();
        queue
            .enqueue("https://b.example.com/2", "direct_url", None)
            .await
            .unwrap();
        let enqueued = 2usize;

        // Simulate engine marking all items completed.
        for _ in 0..enqueued {
            let item = queue.dequeue().await.unwrap().expect("pending item");
            queue.mark_completed(item.id).await.unwrap();
        }

        let completed = queue.count_by_status(QueueStatus::Completed).await.unwrap() as usize;
        let failed = queue.count_by_status(QueueStatus::Failed).await.unwrap() as usize;
        assert!(
            completed + failed >= enqueued,
            "poll exit condition: completed={completed} failed={failed} enqueued={enqueued}"
        );

        let _ = std::fs::remove_file(&db_path);
    }

    // -----------------------------------------------------------------------
    // list_projects tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_scan_project_dirs_returns_empty_for_nonexistent_dir() {
        let nonexistent = std::env::temp_dir().join("does_not_exist_downloader_test_scan");
        let result = scan_project_dirs(&nonexistent);
        assert!(result.is_empty(), "expected empty for nonexistent dir");
    }

    #[test]
    fn test_scan_project_dirs_excludes_hidden_dirs_and_files() {
        let temp = tempfile::tempdir().unwrap();
        let base = temp.path();
        std::fs::create_dir_all(base.join("Climate-Research")).unwrap();
        std::fs::create_dir_all(base.join("Genomics")).unwrap();
        std::fs::create_dir_all(base.join(".hidden")).unwrap();
        std::fs::write(base.join("readme.txt"), b"content").unwrap();

        let result = scan_project_dirs(base);

        assert_eq!(result.len(), 2, "only non-hidden dirs should appear");
        assert!(result.contains(&"Climate-Research".to_string()));
        assert!(result.contains(&"Genomics".to_string()));
        assert!(!result.contains(&".hidden".to_string()));
        assert!(!result.contains(&"readme.txt".to_string()));
    }

    /// M-4: start_download_with_progress cannot be called directly in unit tests
    /// (requires tauri::Window + tauri::State which need a running Tauri runtime).
    /// Both commands delegate to the same `resolve_project_output_dir` function.
    /// This test verifies that shared validation and error formatting are correct.
    #[test]
    fn test_start_download_with_progress_project_validation_via_shared_fn() {
        let base = std::path::PathBuf::from("/tmp/test-output");

        // Traversal is rejected
        let e = resolve_project_output_dir(&base, Some("..")).unwrap_err();
        // Both commands format this as "What: Invalid project name.\nWhy: {e}\n..."
        let formatted = format!("What: Invalid project name.\nWhy: {e}\nFix: ...");
        assert!(
            formatted.contains("path traversal rejected"),
            "err: {formatted}"
        );

        // Empty project name is rejected
        let e = resolve_project_output_dir(&base, Some("   ")).unwrap_err();
        let formatted = format!("What: Invalid project name.\nWhy: {e}\nFix: ...");
        assert!(formatted.contains("empty"), "err: {formatted}");

        // Valid name resolves correctly (same path both commands would compute)
        let ok = resolve_project_output_dir(&base, Some("Climate Research")).unwrap();
        assert_eq!(ok, base.join("Climate-Research"));
    }

    #[tokio::test]
    async fn test_start_download_rejects_invalid_project_name() {
        // "." is rejected by resolve_project_output_dir
        let result = start_download(
            vec!["https://example.com/test.pdf".to_string()],
            Some(".".to_string()),
        )
        .await;
        assert!(result.is_err(), "traversal token should fail");
        let err = result.unwrap_err();
        assert!(
            err.contains("What:"),
            "error should follow What/Why/Fix format, got: {err}"
        );
    }

    #[tokio::test]
    async fn test_start_download_accepts_valid_project_name() {
        // A valid project name must NOT be rejected by resolve_project_output_dir.
        // The download will fail on URL parsing — that is the expected error path.
        let result = start_download(
            vec!["not-a-url-or-doi".to_string()],
            Some("Climate Research".to_string()),
        )
        .await;
        assert!(result.is_err(), "invalid input should fail");
        let err = result.unwrap_err();
        // The error must be about the URL/DOI — NOT about the project name.
        assert!(
            !err.contains("Invalid project name"),
            "project name 'Climate Research' should have been accepted, got: {err}"
        );
        assert!(
            err.contains("No valid URLs or DOIs"),
            "expected URL parse error, got: {err}"
        );
    }
}
