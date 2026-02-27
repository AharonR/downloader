//! Shared utilities for critical tests (data corruption, flaky network, FD exhaustion, load generation).
//!
//! Used by tests under `tests/critical/` to create corrupted DBs, flaky HTTP mocks,
//! lowered file descriptor limits, and concurrent queue load.

use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use downloader_core::Database;
use tempfile::TempDir;
use wiremock::matchers::{method, path};
use wiremock::{Mock, Respond, ResponseTemplate};

use super::socket_guard::start_mock_server_or_skip;

/// Creates a SQLite database file that is intentionally corrupted (invalid content).
///
/// Returns the path to the corrupted file and a `TempDir` that must be kept alive
/// for the path to remain valid. Opening this path with `Database::new()` should
/// yield `DbError::Connection` or `DbError::Migration`.
pub fn corrupted_database() -> (TempDir, PathBuf) {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let db_path = temp_dir.path().join("corrupted.db");

    // Write invalid SQLite content (garbage header so file exists but is not valid DB)
    std::fs::write(&db_path, b"not a valid sqlite file\x00\x00\x00")
        .expect("Failed to write corrupted db file");

    (temp_dir, db_path)
}

/// Creates a truncated SQLite file (valid header but truncated, e.g. mid-write).
///
/// First creates a valid DB, then truncates the file to simulate partial write/crash.
#[allow(dead_code)]
pub async fn truncated_database() -> (TempDir, PathBuf) {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let db_path = temp_dir.path().join("truncated.db");

    let db = Database::new(&db_path).await.expect("create valid db");
    drop(db);

    // Truncate to corrupt (e.g. 512 bytes - past header but before full schema)
    let meta = std::fs::metadata(&db_path).expect("metadata");
    let len = meta.len();
    let truncated_len = std::cmp::min(512_usize, len as usize);
    let content = std::fs::read(&db_path).expect("read");
    std::fs::write(&db_path, &content[..truncated_len]).expect("truncate");

    (temp_dir, db_path)
}

/// Responder that fails the first `fail_count` requests with 500, then returns 200 with body.
#[allow(dead_code)]
struct FlakyResponder {
    request_count: Arc<AtomicUsize>,
    fail_count: usize,
    success_body: Vec<u8>,
}

impl Respond for FlakyResponder {
    fn respond(&self, _request: &wiremock::Request) -> ResponseTemplate {
        let n = self.request_count.fetch_add(1, Ordering::SeqCst);
        if n < self.fail_count {
            ResponseTemplate::new(500).set_body_bytes(b"internal server error")
        } else {
            ResponseTemplate::new(200).set_body_bytes(self.success_body.clone())
        }
    }
}

/// Flaky mock server: fails `fail_count` times with 500 then returns 200 with `success_body`.
///
/// Returns `Some((MockServer, base_uri))` when socket is available, or `None` when skipped.
/// Mounts a single GET responder for path `/file` that counts requests and returns
/// 500 for the first `fail_count` requests, then 200 with the given body.
#[allow(dead_code)]
pub async fn flaky_network_mock(
    fail_count: usize,
    success_body: Vec<u8>,
) -> Option<(wiremock::MockServer, String)> {
    let mock_server = start_mock_server_or_skip().await?;
    let request_count = Arc::new(AtomicUsize::new(0));

    let responder = FlakyResponder {
        request_count: Arc::clone(&request_count),
        fail_count,
        success_body,
    };

    Mock::given(method("GET"))
        .and(path("/file"))
        .respond_with(responder)
        .mount(&mock_server)
        .await;

    let base_uri = mock_server.uri();
    Some((mock_server, base_uri))
}

/// Result of a concurrent load run (enqueued, completed, failed counts).
#[allow(dead_code)]
#[derive(Debug, Default)]
pub struct LoadResult {
    pub enqueued: usize,
    pub completed: usize,
    pub failed: usize,
}

/// Runs many concurrent queue operations to stress the queue and connection pool.
///
/// Spawns `num_tasks` tasks; each task enqueues `ops_per_task` items then dequeues
/// and marks them completed (or failed if dequeue returns None). Uses a single shared
/// queue (cloned per task). Returns aggregate counts.
#[allow(dead_code)]
pub async fn concurrent_load_generator(
    queue: downloader_core::Queue,
    num_tasks: usize,
    ops_per_task: usize,
) -> LoadResult {
    use tokio::sync::Barrier;

    let barrier = Arc::new(Barrier::new(num_tasks + 1));
    let enqueued = Arc::new(AtomicUsize::new(0));
    let completed = Arc::new(AtomicUsize::new(0));
    let failed = Arc::new(AtomicUsize::new(0));

    let mut handles = Vec::with_capacity(num_tasks);
    for t in 0..num_tasks {
        let q = queue.clone();
        let bar = Arc::clone(&barrier);
        let enc = Arc::clone(&enqueued);
        let comp = Arc::clone(&completed);
        let fail = Arc::clone(&failed);
        handles.push(tokio::spawn(async move {
            let mut my_enqueued = 0_usize;
            for i in 0..ops_per_task {
                let url = format!("https://example.com/file-{}-{}.pdf", t, i);
                if q.enqueue(&url, "direct_url", None).await.is_ok() {
                    my_enqueued += 1;
                }
            }
            enc.fetch_add(my_enqueued, Ordering::SeqCst);

            bar.wait().await;

            for _ in 0..my_enqueued {
                if let Ok(Some(item)) = q.dequeue().await {
                    if q.mark_completed(item.id).await.is_ok() {
                        comp.fetch_add(1, Ordering::SeqCst);
                    } else {
                        fail.fetch_add(1, Ordering::SeqCst);
                    }
                }
            }
        }));
    }

    barrier.wait().await;

    for h in handles {
        let _ = h.await;
    }

    LoadResult {
        enqueued: enqueued.load(Ordering::SeqCst),
        completed: completed.load(Ordering::SeqCst),
        failed: failed.load(Ordering::SeqCst),
    }
}

/// Guard that restores the process file descriptor limit when dropped.
/// On non-Unix platforms this is a no-op and `exhausted_file_descriptors()` returns `None`.
#[cfg(unix)]
#[allow(dead_code)]
pub struct FdLimitGuard {
    soft: libc::rlim_t,
    hard: libc::rlim_t,
}

#[cfg(unix)]
impl Drop for FdLimitGuard {
    fn drop(&mut self) {
        let mut rlim = libc::rlimit {
            rlim_cur: self.soft,
            rlim_max: self.hard,
        };
        unsafe { libc::setrlimit(libc::RLIMIT_NOFILE, &mut rlim) };
    }
}

/// Lowers the process soft file descriptor limit to `limit` for testing.
/// Returns a guard that restores the previous limit on drop, or `None` on non-Unix
/// or if getting/setting the limit fails (e.g. permission denied).
#[cfg(unix)]
#[allow(dead_code)]
pub fn exhausted_file_descriptors(limit: u64) -> Option<FdLimitGuard> {
    let mut rlim = libc::rlimit {
        rlim_cur: 0,
        rlim_max: 0,
    };
    if unsafe { libc::getrlimit(libc::RLIMIT_NOFILE, &mut rlim) } != 0 {
        return None;
    }
    let previous_soft = rlim.rlim_cur;
    let previous_hard = rlim.rlim_max;
    let new_soft = limit.min(previous_hard) as libc::rlim_t;
    rlim.rlim_cur = new_soft;
    if unsafe { libc::setrlimit(libc::RLIMIT_NOFILE, &rlim) } != 0 {
        return None;
    }
    Some(FdLimitGuard {
        soft: previous_soft,
        hard: previous_hard,
    })
}

/// Stub on non-Unix: no FD limit change; tests that require it should skip.
#[cfg(not(unix))]
#[allow(dead_code)]
pub fn exhausted_file_descriptors(_limit: u64) -> Option<()> {
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_corrupted_database_creates_file() {
        let (_temp, path) = corrupted_database();
        assert!(path.exists());
        let b = std::fs::read(&path).unwrap();
        assert!(!b.starts_with(b"SQLite format 3"));
    }
}
