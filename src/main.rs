//! CLI entry point for the downloader tool.

use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use anyhow::Result;
use clap::Parser;
use downloader_core::{Database, DownloadEngine, HttpClient, Queue, RateLimiter, RetryPolicy};
use tracing::{debug, info};

mod cli;

use cli::Args;

#[tokio::main]
async fn main() -> Result<()> {
    // Parse CLI arguments first (before tracing, so --help works without logs)
    let args = Args::parse();

    // Determine log level based on verbose/quiet flags
    // Priority: RUST_LOG env var > quiet flag > verbose flag > default (info)
    let default_level = if args.quiet {
        "error"
    } else {
        match args.verbose {
            0 => "info",
            1 => "debug",
            _ => "trace",
        }
    };

    let filter = tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new(default_level));

    tracing_subscriber::fmt().with_env_filter(filter).init();

    debug!(?args, "CLI arguments parsed");
    info!("Downloader starting");

    // Initialize database and queue
    let db = Database::new_in_memory().await?;
    let queue = Queue::new(db);

    // Create HTTP client and download engine with retry policy and rate limiter
    let client = HttpClient::new();
    let retry_policy = RetryPolicy::with_max_attempts(args.max_retries as u32);

    // Create rate limiter based on CLI flag
    let rate_limiter = if args.rate_limit == 0 {
        debug!("rate limiting disabled");
        Arc::new(RateLimiter::disabled())
    } else {
        debug!(rate_limit_ms = args.rate_limit, "rate limiting enabled");
        Arc::new(RateLimiter::new(Duration::from_millis(args.rate_limit)))
    };

    let engine = DownloadEngine::new(args.concurrency as usize, retry_policy, rate_limiter)?;

    // Default output directory (current directory)
    let output_dir = PathBuf::from(".");

    // Process the download queue
    let stats = engine.process_queue(&queue, &client, &output_dir).await?;

    info!(
        completed = stats.completed(),
        failed = stats.failed(),
        retried = stats.retried(),
        total = stats.total(),
        "Download complete"
    );

    Ok(())
}
