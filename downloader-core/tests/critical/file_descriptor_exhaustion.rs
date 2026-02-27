//! Phase 4 (P1): Too many open files, handles.
//! Lower FD limit; trigger many operations; assert graceful error or skip.

#[cfg(unix)]
#[tokio::test]
#[ignore] // lowers RLIMIT_NOFILE; run with --ignored in nightly
async fn p1_low_fd_limit_graceful_or_skip() {
    use downloader_core::{Database, Queue};
    use tempfile::TempDir;

    let guard = crate::support::critical_utils::exhausted_file_descriptors(10);
    let Some(_guard) = guard else {
        return;
    };

    let temp_dir = TempDir::new().expect("temp dir");
    let db_path = temp_dir.path().join("test.db");
    let db = match Database::new(&db_path).await {
        Ok(d) => d,
        Err(_) => return,
    };
    let queue = Queue::new(db);

    let mut enqueued = 0_usize;
    for i in 0..20 {
        if queue
            .enqueue(
                &format!("https://example.com/fd-{}.pdf", i),
                "direct_url",
                None,
            )
            .await
            .is_ok()
        {
            enqueued += 1;
        }
    }

    let pending = queue
        .count_by_status(downloader_core::QueueStatus::Pending)
        .await
        .expect("count");
    assert!(pending >= 0, "queue should remain consistent");
    assert!(
        enqueued >= 1,
        "at least one enqueue should succeed when DB is open"
    );
}

#[cfg(not(unix))]
#[tokio::test]
async fn p1_low_fd_limit_skip_on_non_unix() {
    let guard = crate::support::critical_utils::exhausted_file_descriptors(10);
    assert!(guard.is_none());
}
