//! CLI entry point for the downloader tool.

use std::io::{self, IsTerminal, Read};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use anyhow::Result;
use clap::Parser;
use downloader_core::{
    Database, DownloadEngine, HttpClient, Queue, RateLimiter, RetryPolicy, parse_input,
};
use tracing::{debug, info, warn};

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

    // Read input: from positional args or stdin
    let input_text = if !args.urls.is_empty() {
        args.urls.join("\n")
    } else if !io::stdin().is_terminal() {
        let mut buffer = String::new();
        io::stdin().read_to_string(&mut buffer)?;
        buffer
    } else {
        info!("No input provided. Pipe URLs via stdin or pass as arguments.");
        info!("Example: echo 'https://example.com/file.pdf' | downloader");
        return Ok(());
    };

    // Parse input to extract URLs
    let parse_result = parse_input(&input_text);

    if parse_result.is_empty() {
        info!("No valid URLs found in input");
        return Ok(());
    }

    info!(
        urls = parse_result.len(),
        skipped = parse_result.skipped_count(),
        "Parsed input"
    );

    for skipped in &parse_result.skipped {
        warn!(skipped = %skipped, "Skipped unrecognized input");
    }

    // Initialize database and queue
    // In-memory database is sufficient for one-shot pipeline mode.
    // File-based persistence (Database::new(path)) is available for future use.
    let db = Database::new_in_memory().await?;
    let queue = Queue::new(db);

    // Enqueue all parsed URLs
    for item in &parse_result.items {
        queue
            .enqueue(&item.value, "direct_url", Some(&item.raw))
            .await?;
        debug!(url = %item.value, "Enqueued URL");
    }

    // Create HTTP client and download engine with retry policy and rate limiter
    let client = HttpClient::new();
    let retry_policy = RetryPolicy::with_max_attempts(u32::from(args.max_retries));

    // Create rate limiter based on CLI flag
    let rate_limiter = if args.rate_limit == 0 {
        debug!("rate limiting disabled");
        Arc::new(RateLimiter::disabled())
    } else {
        debug!(rate_limit_ms = args.rate_limit, "rate limiting enabled");
        Arc::new(RateLimiter::new(Duration::from_millis(args.rate_limit)))
    };

    let engine = DownloadEngine::new(usize::from(args.concurrency), retry_policy, rate_limiter)?;

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
