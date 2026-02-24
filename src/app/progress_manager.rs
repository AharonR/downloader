//! Progress UI (spinner) for download runs.

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use downloader_core::{Queue, QueueStatus};
use indicatif::{ProgressBar, ProgressStyle};
use url::Url;

/// Spawns the progress UI (spinner) when requested.
/// Returns (handle, stop) so the caller can signal stop and await the handle.
/// When `use_spinner` is false, returns (None, stop) with stop already true.
pub(crate) fn spawn_progress_ui(
    use_spinner: bool,
    queue: Arc<Queue>,
    total: usize,
) -> (Option<tokio::task::JoinHandle<()>>, Arc<AtomicBool>) {
    if !use_spinner {
        return (None, Arc::new(AtomicBool::new(true)));
    }
    let stop = Arc::new(AtomicBool::new(false));
    let handle = spawn_spinner_inner(queue, total, Arc::clone(&stop));
    (Some(handle), stop)
}

fn spawn_spinner_inner(
    queue: Arc<Queue>,
    total: usize,
    stop: Arc<AtomicBool>,
) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        let spinner = ProgressBar::new_spinner();
        spinner.set_style(
            ProgressStyle::with_template("{spinner} {msg}")
                .unwrap_or_else(|_| ProgressStyle::default_spinner()),
        );
        spinner.enable_steady_tick(Duration::from_millis(100));

        while !stop.load(Ordering::SeqCst) {
            let completed = queue
                .count_by_status(QueueStatus::Completed)
                .await
                .unwrap_or(0);
            let failed = queue
                .count_by_status(QueueStatus::Failed)
                .await
                .unwrap_or(0);
            let in_progress_items = queue.get_in_progress().await.unwrap_or_default();

            let done = usize::try_from(completed.saturating_add(failed)).unwrap_or(0);
            let current = if in_progress_items.is_empty() {
                done
            } else {
                done.saturating_add(1)
            };
            let domain = in_progress_items
                .first()
                .and_then(|item| Url::parse(&item.url).ok())
                .and_then(|url| url.host_str().map(std::string::ToString::to_string))
                .unwrap_or_else(|| "queue".to_string());

            spinner.set_message(format!(
                "[{}/{}] Downloading from {}...",
                current.min(total),
                total,
                domain
            ));
            tokio::time::sleep(Duration::from_millis(120)).await;
        }

        spinner.finish_and_clear();
    })
}

#[cfg(test)]
mod tests {
    use super::spawn_progress_ui;
    use downloader_core::{Database, Queue};
    use std::sync::Arc;
    use std::sync::atomic::Ordering;

    #[tokio::test]
    async fn spawn_progress_ui_when_disabled_returns_none_handle_and_stop_already_true() {
        let db = Database::new_in_memory().await.unwrap();
        let queue = Arc::new(Queue::new(db));

        let (handle, stop) = spawn_progress_ui(false, queue, 1);

        assert!(handle.is_none());
        assert!(
            stop.load(Ordering::SeqCst),
            "stop signal should be true when spinner disabled"
        );
    }

    #[tokio::test]
    async fn spawn_progress_ui_when_enabled_returns_handle_and_stop_and_stop_ends_task() {
        let db = Database::new_in_memory().await.unwrap();
        let queue = Arc::new(Queue::new(db));

        let (handle, stop) = spawn_progress_ui(true, queue, 1);

        assert!(
            handle.is_some(),
            "handle should be Some when spinner enabled"
        );
        assert!(
            !stop.load(Ordering::SeqCst),
            "stop should be false initially"
        );

        stop.store(true, Ordering::SeqCst);
        let join_handle = handle.unwrap();
        let _ = join_handle.await;
        // If we get here without hanging, the spinner task exited on stop signal
    }
}
