//! Integration tests for the queue module.
//!
//! These tests verify Queue operations against a real SQLite database.

use downloader_core::{
    Database, DownloadAttemptQuery, DownloadAttemptStatus, DownloadErrorType, DownloadSearchQuery,
    NewDownloadAttempt, Queue, QueueError, QueueMetadata, QueueStatus, parse_input,
};
use sqlx::Row;
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
async fn test_enqueue_with_metadata_persists_parse_confidence_fields() {
    let (db, _temp_dir) = setup_test_db().await;
    let queue = Queue::new(db);

    let metadata = QueueMetadata {
        suggested_filename: Some("Reference.pdf".to_string()),
        title: Some("Reference Title".to_string()),
        authors: Some("Author, A".to_string()),
        year: Some("2024".to_string()),
        doi: None,
        topics: None,
        parse_confidence: Some("low".to_string()),
        parse_confidence_factors: Some(
            r#"{"has_authors":false,"has_year":true,"has_title":false,"author_count":0}"#
                .to_string(),
        ),
    };

    let id = queue
        .enqueue_with_metadata(
            "https://example.com/reference.pdf",
            "reference",
            Some("Weak reference text"),
            Some(&metadata),
        )
        .await
        .expect("enqueue_with_metadata should succeed");

    let item = queue
        .get(id)
        .await
        .expect("queue get should succeed")
        .expect("queue item should exist");
    assert_eq!(item.parse_confidence.as_deref(), Some("low"));
    assert_eq!(
        item.parse_confidence_factors.as_deref(),
        Some(r#"{"has_authors":false,"has_year":true,"has_title":false,"author_count":0}"#)
    );
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

// ==================== Progress Metadata ====================

#[tokio::test]
async fn test_update_progress_persists_bytes_and_content_length() {
    let (db, _temp_dir) = setup_test_db().await;
    let queue = Queue::new(db);

    let id = queue
        .enqueue("https://example.com/large.pdf", "direct_url", None)
        .await
        .unwrap();
    queue.dequeue().await.unwrap();

    queue
        .update_progress(id, 500_000, Some(1_000_000))
        .await
        .expect("Failed to update progress");

    let item = queue.get(id).await.unwrap().unwrap();
    assert_eq!(item.bytes_downloaded, 500_000);
    assert_eq!(item.content_length, Some(1_000_000));
}

#[tokio::test]
async fn test_update_progress_without_content_length() {
    let (db, _temp_dir) = setup_test_db().await;
    let queue = Queue::new(db);

    let id = queue
        .enqueue("https://example.com/stream.bin", "direct_url", None)
        .await
        .unwrap();
    queue.dequeue().await.unwrap();

    queue
        .update_progress(id, 12345, None)
        .await
        .expect("Failed to update progress");

    let item = queue.get(id).await.unwrap().unwrap();
    assert_eq!(item.bytes_downloaded, 12345);
    assert_eq!(item.content_length, None);
}

#[tokio::test]
async fn test_update_progress_nonexistent_returns_error() {
    let (db, _temp_dir) = setup_test_db().await;
    let queue = Queue::new(db);

    let result = queue.update_progress(9999, 100, None).await;
    assert!(result.is_err(), "Expected ItemNotFound for nonexistent ID");
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

#[tokio::test]
async fn test_enqueue_mixed_parser_output_preserves_source_type_metadata() {
    let (db, _temp_dir) = setup_test_db().await;
    let queue = Queue::new(db);

    let input = r#"
https://example.com/a.pdf
10.1234/example
Smith, J. (2024). Existing Reference. Journal.
@article{key, title={BibTeX Title}, author={Doe, R.}, year={2023}, doi={10.5678/bib}}
"#;
    let parse_result = parse_input(input);
    assert!(
        !parse_result.is_empty(),
        "mixed parse result should contain actionable items"
    );

    for item in &parse_result.items {
        queue
            .enqueue(
                &item.value,
                item.input_type.queue_source_type(),
                Some(&item.raw),
            )
            .await
            .expect("failed to enqueue mixed parsed item");
    }

    let all_items = queue.list_all().await.expect("failed to list queue");
    assert_eq!(
        all_items.len(),
        parse_result.len(),
        "all parsed items should be represented in queue"
    );
    assert!(
        all_items
            .iter()
            .any(|item| item.source_type == "direct_url"),
        "queue should contain direct_url sourced items"
    );
    assert!(
        all_items.iter().any(|item| item.source_type == "doi"),
        "queue should contain doi sourced items"
    );
    assert!(
        all_items.iter().any(|item| item.source_type == "reference"),
        "queue should contain reference-sourced items"
    );
    assert!(
        all_items.iter().any(|item| item.source_type == "bibtex"),
        "queue should contain bibtex-sourced items"
    );
}

// ==================== Download History ====================

#[tokio::test]
async fn test_log_download_attempt_persists_success_row() {
    let (db, _temp_dir) = setup_test_db().await;
    let queue = Queue::new(db);

    let attempt = NewDownloadAttempt {
        url: "https://example.com/paper.pdf",
        final_url: Some("https://example.com/paper.pdf"),
        status: DownloadAttemptStatus::Success,
        file_path: Some("/tmp/paper.pdf"),
        file_size: Some(1024),
        content_type: Some("application/pdf"),
        error_message: None,
        error_type: None,
        retry_count: 0,
        project: Some("project-a"),
        original_input: Some("https://example.com/paper.pdf"),
        http_status: Some(200),
        duration_ms: Some(120),
        title: Some("Paper Title"),
        authors: Some("Doe, Jane"),
        doi: Some("10.1000/example"),
        topics: None,
        parse_confidence: Some("high"),
        parse_confidence_factors: Some(
            r#"{"has_authors":true,"has_year":true,"has_title":true,"author_count":1}"#,
        ),
    };

    let id = queue
        .log_download_attempt(&attempt)
        .await
        .expect("history row insert should succeed");
    assert!(id > 0, "history id should be positive");

    let rows = queue
        .query_download_attempts(&DownloadAttemptQuery::default())
        .await
        .expect("history query should succeed");
    assert_eq!(rows.len(), 1, "one history row expected");

    let row = &rows[0];
    assert_eq!(row.status(), DownloadAttemptStatus::Success);
    assert_eq!(row.url, "https://example.com/paper.pdf");
    assert_eq!(row.file_path.as_deref(), Some("/tmp/paper.pdf"));
    assert_eq!(row.title.as_deref(), Some("Paper Title"));
    assert_eq!(row.authors.as_deref(), Some("Doe, Jane"));
    assert_eq!(row.doi.as_deref(), Some("10.1000/example"));
    assert_eq!(row.project.as_deref(), Some("project-a"));
    assert_eq!(row.parse_confidence.as_deref(), Some("high"));
    assert_eq!(
        row.parse_confidence_factors.as_deref(),
        Some(r#"{"has_authors":true,"has_year":true,"has_title":true,"author_count":1}"#)
    );
}

#[tokio::test]
async fn test_log_download_attempt_confidence_factors_are_queryable_as_json() {
    let (db, _temp_dir) = setup_test_db().await;
    let db_for_query = db.clone();
    let queue = Queue::new(db);

    let low_attempt = NewDownloadAttempt {
        url: "https://example.com/low.pdf",
        final_url: None,
        status: DownloadAttemptStatus::Success,
        file_path: Some("/tmp/low.pdf"),
        file_size: Some(256),
        content_type: Some("application/pdf"),
        error_message: None,
        error_type: None,
        retry_count: 0,
        project: Some("project-a"),
        original_input: Some("https://example.com/low.pdf"),
        http_status: Some(200),
        duration_ms: Some(15),
        title: Some("Low"),
        authors: Some("Author"),
        doi: None,
        topics: None,
        parse_confidence: Some("low"),
        parse_confidence_factors: Some(
            r#"{"has_authors":false,"has_year":true,"has_title":false,"author_count":0}"#,
        ),
    };
    queue
        .log_download_attempt(&low_attempt)
        .await
        .expect("low-confidence row insert should succeed");

    let legacy_attempt = NewDownloadAttempt {
        url: "https://example.com/legacy.pdf",
        final_url: None,
        status: DownloadAttemptStatus::Success,
        file_path: None,
        file_size: None,
        content_type: Some("application/pdf"),
        error_message: None,
        error_type: None,
        retry_count: 0,
        project: Some("project-a"),
        original_input: Some("https://example.com/legacy.pdf"),
        http_status: Some(200),
        duration_ms: Some(12),
        title: Some("Legacy"),
        authors: None,
        doi: None,
        topics: None,
        parse_confidence: None,
        parse_confidence_factors: None,
    };
    queue
        .log_download_attempt(&legacy_attempt)
        .await
        .expect("legacy row insert should succeed");

    let low_has_year_count: i64 = sqlx::query(
        "SELECT COUNT(*) AS count
         FROM download_log
         WHERE json_extract(parse_confidence_factors, '$.has_year') = 1",
    )
    .fetch_one(db_for_query.pool())
    .await
    .expect("json_extract query should succeed")
    .try_get("count")
    .expect("count column should exist");

    let low_author_count_rows: i64 = sqlx::query(
        "SELECT COUNT(*) AS count
         FROM download_log
         WHERE json_extract(parse_confidence_factors, '$.author_count') = 0",
    )
    .fetch_one(db_for_query.pool())
    .await
    .expect("author_count query should succeed")
    .try_get("count")
    .expect("count column should exist");

    assert_eq!(
        low_has_year_count, 1,
        "rows with parse_confidence_factors JSON should be queryable via json_extract"
    );
    assert_eq!(
        low_author_count_rows, 1,
        "querying numeric confidence factors should match only JSON-backed rows"
    );
}

/// Regression: topics field was missing from the download_log SQL INSERT in log_download_attempt.
/// Bug: NewDownloadAttempt.topics was declared but never bound in the INSERT, so topics were
/// silently dropped and never stored in the database.
#[tokio::test]
async fn test_log_download_attempt_persists_topics_in_download_log() {
    let (db, _temp_dir) = setup_test_db().await;
    let db_for_query = db.clone(); // Clone before Queue takes ownership
    let queue = Queue::new(db);

    let topics_json = r#"["machine learning","neural networks"]"#;
    let attempt = NewDownloadAttempt {
        url: "https://example.com/ml-paper.pdf",
        final_url: None,
        status: DownloadAttemptStatus::Success,
        file_path: Some("/tmp/ml-paper.pdf"),
        file_size: Some(512),
        content_type: Some("application/pdf"),
        error_message: None,
        error_type: None,
        retry_count: 0,
        project: Some("ml-research"),
        original_input: None,
        http_status: Some(200),
        duration_ms: Some(80),
        title: Some("Deep Learning Survey"),
        authors: Some("LeCun, Y."),
        doi: None,
        topics: Some(topics_json),
        parse_confidence: None,
        parse_confidence_factors: None,
    };

    let id = queue
        .log_download_attempt(&attempt)
        .await
        .expect("log_download_attempt should succeed");

    // Verify topics were actually stored via a direct SQL query (DownloadAttempt
    // read model does not project the topics column, so we query raw).
    let stored_topics: Option<String> = sqlx::query("SELECT topics FROM download_log WHERE id = ?")
        .bind(id)
        .fetch_one(db_for_query.pool())
        .await
        .expect("direct topics query should succeed")
        .try_get("topics")
        .expect("topics column should exist");

    assert_eq!(
        stored_topics.as_deref(),
        Some(topics_json),
        "topics JSON must be persisted to download_log (regression: was silently dropped)"
    );
}

/// Regression: when topics is None, download_log.topics must be NULL (not a crash or default).
#[tokio::test]
async fn test_log_download_attempt_stores_null_when_topics_is_none() {
    let (db, _temp_dir) = setup_test_db().await;
    let db_for_query = db.clone();
    let queue = Queue::new(db);

    let attempt = NewDownloadAttempt {
        url: "https://example.com/paper.pdf",
        final_url: None,
        status: DownloadAttemptStatus::Success,
        file_path: None,
        file_size: None,
        content_type: None,
        error_message: None,
        error_type: None,
        retry_count: 0,
        project: None,
        original_input: None,
        http_status: None,
        duration_ms: None,
        title: None,
        authors: None,
        doi: None,
        topics: None,
        parse_confidence: None,
        parse_confidence_factors: None,
    };

    let id = queue
        .log_download_attempt(&attempt)
        .await
        .expect("log_download_attempt should succeed with no topics");

    let stored_topics: Option<String> = sqlx::query("SELECT topics FROM download_log WHERE id = ?")
        .bind(id)
        .fetch_one(db_for_query.pool())
        .await
        .expect("direct topics query should succeed")
        .try_get("topics")
        .expect("topics column should exist");

    assert!(
        stored_topics.is_none(),
        "topics should be NULL in download_log when NewDownloadAttempt.topics is None"
    );
}

#[tokio::test]
async fn test_query_download_attempts_filters_status_project_and_date() {
    let (db, _temp_dir) = setup_test_db().await;
    let queue = Queue::new(db);

    let success_attempt = NewDownloadAttempt {
        url: "https://example.com/success.pdf",
        final_url: None,
        status: DownloadAttemptStatus::Success,
        file_path: Some("/tmp/success.pdf"),
        file_size: Some(42),
        content_type: Some("application/pdf"),
        error_message: None,
        error_type: None,
        retry_count: 0,
        project: Some("project-a"),
        original_input: Some("https://example.com/success.pdf"),
        http_status: Some(200),
        duration_ms: Some(12),
        title: Some("Success"),
        authors: Some("Author A"),
        doi: None,
        topics: None,
        parse_confidence: Some("low"),
        parse_confidence_factors: Some(
            r#"{"has_authors":false,"has_year":true,"has_title":false,"author_count":0}"#,
        ),
    };
    queue
        .log_download_attempt(&success_attempt)
        .await
        .expect("success history row insert should succeed");

    let failed_attempt = NewDownloadAttempt {
        url: "https://other.org/failed.pdf",
        final_url: None,
        status: DownloadAttemptStatus::Failed,
        file_path: None,
        file_size: None,
        content_type: None,
        error_message: Some("HTTP 404\n  Suggestion: verify URL"),
        error_type: Some(DownloadErrorType::NotFound),
        retry_count: 2,
        project: Some("project-b"),
        original_input: Some("10.2000/fail"),
        http_status: Some(404),
        duration_ms: Some(22),
        title: Some("Failed"),
        authors: Some("Author B"),
        doi: Some("10.2000/fail"),
        topics: None,
        parse_confidence: Some("medium"),
        parse_confidence_factors: Some(
            r#"{"has_authors":true,"has_year":true,"has_title":false,"author_count":1}"#,
        ),
    };
    queue
        .log_download_attempt(&failed_attempt)
        .await
        .expect("failed history row insert should succeed");

    let mut by_status = DownloadAttemptQuery::default();
    by_status.status = Some(DownloadAttemptStatus::Failed);
    let failed_rows = queue
        .query_download_attempts(&by_status)
        .await
        .expect("status-filtered query should succeed");
    assert_eq!(failed_rows.len(), 1);
    assert_eq!(failed_rows[0].status(), DownloadAttemptStatus::Failed);
    assert_eq!(
        failed_rows[0].error_type(),
        Some(DownloadErrorType::NotFound)
    );
    assert_eq!(failed_rows[0].retry_count, 2);
    assert!(
        failed_rows[0].last_retry_at.is_some(),
        "failed row with retries should persist last_retry_at"
    );
    assert_eq!(
        failed_rows[0].original_input.as_deref(),
        Some("10.2000/fail")
    );

    let mut by_project = DownloadAttemptQuery::default();
    by_project.project = Some("project-a".to_string());
    let project_rows = queue
        .query_download_attempts(&by_project)
        .await
        .expect("project-filtered query should succeed");
    assert_eq!(project_rows.len(), 1);
    assert_eq!(project_rows[0].project.as_deref(), Some("project-a"));

    let mut by_domain = DownloadAttemptQuery::default();
    by_domain.domain = Some("example.com".to_string());
    let domain_rows = queue
        .query_download_attempts(&by_domain)
        .await
        .expect("domain-filtered query should succeed");
    assert_eq!(domain_rows.len(), 1);
    assert_eq!(domain_rows[0].url, "https://example.com/success.pdf");
    assert_eq!(domain_rows[0].parse_confidence.as_deref(), Some("low"));

    let mut by_future_date = DownloadAttemptQuery::default();
    by_future_date.since = Some("9999-01-01 00:00:00".to_string());
    let none_rows = queue
        .query_download_attempts(&by_future_date)
        .await
        .expect("date-filtered query should succeed");
    assert!(
        none_rows.is_empty(),
        "future since filter should return no rows"
    );

    let latest_id = queue
        .latest_download_attempt_id()
        .await
        .expect("latest id query should succeed")
        .expect("latest id should exist");
    let mut by_after_id = DownloadAttemptQuery::default();
    by_after_id.after_id = Some(latest_id - 1);
    let after_rows = queue
        .query_download_attempts(&by_after_id)
        .await
        .expect("after_id filtered query should succeed");
    assert_eq!(after_rows.len(), 1);
    assert_eq!(after_rows[0].id, latest_id);

    let mut uncertain_only = DownloadAttemptQuery::default();
    uncertain_only.uncertain_only = true;
    let uncertain_rows = queue
        .query_download_attempts(&uncertain_only)
        .await
        .expect("uncertain query should succeed");
    assert_eq!(uncertain_rows.len(), 1);
    assert_eq!(uncertain_rows[0].url, "https://example.com/success.pdf");

    uncertain_only.domain = Some("example.com".to_string());
    let uncertain_domain_rows = queue
        .query_download_attempts(&uncertain_only)
        .await
        .expect("uncertain + domain query should succeed");
    assert_eq!(uncertain_domain_rows.len(), 1);
    assert_eq!(
        uncertain_domain_rows[0].url,
        "https://example.com/success.pdf"
    );
}

#[tokio::test]
async fn test_query_download_attempts_domain_filter_paginates_past_non_matches() {
    let (db, _temp_dir) = setup_test_db().await;
    let queue = Queue::new(db);

    let matching_url = "https://target.example.org/match.pdf";
    let matching_attempt = NewDownloadAttempt {
        url: matching_url,
        final_url: None,
        status: DownloadAttemptStatus::Success,
        file_path: Some("/tmp/match.pdf"),
        file_size: Some(42),
        content_type: Some("application/pdf"),
        error_message: None,
        error_type: None,
        retry_count: 0,
        project: Some("project-a"),
        original_input: Some(matching_url),
        http_status: Some(200),
        duration_ms: Some(10),
        title: Some("Target"),
        authors: Some("Author"),
        doi: None,
        topics: None,
        parse_confidence: Some("low"),
        parse_confidence_factors: Some(
            r#"{"has_authors":false,"has_year":true,"has_title":false,"author_count":0}"#,
        ),
    };
    queue
        .log_download_attempt(&matching_attempt)
        .await
        .expect("matching history row insert should succeed");

    for idx in 0..10_050 {
        let url = format!("https://noise{idx}.example.net/{idx}.pdf");
        let noise_attempt = NewDownloadAttempt {
            url: &url,
            final_url: None,
            status: DownloadAttemptStatus::Success,
            file_path: None,
            file_size: None,
            content_type: Some("application/pdf"),
            error_message: None,
            error_type: None,
            retry_count: 0,
            project: Some("project-b"),
            original_input: Some(&url),
            http_status: Some(200),
            duration_ms: Some(5),
            title: None,
            authors: None,
            doi: None,
            topics: None,
            parse_confidence: None,
            parse_confidence_factors: None,
        };
        queue
            .log_download_attempt(&noise_attempt)
            .await
            .expect("noise history row insert should succeed");
    }

    let mut query = DownloadAttemptQuery::default();
    query.domain = Some("target.example.org".to_string());
    query.limit = 1;

    let rows = queue
        .query_download_attempts(&query)
        .await
        .expect("domain-filtered query should succeed");

    assert_eq!(
        rows.len(),
        1,
        "expected to find the only matching domain row"
    );
    assert_eq!(rows[0].url, matching_url);
}

#[tokio::test]
async fn test_query_download_search_candidates_filters_openable_project_and_dates() {
    let (db, _temp_dir) = setup_test_db().await;
    let queue = Queue::new(db);

    let success_openable = NewDownloadAttempt {
        url: "https://example.com/openable.pdf",
        final_url: None,
        status: DownloadAttemptStatus::Success,
        file_path: Some("/tmp/openable.pdf"),
        file_size: Some(10),
        content_type: Some("application/pdf"),
        error_message: None,
        error_type: None,
        retry_count: 0,
        project: Some("project-a"),
        original_input: Some("https://example.com/openable.pdf"),
        http_status: Some(200),
        duration_ms: Some(10),
        title: Some("Openable"),
        authors: Some("Doe, Jane"),
        doi: Some("10.1000/openable"),
        topics: None,
        parse_confidence: None,
        parse_confidence_factors: None,
    };
    queue
        .log_download_attempt(&success_openable)
        .await
        .expect("openable row should insert");

    let success_without_path = NewDownloadAttempt {
        url: "https://example.com/no-path.pdf",
        final_url: None,
        status: DownloadAttemptStatus::Success,
        file_path: None,
        file_size: None,
        content_type: Some("application/pdf"),
        error_message: None,
        error_type: None,
        retry_count: 0,
        project: Some("project-a"),
        original_input: Some("https://example.com/no-path.pdf"),
        http_status: Some(200),
        duration_ms: Some(10),
        title: Some("No Path"),
        authors: Some("Doe, Alex"),
        doi: None,
        topics: None,
        parse_confidence: None,
        parse_confidence_factors: None,
    };
    queue
        .log_download_attempt(&success_without_path)
        .await
        .expect("no-path row should insert");

    let failed_row = NewDownloadAttempt {
        url: "https://example.com/failed.pdf",
        final_url: None,
        status: DownloadAttemptStatus::Failed,
        file_path: None,
        file_size: None,
        content_type: None,
        error_message: Some("HTTP 404\n  Suggestion: verify URL"),
        error_type: Some(DownloadErrorType::NotFound),
        retry_count: 0,
        project: Some("project-a"),
        original_input: Some("10.1000/fail"),
        http_status: Some(404),
        duration_ms: Some(12),
        title: Some("Failed"),
        authors: Some("Doe, Pat"),
        doi: Some("10.1000/fail"),
        topics: None,
        parse_confidence: None,
        parse_confidence_factors: None,
    };
    queue
        .log_download_attempt(&failed_row)
        .await
        .expect("failed row should insert");

    let other_project = NewDownloadAttempt {
        url: "https://other.org/openable.pdf",
        final_url: None,
        status: DownloadAttemptStatus::Success,
        file_path: Some("/tmp/other-openable.pdf"),
        file_size: Some(10),
        content_type: Some("application/pdf"),
        error_message: None,
        error_type: None,
        retry_count: 0,
        project: Some("project-b"),
        original_input: Some("https://other.org/openable.pdf"),
        http_status: Some(200),
        duration_ms: Some(10),
        title: Some("Other Project"),
        authors: Some("Roe, Sam"),
        doi: None,
        topics: None,
        parse_confidence: None,
        parse_confidence_factors: None,
    };
    queue
        .log_download_attempt(&other_project)
        .await
        .expect("other project row should insert");

    let mut query = DownloadSearchQuery {
        project: Some("project-a".to_string()),
        openable_only: true,
        limit: 100,
        ..DownloadSearchQuery::default()
    };

    let openable_rows = queue
        .query_download_search_candidates(&query)
        .await
        .expect("search query should succeed");
    assert_eq!(openable_rows.len(), 1);
    assert_eq!(openable_rows[0].url, "https://example.com/openable.pdf");
    assert_eq!(openable_rows[0].status(), DownloadAttemptStatus::Success);

    query.openable_only = false;
    let all_project_rows = queue
        .query_download_search_candidates(&query)
        .await
        .expect("search query should succeed when openable_only=false");
    assert_eq!(all_project_rows.len(), 3);

    query.since = Some("9999-01-01 00:00:00".to_string());
    let none_rows = queue
        .query_download_search_candidates(&query)
        .await
        .expect("search query should support date bounds");
    assert!(none_rows.is_empty());
}

#[tokio::test]
async fn test_query_download_search_candidates_orders_by_recency_desc() {
    let (db, _temp_dir) = setup_test_db().await;
    let queue = Queue::new(db);

    for idx in 0..3 {
        let title = format!("Result {idx}");
        let url = format!("https://example.com/{idx}.pdf");
        let path = format!("/tmp/{idx}.pdf");
        let attempt = NewDownloadAttempt {
            url: &url,
            final_url: None,
            status: DownloadAttemptStatus::Success,
            file_path: Some(&path),
            file_size: Some(10),
            content_type: Some("application/pdf"),
            error_message: None,
            error_type: None,
            retry_count: 0,
            project: Some("project-a"),
            original_input: Some(&url),
            http_status: Some(200),
            duration_ms: Some(10),
            title: Some(&title),
            authors: Some("Doe, Jane"),
            doi: None,
            topics: None,
            parse_confidence: None,
            parse_confidence_factors: None,
        };
        queue
            .log_download_attempt(&attempt)
            .await
            .expect("history row insert should succeed");
    }

    let query = DownloadSearchQuery {
        project: Some("project-a".to_string()),
        openable_only: true,
        limit: 2,
        ..DownloadSearchQuery::default()
    };
    let rows = queue
        .query_download_search_candidates(&query)
        .await
        .expect("search query should succeed");

    assert_eq!(rows.len(), 2);
    assert!(rows[0].id > rows[1].id, "rows should be newest first");
}

#[tokio::test]
async fn test_query_download_search_candidates_since_until_bounds_are_inclusive() {
    let (db, _temp_dir) = setup_test_db().await;
    let db_for_update = db.clone();
    let queue = Queue::new(db);

    let first_id = queue
        .log_download_attempt(&NewDownloadAttempt {
            url: "https://example.com/first.pdf",
            final_url: None,
            status: DownloadAttemptStatus::Success,
            file_path: Some("/tmp/first.pdf"),
            file_size: Some(10),
            content_type: Some("application/pdf"),
            error_message: None,
            error_type: None,
            retry_count: 0,
            project: Some("project-a"),
            original_input: Some("https://example.com/first.pdf"),
            http_status: Some(200),
            duration_ms: Some(10),
            title: Some("First"),
            authors: Some("Doe, One"),
            doi: None,
            topics: None,
            parse_confidence: None,
            parse_confidence_factors: None,
        })
        .await
        .expect("first row insert should succeed");

    let middle_id = queue
        .log_download_attempt(&NewDownloadAttempt {
            url: "https://example.com/middle.pdf",
            final_url: None,
            status: DownloadAttemptStatus::Success,
            file_path: Some("/tmp/middle.pdf"),
            file_size: Some(10),
            content_type: Some("application/pdf"),
            error_message: None,
            error_type: None,
            retry_count: 0,
            project: Some("project-a"),
            original_input: Some("https://example.com/middle.pdf"),
            http_status: Some(200),
            duration_ms: Some(10),
            title: Some("Middle"),
            authors: Some("Doe, Two"),
            doi: None,
            topics: None,
            parse_confidence: None,
            parse_confidence_factors: None,
        })
        .await
        .expect("middle row insert should succeed");

    let last_id = queue
        .log_download_attempt(&NewDownloadAttempt {
            url: "https://example.com/last.pdf",
            final_url: None,
            status: DownloadAttemptStatus::Success,
            file_path: Some("/tmp/last.pdf"),
            file_size: Some(10),
            content_type: Some("application/pdf"),
            error_message: None,
            error_type: None,
            retry_count: 0,
            project: Some("project-a"),
            original_input: Some("https://example.com/last.pdf"),
            http_status: Some(200),
            duration_ms: Some(10),
            title: Some("Last"),
            authors: Some("Doe, Three"),
            doi: None,
            topics: None,
            parse_confidence: None,
            parse_confidence_factors: None,
        })
        .await
        .expect("last row insert should succeed");

    for (id, started_at) in [
        (first_id, "2026-01-01 00:00:00"),
        (middle_id, "2026-01-02 00:00:00"),
        (last_id, "2026-01-03 00:00:00"),
    ] {
        sqlx::query("UPDATE download_log SET started_at = ? WHERE id = ?")
            .bind(started_at)
            .bind(id)
            .execute(db_for_update.pool())
            .await
            .expect("started_at update should succeed");
    }

    let mut query = DownloadSearchQuery {
        project: Some("project-a".to_string()),
        openable_only: true,
        limit: 100,
        ..DownloadSearchQuery::default()
    };

    query.since = Some("2026-01-02 00:00:00".to_string());
    query.until = Some("2026-01-02 00:00:00".to_string());
    let exact_rows = queue
        .query_download_search_candidates(&query)
        .await
        .expect("search query with exact bounds should succeed");
    assert_eq!(exact_rows.len(), 1);
    assert_eq!(exact_rows[0].id, middle_id);

    query.since = Some("2026-01-01 00:00:00".to_string());
    query.until = Some("2026-01-02 00:00:00".to_string());
    let inclusive_rows = queue
        .query_download_search_candidates(&query)
        .await
        .expect("search query with inclusive bounds should succeed");
    assert_eq!(inclusive_rows.len(), 2);
    assert_eq!(inclusive_rows[0].id, middle_id);
    assert_eq!(inclusive_rows[1].id, first_id);
}
