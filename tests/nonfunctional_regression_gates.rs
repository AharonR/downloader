//! Non-functional regression gates for queue/runtime behavior.
//!
//! These tests are intentionally `#[ignore]` so they run on-demand during
//! refactor phase reviews:
//! `cargo test --test nonfunctional_regression_gates -- --ignored --nocapture`

use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Instant;

use downloader_core::{Database, DatabaseOptions, Queue, QueueStatus};
use tempfile::TempDir;
use tokio::task::JoinSet;

const MAX_P95_RUNTIME_REGRESSION: f64 = 0.07;
const MAX_QUEUE_THROUGHPUT_REGRESSION: f64 = 0.05;
const MAX_DB_BUSY_LOCK_RATE: f64 = 0.005;

const DEFAULT_BASELINE_QUEUE_THROUGHPUT_OPS_PER_SEC: f64 = 200.0;
const DEFAULT_BASELINE_RETRY_P95_MS: f64 = 50.0;

fn baseline_from_env(var_name: &str, fallback: f64) -> f64 {
    std::env::var(var_name)
        .ok()
        .and_then(|value| value.parse::<f64>().ok())
        .filter(|value| *value > 0.0)
        .unwrap_or(fallback)
}

fn p95(samples_ms: &mut [f64]) -> f64 {
    samples_ms.sort_by(|a, b| a.total_cmp(b));
    let rank = ((samples_ms.len() as f64) * 0.95).ceil() as usize;
    let index = rank
        .saturating_sub(1)
        .min(samples_ms.len().saturating_sub(1));
    samples_ms[index]
}

async fn setup_file_backed_queue(
    file_name: &str,
    options: DatabaseOptions,
) -> Result<(Queue, TempDir), Box<dyn std::error::Error>> {
    let temp_dir = TempDir::new()?;
    let db_path = temp_dir.path().join(file_name);
    let db = Database::new_with_options(&db_path, &options).await?;
    Ok((Queue::new(db), temp_dir))
}

#[tokio::test]
#[ignore = "non-functional gate: queue throughput baseline"]
async fn gate_queue_throughput_regression_is_within_5_percent()
-> Result<(), Box<dyn std::error::Error>> {
    let (queue, _temp_dir) = setup_file_backed_queue(
        "throughput_gate.db",
        DatabaseOptions {
            max_connections: 4,
            busy_timeout_ms: 5_000,
        },
    )
    .await?;

    let item_count = 600usize;
    for i in 0..item_count {
        queue
            .enqueue(&format!("https://example.com/{i}.pdf"), "direct_url", None)
            .await?;
    }

    let start = Instant::now();
    let mut completed = 0usize;
    while let Some(item) = queue.dequeue().await? {
        queue.mark_completed(item.id).await?;
        completed += 1;
    }
    let elapsed = start.elapsed();
    let throughput = completed as f64 / elapsed.as_secs_f64();

    let baseline = baseline_from_env(
        "NF_BASELINE_QUEUE_THROUGHPUT_OPS_PER_SEC",
        DEFAULT_BASELINE_QUEUE_THROUGHPUT_OPS_PER_SEC,
    );
    let min_allowed = baseline * (1.0 - MAX_QUEUE_THROUGHPUT_REGRESSION);

    assert_eq!(completed, item_count);
    assert!(
        throughput >= min_allowed,
        "throughput regression exceeded threshold: measured={throughput:.2}ops/s baseline={baseline:.2}ops/s min_allowed={min_allowed:.2}ops/s"
    );
    Ok(())
}

#[tokio::test]
#[ignore = "non-functional gate: retry-path p95 baseline"]
async fn gate_retry_path_p95_regression_is_within_7_percent()
-> Result<(), Box<dyn std::error::Error>> {
    let (queue, _temp_dir) = setup_file_backed_queue(
        "retry_p95_gate.db",
        DatabaseOptions {
            max_connections: 2,
            busy_timeout_ms: 5_000,
        },
    )
    .await?;

    for i in 0..120 {
        queue
            .enqueue(
                &format!("https://example.com/retry-heavy-{i}.pdf"),
                "direct_url",
                None,
            )
            .await?;
    }

    let mut samples_ms = Vec::with_capacity(120);
    while let Some(item) = queue.dequeue().await? {
        let start = Instant::now();

        queue
            .mark_failed(item.id, "transient network error", 1)
            .await?;
        queue.requeue(item.id).await?;

        let retry_one = queue.dequeue().await?.expect("requeued item should exist");
        queue
            .mark_failed(retry_one.id, "transient timeout", 2)
            .await?;
        queue.requeue(retry_one.id).await?;

        let retry_two = queue.dequeue().await?.expect("second requeue should exist");
        queue.mark_completed(retry_two.id).await?;

        samples_ms.push(start.elapsed().as_secs_f64() * 1000.0);
    }

    let baseline = baseline_from_env(
        "NF_BASELINE_RETRY_PATH_P95_MS",
        DEFAULT_BASELINE_RETRY_P95_MS,
    );
    let max_allowed = baseline * (1.0 + MAX_P95_RUNTIME_REGRESSION);
    let measured_p95 = p95(&mut samples_ms);

    let completed = queue.count_by_status(QueueStatus::Completed).await?;
    assert_eq!(completed, 120);
    assert!(
        measured_p95 <= max_allowed,
        "retry-path p95 regression exceeded threshold: measured={measured_p95:.3}ms baseline={baseline:.3}ms max_allowed={max_allowed:.3}ms"
    );
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
#[ignore = "non-functional gate: db lock/busy incidence"]
async fn gate_db_busy_lock_incidence_stays_below_half_percent()
-> Result<(), Box<dyn std::error::Error>> {
    let (queue, _temp_dir) = setup_file_backed_queue(
        "lock_gate.db",
        DatabaseOptions {
            max_connections: 8,
            busy_timeout_ms: 200,
        },
    )
    .await?;

    let mut ids = Vec::with_capacity(32);
    for i in 0..32 {
        ids.push(
            queue
                .enqueue(
                    &format!("https://example.com/lock-{i}.pdf"),
                    "direct_url",
                    None,
                )
                .await?,
        );
    }

    let ids = Arc::new(ids);
    let workers = 12usize;
    let ops_per_worker = 300usize;
    let total_ops = workers * ops_per_worker;
    let busy_errors = Arc::new(AtomicUsize::new(0));
    let total_errors = Arc::new(AtomicUsize::new(0));
    let mut tasks = JoinSet::new();

    for worker in 0..workers {
        let queue = queue.clone();
        let ids = Arc::clone(&ids);
        let busy_errors = Arc::clone(&busy_errors);
        let total_errors = Arc::clone(&total_errors);

        tasks.spawn(async move {
            for i in 0..ops_per_worker {
                let id = ids[(worker + i) % ids.len()];
                let bytes_downloaded = ((worker * ops_per_worker + i) as i64) % 20_000;
                if let Err(error) = queue
                    .update_progress(id, bytes_downloaded, Some(20_000))
                    .await
                {
                    total_errors.fetch_add(1, Ordering::SeqCst);
                    if error.is_busy_or_locked() {
                        busy_errors.fetch_add(1, Ordering::SeqCst);
                    }
                }
            }
        });
    }

    while tasks.join_next().await.is_some() {}

    let busy = busy_errors.load(Ordering::SeqCst);
    let busy_ratio = busy as f64 / total_ops as f64;

    assert!(
        busy_ratio <= MAX_DB_BUSY_LOCK_RATE,
        "busy/lock rate exceeded threshold: busy={busy} total_ops={total_ops} ratio={busy_ratio:.6} max={MAX_DB_BUSY_LOCK_RATE:.6}"
    );
    Ok(())
}
