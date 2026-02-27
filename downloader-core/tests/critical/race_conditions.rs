//! Phase 1 (P0): Concurrent queue access, file I/O races.
//! Multiple tasks enqueue/dequeue/status updates; assert no panics and eventual consistency.

use std::sync::atomic::{AtomicUsize, Ordering};

use downloader_core::{Database, Queue, QueueStatus};
use tempfile::TempDir;

async fn setup_queue() -> (Queue, TempDir) {
    let temp_dir = TempDir::new().expect("temp dir");
    let db_path = temp_dir.path().join("test.db");
    let db = Database::new(&db_path).await.expect("create db");
    (Queue::new(db), temp_dir)
}

#[tokio::test]
async fn p0_concurrent_enqueue_dequeue_no_panic() {
    let (queue, _temp) = setup_queue().await;

    let num_tasks = 20_usize;
    let ops_per_task = 10_usize;
    let barrier = std::sync::Arc::new(tokio::sync::Barrier::new(num_tasks + 1));
    let enqueued = std::sync::Arc::new(AtomicUsize::new(0));
    let completed = std::sync::Arc::new(AtomicUsize::new(0));

    let mut handles = Vec::new();
    for t in 0..num_tasks {
        let q = queue.clone();
        let bar = std::sync::Arc::clone(&barrier);
        let enc = std::sync::Arc::clone(&enqueued);
        let comp = std::sync::Arc::clone(&completed);
        handles.push(tokio::spawn(async move {
            for i in 0..ops_per_task {
                let url = format!("https://example.com/r-{}-{}.pdf", t, i);
                if q.enqueue(&url, "direct_url", None).await.is_ok() {
                    enc.fetch_add(1, Ordering::SeqCst);
                }
            }
            bar.wait().await;
            for _ in 0..ops_per_task {
                if let Ok(Some(item)) = q.dequeue().await {
                    let _ = q.mark_completed(item.id).await;
                    comp.fetch_add(1, Ordering::SeqCst);
                }
            }
        }));
    }

    barrier.wait().await;

    for h in handles {
        let _ = h.await.expect("task join");
    }

    let enc = enqueued.load(Ordering::SeqCst);
    let comp = completed.load(Ordering::SeqCst);
    assert!(enc > 0 && comp > 0, "enqueued={} completed={}", enc, comp);
}

#[tokio::test]
async fn p0_concurrent_status_updates_consistent() {
    let (queue, _temp) = setup_queue().await;

    let ids: Vec<i64> = {
        let mut v = Vec::new();
        for i in 0..50 {
            let id = queue
                .enqueue(
                    &format!("https://example.com/s-{}.pdf", i),
                    "direct_url",
                    None,
                )
                .await
                .expect("enqueue");
            v.push(id);
        }
        v
    };

    let completed = std::sync::Arc::new(AtomicUsize::new(0));
    let mut handles = Vec::new();
    for chunk in ids.chunks(10) {
        let q = queue.clone();
        let chunk = chunk.to_vec();
        let comp = std::sync::Arc::clone(&completed);
        handles.push(tokio::spawn(async move {
            for id in chunk {
                let _ = q.mark_completed(id).await;
                comp.fetch_add(1, Ordering::SeqCst);
            }
        }));
    }

    for h in handles {
        let _ = h.await.expect("join");
    }

    assert_eq!(completed.load(Ordering::SeqCst), 50);

    for id in &ids {
        let item = queue.get(*id).await.expect("get").expect("item exists");
        assert_eq!(item.status(), QueueStatus::Completed, "id {}", id);
    }
}

/// Production-style: workers only dequeue then mark_completed (no direct mark on Pending).
#[tokio::test]
async fn p0_concurrent_dequeue_then_mark_completed_no_panic() {
    let (queue, _temp) = setup_queue().await;
    let n = 30_usize;

    for i in 0..n {
        queue
            .enqueue(
                &format!("https://example.com/dq-{}.pdf", i),
                "direct_url",
                None,
            )
            .await
            .expect("enqueue");
    }

    let completed = std::sync::Arc::new(AtomicUsize::new(0));
    let mut handles = Vec::new();
    for _ in 0..8 {
        let q = queue.clone();
        let c = std::sync::Arc::clone(&completed);
        handles.push(tokio::spawn(async move {
            loop {
                if let Ok(Some(item)) = q.dequeue().await {
                    let _ = q.mark_completed(item.id).await;
                    c.fetch_add(1, Ordering::SeqCst);
                } else {
                    if c.load(Ordering::SeqCst) >= n {
                        break;
                    }
                    tokio::task::yield_now().await;
                }
            }
        }));
    }
    for h in handles {
        let _ = h.await.expect("join");
    }
    assert_eq!(completed.load(Ordering::SeqCst), n);
}
