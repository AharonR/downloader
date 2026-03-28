use std::path::PathBuf;
use std::sync::Arc;
use std::time::Instant;

use tokio::task::JoinError;
use tracing::{info, warn};

use crate::queue::{Queue, QueueItem, QueueRepository};

use super::persistence::{persist_download_failure, persist_download_success};
use super::{DownloadStats, HttpClient, RateLimiter, RetryPolicy, download_with_retry};
use crate::{RobotsCache, RobotsDecision, origin_for_robots};

/// Per-task configuration extracted from [`super::QueueProcessingOptions`] and engine settings.
///
/// Groups the configuration arguments for [`process_download_item`] into a single struct,
/// keeping the function signature manageable as options grow.
pub(super) struct DownloadTaskConfig {
    pub retry_policy: RetryPolicy,
    pub rate_limiter: Arc<RateLimiter>,
    pub project_key: String,
    pub generate_sidecars: bool,
    pub check_robots: bool,
    pub robots_cache: Option<Arc<RobotsCache>>,
}

pub(super) async fn process_download_item(
    queue: Queue,
    client: HttpClient,
    item: QueueItem,
    output_dir: PathBuf,
    stats: Arc<DownloadStats>,
    config: DownloadTaskConfig,
) {
    let attempt_started = Instant::now();

    if config.check_robots {
        if let Some(ref cache) = config.robots_cache {
            if let Some(origin) = origin_for_robots(&item.url) {
                match cache.check_allowed(&item.url, &origin, &client).await {
                    Ok(RobotsDecision::Disallowed) => {
                        info!(url = %item.url, "skipping download: robots.txt disallows");
                        stats.increment_failed();
                        if let Err(e) = queue
                            .mark_failed(item.id, "robots.txt disallows this URL", 0)
                            .await
                        {
                            warn!(item_id = item.id, error = %e, "failed to mark robots-disallowed item");
                        }
                        return;
                    }
                    Ok(RobotsDecision::Allowed) => {}
                    Err(e) => {
                        warn!(url = %item.url, error = %e, "robots.txt check failed; proceeding with download");
                    }
                }
            }
        }
    }

    let result = download_with_retry(
        &queue,
        &client,
        &item,
        &output_dir,
        &config.retry_policy,
        &stats,
        &config.rate_limiter,
    )
    .await;

    match result {
        Ok(download) => {
            persist_download_success(
                &queue,
                &item,
                &download,
                &config.project_key,
                attempt_started,
                config.generate_sidecars,
                stats.as_ref(),
            )
            .await;
        }
        Err((error, attempts)) => {
            persist_download_failure(
                &queue,
                &item,
                &error,
                attempts,
                &config.project_key,
                attempt_started,
                stats.as_ref(),
            )
            .await;
        }
    }
}

pub(super) async fn handle_task_join_error(
    queue: &impl QueueRepository,
    item_id: i64,
    join_error: JoinError,
    stats: &DownloadStats,
) {
    warn!(
        item_id = item_id,
        error = %join_error,
        "download task panicked"
    );
    if let Err(queue_error) = queue
        .mark_failed(item_id, &format!("task panic: {join_error}"), 0)
        .await
    {
        warn!(
            item_id = item_id,
            error = %queue_error,
            "failed to mark panicked item as failed"
        );
    }
    stats.increment_failed();
}
