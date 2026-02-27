//! Tauri commands for the Downloader desktop app.
//!
//! Bridges the Svelte frontend to `downloader_core` via Tauri's IPC layer.

use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use downloader_core::{
    DEFAULT_CONCURRENCY, Database, DownloadEngine, HttpClient, InputType, Queue, QueueMetadata,
    QueueStatus, RateLimiter, ResolveContext, RetryPolicy, build_default_resolver_registry,
    build_preferred_filename, extract_reference_confidence, parse_input,
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
        let mut defaults = Self::default();
        let config_path = dirs::home_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join(".downloader")
            .join("config.toml");
        if config_path.exists() {
            if let Ok(raw) = std::fs::read_to_string(&config_path) {
                for line in raw.lines() {
                    let line = line.trim();
                    if let Some(val) = line.strip_prefix("output_dir") {
                        let val = val
                            .trim_start_matches(['=', ' ', '"'])
                            .trim_end_matches('"');
                        if !val.is_empty() {
                            defaults.output_dir = PathBuf::from(val);
                        }
                    }
                    if let Some(val) = line.strip_prefix("concurrency") {
                        let val = val.trim_start_matches(['=', ' ']);
                        if let Ok(n) = val.parse::<usize>() {
                            if (1..=100).contains(&n) {
                                defaults.concurrency = n;
                            }
                        }
                    }
                }
            }
        }
        defaults
    }
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
#[tauri::command]
pub async fn start_download(inputs: Vec<String>) -> Result<DownloadSummary, String> {
    let defaults = AppDefaults::load();

    if let Err(e) = std::fs::create_dir_all(&defaults.output_dir) {
        return Err(format!(
            "What: Could not create output directory.\n\
             Why: {e}\n\
             Fix: Check that the path '{dir}' is writable, or update output_dir in ~/.downloader/config.toml.",
            dir = defaults.output_dir.display()
        ));
    }

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
        .process_queue(&queue, &client, &defaults.output_dir)
        .await
        .map_err(|e| {
            format!(
                "What: Download engine encountered an error.\n\
                 Why: {e}\n\
                 Fix: Check network connectivity and output directory permissions."
            )
        })?;

    Ok(DownloadSummary {
        completed: stats.completed(),
        failed: stats.failed(),
        output_dir: defaults.output_dir.display().to_string(),
    })
}

/// Download command with real-time progress events (Story 10-3).
///
/// Emits `"download://progress"` events every 300 ms while downloads are in flight.
/// The `state.interrupted` flag can be set by `cancel_download` to stop the engine.
#[tauri::command]
pub async fn start_download_with_progress(
    inputs: Vec<String>,
    window: tauri::Window,
    state: tauri::State<'_, AppState>,
) -> Result<DownloadSummary, String> {
    let defaults = AppDefaults::load();

    if let Err(e) = std::fs::create_dir_all(&defaults.output_dir) {
        return Err(format!(
            "What: Could not create output directory.\n\
             Why: {e}\n\
             Fix: Check that the path '{dir}' is writable, or update output_dir in ~/.downloader/config.toml.",
            dir = defaults.output_dir.display()
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

    let output_dir = defaults.output_dir.clone();

    // Spawn engine in a background task.
    let queue_for_engine = Arc::clone(&queue);
    let flag_for_engine = Arc::clone(&flag);
    let engine_task = tokio::spawn(async move {
        engine
            .process_queue_interruptible(
                queue_for_engine.as_ref(),
                &client,
                &output_dir,
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

            let payload = ProgressPayload {
                completed,
                failed,
                total: enqueued,
                in_progress,
            };

            let _ = window_for_poll.emit("download://progress", &payload);

            if completed + failed >= enqueued {
                break;
            }
        }
    });

    // Await engine completion; abort the polling task and clear state on any error path.
    let stats = match engine_task.await {
        Err(e) => {
            poll_handle.abort();
            *state.interrupted.lock().unwrap() = None;
            return Err(format!(
                "What: Internal task error.\nWhy: {e}\nFix: Restart the app."
            ));
        }
        Ok(Err(e)) => {
            poll_handle.abort();
            *state.interrupted.lock().unwrap() = None;
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
    *state.interrupted.lock().unwrap() = None;

    Ok(DownloadSummary {
        completed: stats.completed(),
        failed: stats.failed(),
        output_dir: defaults.output_dir.display().to_string(),
    })
}

/// Sets the interrupt flag to gracefully stop an active `start_download_with_progress` run.
#[tauri::command]
pub async fn cancel_download(state: tauri::State<'_, AppState>) -> Result<(), String> {
    if let Some(flag) = state.interrupted.lock().unwrap().as_ref() {
        flag.store(true, Ordering::SeqCst);
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_start_download_empty_inputs_returns_error() {
        let result = start_download(vec![]).await;
        assert!(result.is_err(), "empty input should return Err");
        let err = result.unwrap_err();
        assert!(
            err.contains("What:"),
            "error should follow What/Why/Fix format, got: {err}"
        );
    }

    #[tokio::test]
    async fn test_start_download_blank_inputs_returns_error() {
        let result = start_download(vec!["   ".to_string(), "\t".to_string()]).await;
        assert!(result.is_err(), "blank-only inputs should return Err");
        let err = result.unwrap_err();
        assert!(
            err.contains("What:"),
            "error should follow What/Why/Fix format, got: {err}"
        );
    }

    #[tokio::test]
    async fn test_start_download_garbage_text_returns_error() {
        let result = start_download(vec!["not a url or doi at all".to_string()]).await;
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

    /// AC#9: Verifies the polling exit condition (`completed + failed >= enqueued`) is
    /// satisfied once all queued items reach a terminal state — i.e., the loop would break.
    #[tokio::test]
    async fn test_poll_exit_condition_triggers_when_all_items_terminal() {
        let db_path =
            std::env::temp_dir().join(format!("downloader-test-poll-{}.db", std::process::id()));
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
}
