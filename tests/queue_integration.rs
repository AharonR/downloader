//! Integration tests for the queue module.
//!
//! These tests verify Queue operations against a real SQLite database.

use downloader_core::{Database, Queue, QueueError, QueueStatus};
use tempfile::TempDir;

/// Helper to create a test database with migrations applied.
async fn setup_test_db() -> (Database, TempDir) {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let db_path = temp_dir.path().join("test.db");

    let db = Database::new(&db_path)
        .await
        .expect("Failed to create database");

    (db, temp_dir)
}

// ==================== Basic Operations ====================

#[tokio::test]
async fn test_enqueue_creates_pending_item() {
    let (db, _temp_dir) = setup_test_db().await;
    let queue = Queue::new(db);

    let id = queue
        .enqueue("https://example.com/paper.pdf", "direct_url", None)
        .await
        .expect("Failed to enqueue");

    assert!(id > 0);

    let item = queue.get(id).await.expect("Failed to get").unwrap();
    assert_eq!(item.url, "https://example.com/paper.pdf");
    assert_eq!(item.source_type, "direct_url");
    assert_eq!(item.status(), QueueStatus::Pending);
    assert_eq!(item.retry_count, 0);
}

#[tokio::test]
async fn test_enqueue_with_original_input() {
    let (db, _temp_dir) = setup_test_db().await;
    let queue = Queue::new(db);

    let id = queue
        .enqueue(
            "https://doi.org/10.1234/example",
            "doi",
            Some("10.1234/example"),
        )
        .await
        .expect("Failed to enqueue");

    let item = queue.get(id).await.expect("Failed to get").unwrap();
    assert_eq!(item.original_input, Some("10.1234/example".to_string()));
    assert_eq!(item.source_type, "doi");
}

#[tokio::test]
async fn test_dequeue_returns_pending_item_and_marks_in_progress() {
    let (db, _temp_dir) = setup_test_db().await;
    let queue = Queue::new(db);

    queue
        .enqueue("https://example.com/1.pdf", "direct_url", None)
        .await
        .expect("Failed to enqueue");

    let item = queue
        .dequeue()
        .await
        .expect("Failed to dequeue")
        .expect("Expected item");

    assert_eq!(item.url, "https://example.com/1.pdf");
    assert_eq!(item.status(), QueueStatus::InProgress);
}

#[tokio::test]
async fn test_dequeue_returns_none_when_empty() {
    let (db, _temp_dir) = setup_test_db().await;
    let queue = Queue::new(db);

    let result = queue.dequeue().await.expect("Failed to dequeue");
    assert!(result.is_none());
}

#[tokio::test]
async fn test_dequeue_respects_priority_order() {
    let (db, _temp_dir) = setup_test_db().await;
    let queue = Queue::new(db);

    // Enqueue items (priority is 0 by default, so FIFO by created_at)
    queue
        .enqueue("https://example.com/first.pdf", "direct_url", None)
        .await
        .unwrap();
    queue
        .enqueue("https://example.com/second.pdf", "direct_url", None)
        .await
        .unwrap();

    // Should get first item (oldest)
    let item = queue.dequeue().await.unwrap().unwrap();
    assert_eq!(item.url, "https://example.com/first.pdf");

    // Should get second item
    let item = queue.dequeue().await.unwrap().unwrap();
    assert_eq!(item.url, "https://example.com/second.pdf");
}

// ==================== Status Transitions ====================

#[tokio::test]
async fn test_mark_completed() {
    let (db, _temp_dir) = setup_test_db().await;
    let queue = Queue::new(db);

    let id = queue
        .enqueue("https://example.com/paper.pdf", "direct_url", None)
        .await
        .unwrap();
    queue.dequeue().await.unwrap(); // Mark as in_progress

    queue
        .mark_completed(id)
        .await
        .expect("Failed to mark completed");

    let item = queue.get(id).await.unwrap().unwrap();
    assert_eq!(item.status(), QueueStatus::Completed);
}

#[tokio::test]
async fn test_mark_failed_sets_retry_count() {
    let (db, _temp_dir) = setup_test_db().await;
    let queue = Queue::new(db);

    let id = queue
        .enqueue("https://example.com/paper.pdf", "direct_url", None)
        .await
        .unwrap();
    queue.dequeue().await.unwrap();

    queue
        .mark_failed(id, "Connection timeout", 1)
        .await
        .expect("Failed to mark failed");

    let item = queue.get(id).await.unwrap().unwrap();
    assert_eq!(item.status(), QueueStatus::Failed);
    assert_eq!(item.retry_count, 1);
    assert_eq!(item.last_error, Some("Connection timeout".to_string()));

    // Mark failed again
    queue.requeue(id).await.unwrap();
    queue.dequeue().await.unwrap();
    queue.mark_failed(id, "Server error", 2).await.unwrap();

    let item = queue.get(id).await.unwrap().unwrap();
    assert_eq!(item.retry_count, 2);
    assert_eq!(item.last_error, Some("Server error".to_string()));
}

#[tokio::test]
async fn test_requeue_returns_to_pending() {
    let (db, _temp_dir) = setup_test_db().await;
    let queue = Queue::new(db);

    let id = queue
        .enqueue("https://example.com/paper.pdf", "direct_url", None)
        .await
        .unwrap();
    queue.dequeue().await.unwrap(); // Mark as in_progress

    queue.requeue(id).await.expect("Failed to requeue");

    let item = queue.get(id).await.unwrap().unwrap();
    assert_eq!(item.status(), QueueStatus::Pending);
}

// ==================== Error Handling ====================

#[tokio::test]
async fn test_mark_completed_nonexistent_returns_error() {
    let (db, _temp_dir) = setup_test_db().await;
    let queue = Queue::new(db);

    let result = queue.mark_completed(99999).await;
    assert!(matches!(result, Err(QueueError::ItemNotFound(99999))));
}

#[tokio::test]
async fn test_mark_failed_nonexistent_returns_error() {
    let (db, _temp_dir) = setup_test_db().await;
    let queue = Queue::new(db);

    let result = queue.mark_failed(99999, "error", 1).await;
    assert!(matches!(result, Err(QueueError::ItemNotFound(99999))));
}

#[tokio::test]
async fn test_requeue_nonexistent_returns_error() {
    let (db, _temp_dir) = setup_test_db().await;
    let queue = Queue::new(db);

    let result = queue.requeue(99999).await;
    assert!(matches!(result, Err(QueueError::ItemNotFound(99999))));
}

#[tokio::test]
async fn test_remove_nonexistent_returns_error() {
    let (db, _temp_dir) = setup_test_db().await;
    let queue = Queue::new(db);

    let result = queue.remove(99999).await;
    assert!(matches!(result, Err(QueueError::ItemNotFound(99999))));
}

#[tokio::test]
async fn test_get_nonexistent_returns_none() {
    let (db, _temp_dir) = setup_test_db().await;
    let queue = Queue::new(db);

    let result = queue.get(99999).await.expect("Failed to query");
    assert!(result.is_none());
}

// ==================== Listing and Counting ====================

#[tokio::test]
async fn test_count_by_status() {
    let (db, _temp_dir) = setup_test_db().await;
    let queue = Queue::new(db);

    // Add 3 items
    queue
        .enqueue("https://example.com/1.pdf", "direct_url", None)
        .await
        .unwrap();
    queue
        .enqueue("https://example.com/2.pdf", "direct_url", None)
        .await
        .unwrap();
    queue
        .enqueue("https://example.com/3.pdf", "direct_url", None)
        .await
        .unwrap();

    assert_eq!(
        queue.count_by_status(QueueStatus::Pending).await.unwrap(),
        3
    );
    assert_eq!(
        queue
            .count_by_status(QueueStatus::InProgress)
            .await
            .unwrap(),
        0
    );

    // Dequeue one
    queue.dequeue().await.unwrap();

    assert_eq!(
        queue.count_by_status(QueueStatus::Pending).await.unwrap(),
        2
    );
    assert_eq!(
        queue
            .count_by_status(QueueStatus::InProgress)
            .await
            .unwrap(),
        1
    );
}

#[tokio::test]
async fn test_list_by_status() {
    let (db, _temp_dir) = setup_test_db().await;
    let queue = Queue::new(db);

    queue
        .enqueue("https://example.com/1.pdf", "direct_url", None)
        .await
        .unwrap();
    let id2 = queue
        .enqueue("https://example.com/2.pdf", "direct_url", None)
        .await
        .unwrap();
    queue
        .enqueue("https://example.com/3.pdf", "direct_url", None)
        .await
        .unwrap();

    // Mark one as completed
    queue.dequeue().await.unwrap(); // dequeues 1.pdf
    queue
        .mark_completed(queue.get(1).await.unwrap().unwrap().id)
        .await
        .unwrap();

    // Dequeue another (marks as in_progress)
    queue.dequeue().await.unwrap(); // dequeues 2.pdf

    let pending = queue.list_by_status(QueueStatus::Pending).await.unwrap();
    assert_eq!(pending.len(), 1);
    assert_eq!(pending[0].url, "https://example.com/3.pdf");

    let in_progress = queue.list_by_status(QueueStatus::InProgress).await.unwrap();
    assert_eq!(in_progress.len(), 1);
    assert_eq!(in_progress[0].id, id2);
}

#[tokio::test]
async fn test_list_all() {
    let (db, _temp_dir) = setup_test_db().await;
    let queue = Queue::new(db);

    queue
        .enqueue("https://example.com/1.pdf", "direct_url", None)
        .await
        .unwrap();
    queue
        .enqueue("https://example.com/2.pdf", "direct_url", None)
        .await
        .unwrap();

    let all = queue.list_all().await.unwrap();
    assert_eq!(all.len(), 2);
}

// ==================== Crash Recovery ====================

#[tokio::test]
async fn test_get_in_progress() {
    let (db, _temp_dir) = setup_test_db().await;
    let queue = Queue::new(db);

    queue
        .enqueue("https://example.com/1.pdf", "direct_url", None)
        .await
        .unwrap();
    queue
        .enqueue("https://example.com/2.pdf", "direct_url", None)
        .await
        .unwrap();
    queue
        .enqueue("https://example.com/3.pdf", "direct_url", None)
        .await
        .unwrap();

    // Dequeue two items (marks them in_progress)
    queue.dequeue().await.unwrap();
    queue.dequeue().await.unwrap();

    let in_progress = queue.get_in_progress().await.unwrap();
    assert_eq!(in_progress.len(), 2);
}

#[tokio::test]
async fn test_reset_in_progress() {
    let (db, _temp_dir) = setup_test_db().await;
    let queue = Queue::new(db);

    queue
        .enqueue("https://example.com/1.pdf", "direct_url", None)
        .await
        .unwrap();
    queue
        .enqueue("https://example.com/2.pdf", "direct_url", None)
        .await
        .unwrap();

    // Dequeue both (marks them in_progress)
    queue.dequeue().await.unwrap();
    queue.dequeue().await.unwrap();

    assert_eq!(
        queue
            .count_by_status(QueueStatus::InProgress)
            .await
            .unwrap(),
        2
    );
    assert_eq!(
        queue.count_by_status(QueueStatus::Pending).await.unwrap(),
        0
    );

    // Reset all in_progress back to pending
    let reset_count = queue.reset_in_progress().await.unwrap();
    assert_eq!(reset_count, 2);

    assert_eq!(
        queue
            .count_by_status(QueueStatus::InProgress)
            .await
            .unwrap(),
        0
    );
    assert_eq!(
        queue.count_by_status(QueueStatus::Pending).await.unwrap(),
        2
    );
}

// ==================== Removal Operations ====================

#[tokio::test]
async fn test_remove() {
    let (db, _temp_dir) = setup_test_db().await;
    let queue = Queue::new(db);

    let id = queue
        .enqueue("https://example.com/paper.pdf", "direct_url", None)
        .await
        .unwrap();

    queue.remove(id).await.expect("Failed to remove");

    let result = queue.get(id).await.unwrap();
    assert!(result.is_none());
}

#[tokio::test]
async fn test_clear_by_status() {
    let (db, _temp_dir) = setup_test_db().await;
    let queue = Queue::new(db);

    // Add items with different statuses
    let id1 = queue
        .enqueue("https://example.com/1.pdf", "direct_url", None)
        .await
        .unwrap();
    queue
        .enqueue("https://example.com/2.pdf", "direct_url", None)
        .await
        .unwrap();
    queue
        .enqueue("https://example.com/3.pdf", "direct_url", None)
        .await
        .unwrap();

    // Mark first as completed
    queue.dequeue().await.unwrap();
    queue.mark_completed(id1).await.unwrap();

    // Clear completed items
    let cleared = queue.clear_by_status(QueueStatus::Completed).await.unwrap();
    assert_eq!(cleared, 1);

    // Verify only pending items remain
    assert_eq!(
        queue.count_by_status(QueueStatus::Completed).await.unwrap(),
        0
    );
    assert_eq!(
        queue.count_by_status(QueueStatus::Pending).await.unwrap(),
        2
    );
}

// ==================== Edge Cases ====================

#[tokio::test]
async fn test_duplicate_urls_allowed() {
    let (db, _temp_dir) = setup_test_db().await;
    let queue = Queue::new(db);

    let id1 = queue
        .enqueue("https://example.com/paper.pdf", "direct_url", None)
        .await
        .unwrap();
    let id2 = queue
        .enqueue("https://example.com/paper.pdf", "direct_url", None)
        .await
        .unwrap();

    assert_ne!(id1, id2);
    assert_eq!(
        queue.count_by_status(QueueStatus::Pending).await.unwrap(),
        2
    );
}

#[tokio::test]
async fn test_empty_queue_operations() {
    let (db, _temp_dir) = setup_test_db().await;
    let queue = Queue::new(db);

    // All these should work on empty queue
    assert_eq!(
        queue.count_by_status(QueueStatus::Pending).await.unwrap(),
        0
    );
    assert!(queue.list_all().await.unwrap().is_empty());
    assert!(queue.dequeue().await.unwrap().is_none());
    assert_eq!(queue.reset_in_progress().await.unwrap(), 0);
    assert_eq!(queue.clear_by_status(QueueStatus::Failed).await.unwrap(), 0);
}

#[tokio::test]
async fn test_long_url_and_error_message() {
    let (db, _temp_dir) = setup_test_db().await;
    let queue = Queue::new(db);

    let long_url = format!("https://example.com/{}", "a".repeat(1000));
    let long_error = "Error: ".to_string() + &"x".repeat(1000);

    let id = queue.enqueue(&long_url, "direct_url", None).await.unwrap();
    queue.dequeue().await.unwrap();
    queue.mark_failed(id, &long_error, 1).await.unwrap();

    let item = queue.get(id).await.unwrap().unwrap();
    assert_eq!(item.url, long_url);
    assert_eq!(item.last_error, Some(long_error));
}

// ==================== Concurrency ====================

#[tokio::test]
async fn test_concurrent_dequeue_returns_different_items() {
    let (db, _temp_dir) = setup_test_db().await;
    let queue = Queue::new(db);

    // Add 10 items
    for i in 0..10 {
        queue
            .enqueue(&format!("https://example.com/{i}.pdf"), "direct_url", None)
            .await
            .unwrap();
    }

    // Spawn 10 concurrent dequeue tasks
    let mut handles = Vec::new();
    for _ in 0..10 {
        let q = queue.clone();
        handles.push(tokio::spawn(async move { q.dequeue().await }));
    }

    // Collect results
    let mut dequeued_ids: Vec<i64> = Vec::new();
    for handle in handles {
        if let Ok(Ok(Some(item))) = handle.await {
            dequeued_ids.push(item.id);
        }
    }

    // All 10 items should be dequeued with unique IDs (no duplicates)
    assert_eq!(dequeued_ids.len(), 10, "All 10 items should be dequeued");

    // Check for duplicates
    dequeued_ids.sort();
    let original_len = dequeued_ids.len();
    dequeued_ids.dedup();
    assert_eq!(
        dequeued_ids.len(),
        original_len,
        "No duplicate items should be dequeued"
    );

    // Queue should be empty now
    assert!(queue.dequeue().await.unwrap().is_none());
}
