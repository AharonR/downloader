//! Build HTTP client and download engine, then process the queue.
//! Maps engine errors to anyhow with context for CI diagnostics.

use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use std::time::Duration;

use anyhow::{Context, Result};
use downloader_core::{
    DownloadEngine, HttpClient, Queue, QueueProcessingOptions, RateLimiter, RetryPolicy,
    RobotsCache,
};
use tracing::debug;

use crate::app::context::RunContext;

/// Builds client, engine, and options; runs queue processing. Returns download statistics.
pub(crate) async fn run_download(
    ctx: &RunContext,
    queue: Arc<Queue>,
    interrupted: Arc<AtomicBool>,
) -> Result<downloader_core::DownloadStats> {
    let client = if let Some(jar) = &ctx.cookie_jar {
        debug!("Creating HTTP client with cookie jar");
        HttpClient::with_cookie_jar_and_timeouts(
            jar.clone(),
            ctx.http_timeouts.download_connect_secs,
            ctx.http_timeouts.download_read_secs,
        )
    } else {
        HttpClient::new_with_timeouts(
            ctx.http_timeouts.download_connect_secs,
            ctx.http_timeouts.download_read_secs,
        )
    };

    let retry_policy = RetryPolicy::with_max_attempts(u32::from(ctx.args.max_retries));

    let rate_limiter = if ctx.args.rate_limit == 0 {
        debug!("rate limiting disabled");
        Arc::new(RateLimiter::disabled())
    } else if ctx.args.rate_limit_jitter > 0 {
        debug!(
            rate_limit_ms = ctx.args.rate_limit,
            jitter_ms = ctx.args.rate_limit_jitter,
            "rate limiting with jitter enabled"
        );
        Arc::new(RateLimiter::new_with_jitter(
            Duration::from_millis(ctx.args.rate_limit),
            ctx.args.rate_limit_jitter,
        ))
    } else {
        debug!(rate_limit_ms = ctx.args.rate_limit, "rate limiting enabled");
        Arc::new(RateLimiter::new(Duration::from_millis(ctx.args.rate_limit)))
    };

    let engine = DownloadEngine::new(
        usize::from(ctx.args.concurrency),
        retry_policy,
        rate_limiter,
    )
    .context("invalid download engine configuration")?;

    let robots_cache = if ctx.args.check_robots {
        Some(Arc::new(RobotsCache::new()))
    } else {
        None
    };

    let stats = engine
        .process_queue_interruptible_with_options(
            queue.as_ref(),
            &client,
            &ctx.output_dir,
            interrupted,
            QueueProcessingOptions {
                generate_sidecars: ctx.args.sidecar,
                check_robots: ctx.args.check_robots,
                robots_cache,
            },
        )
        .await
        .context("queue processing failed")?;

    Ok(stats)
}
