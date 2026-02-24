//! Queue item types and status definitions.

use std::fmt;

use serde::{Deserialize, Serialize};
use sqlx::FromRow;

/// Status of a queue item.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum QueueStatus {
    /// Waiting to be processed.
    Pending,
    /// Currently being downloaded.
    InProgress,
    /// Successfully downloaded.
    Completed,
    /// Failed after all retries exhausted.
    Failed,
}

impl QueueStatus {
    /// Returns the database string representation.
    #[must_use]
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::InProgress => "in_progress",
            Self::Completed => "completed",
            Self::Failed => "failed",
        }
    }
}

impl fmt::Display for QueueStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl std::str::FromStr for QueueStatus {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "pending" => Ok(Self::Pending),
            "in_progress" => Ok(Self::InProgress),
            "completed" => Ok(Self::Completed),
            "failed" => Ok(Self::Failed),
            _ => Err(format!("invalid queue status: {s}")),
        }
    }
}

/// Optional metadata captured during URL resolution for downstream naming/indexing.
#[derive(Debug, Clone, Default)]
pub struct QueueMetadata {
    /// Suggested filename (including extension) generated before download.
    pub suggested_filename: Option<String>,
    /// Resolved title metadata.
    pub title: Option<String>,
    /// Resolved authors metadata.
    pub authors: Option<String>,
    /// Resolved publication year metadata.
    pub year: Option<String>,
    /// Resolved DOI metadata.
    pub doi: Option<String>,
    /// Extracted topics from title/abstract (Story 8.1)
    pub topics: Option<Vec<String>>,
    /// Parser confidence classification for reference-derived inputs.
    pub parse_confidence: Option<String>,
    /// JSON payload for parser confidence factors.
    pub parse_confidence_factors: Option<String>,
}

/// A single item in the download queue.
#[derive(Debug, Clone, FromRow)]
pub struct QueueItem {
    /// Unique identifier.
    pub id: i64,
    /// The resolved URL to download.
    pub url: String,
    /// How this item entered the queue (`direct_url`, doi, reference, bibtex).
    pub source_type: String,
    /// Original user input before resolution (e.g., DOI string).
    pub original_input: Option<String>,
    /// Current processing status (stored as text, parsed via `status()`).
    #[sqlx(rename = "status")]
    pub status_str: String,
    /// Higher priority items processed first (default 0).
    pub priority: i64,
    /// Number of retry attempts made.
    pub retry_count: i64,
    /// Last error message if failed.
    pub last_error: Option<String>,
    /// Pre-computed preferred filename for this item.
    pub suggested_filename: Option<String>,
    /// Metadata title captured at enqueue time.
    pub meta_title: Option<String>,
    /// Metadata authors captured at enqueue time.
    pub meta_authors: Option<String>,
    /// Metadata year captured at enqueue time.
    pub meta_year: Option<String>,
    /// Metadata DOI captured at enqueue time.
    pub meta_doi: Option<String>,
    /// Extracted topics as JSON array (Story 8.1)
    pub topics: Option<String>,
    /// Parser confidence classification (`high`/`medium`/`low`) when present.
    pub parse_confidence: Option<String>,
    /// JSON payload of parser confidence factors when present.
    pub parse_confidence_factors: Option<String>,
    /// Final saved path when download completes.
    pub saved_path: Option<String>,
    /// Bytes currently written for this item (supports resume).
    pub bytes_downloaded: i64,
    /// Expected total bytes when known (from Content-Length).
    pub content_length: Option<i64>,
    /// When the item was created.
    pub created_at: String,
    /// When the item was last updated.
    pub updated_at: String,
}

impl QueueItem {
    /// Returns the parsed status enum.
    ///
    /// Falls back to `Pending` if the status string is invalid.
    #[must_use]
    pub fn status(&self) -> QueueStatus {
        self.status_str.parse().unwrap_or(QueueStatus::Pending)
    }

    /// Parses topics from JSON array string.
    ///
    /// Returns empty vector if topics are None or invalid JSON.
    ///
    /// # Returns
    /// Vector of topic strings from the JSON array
    #[must_use]
    pub fn parse_topics(&self) -> Vec<String> {
        let Some(topics_json) = &self.topics else {
            return Vec::new();
        };

        serde_json::from_str(topics_json).unwrap_or_default()
    }

    /// Serializes topics to JSON array string for database storage.
    ///
    /// # Arguments
    /// * `topics` - Vector of topic strings
    ///
    /// # Returns
    /// JSON array string or None if topics vector is empty
    #[must_use]
    pub fn serialize_topics(topics: &[String]) -> Option<String> {
        if topics.is_empty() {
            return None;
        }

        serde_json::to_string(topics).ok()
    }
}

impl fmt::Display for QueueItem {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "QueueItem {{ id: {}, url: {}, status: {} }}",
            self.id,
            self.url,
            self.status()
        )
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    // ==================== QueueStatus Tests ====================

    #[test]
    fn test_queue_status_as_str() {
        assert_eq!(QueueStatus::Pending.as_str(), "pending");
        assert_eq!(QueueStatus::InProgress.as_str(), "in_progress");
        assert_eq!(QueueStatus::Completed.as_str(), "completed");
        assert_eq!(QueueStatus::Failed.as_str(), "failed");
    }

    #[test]
    fn test_queue_status_display() {
        assert_eq!(QueueStatus::Pending.to_string(), "pending");
        assert_eq!(QueueStatus::InProgress.to_string(), "in_progress");
        assert_eq!(QueueStatus::Completed.to_string(), "completed");
        assert_eq!(QueueStatus::Failed.to_string(), "failed");
    }

    #[test]
    fn test_queue_status_from_str_valid() {
        assert_eq!(
            "pending".parse::<QueueStatus>().unwrap(),
            QueueStatus::Pending
        );
        assert_eq!(
            "in_progress".parse::<QueueStatus>().unwrap(),
            QueueStatus::InProgress
        );
        assert_eq!(
            "completed".parse::<QueueStatus>().unwrap(),
            QueueStatus::Completed
        );
        assert_eq!(
            "failed".parse::<QueueStatus>().unwrap(),
            QueueStatus::Failed
        );
    }

    #[test]
    fn test_queue_status_from_str_invalid() {
        let result = "unknown".parse::<QueueStatus>();
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("invalid queue status"));
    }

    #[test]
    fn test_queue_status_serde_roundtrip() {
        let status = QueueStatus::InProgress;
        let json = serde_json::to_string(&status).unwrap();
        assert_eq!(json, "\"in_progress\"");
        let parsed: QueueStatus = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed, status);
    }

    #[test]
    fn test_queue_status_equality() {
        assert_eq!(QueueStatus::Pending, QueueStatus::Pending);
        assert_ne!(QueueStatus::Pending, QueueStatus::Failed);
    }

    #[test]
    fn test_queue_status_clone() {
        let status = QueueStatus::Completed;
        let cloned = status;
        assert_eq!(status, cloned);
    }

    // ==================== QueueItem Tests ====================

    #[test]
    fn test_queue_item_status_parses_correctly() {
        let item = QueueItem {
            id: 1,
            url: "https://example.com".to_string(),
            source_type: "direct_url".to_string(),
            original_input: None,
            status_str: "in_progress".to_string(),
            priority: 0,
            retry_count: 0,
            last_error: None,
            suggested_filename: None,
            meta_title: None,
            meta_authors: None,
            meta_year: None,
            meta_doi: None,
            topics: None,
            parse_confidence: None,
            parse_confidence_factors: None,
            saved_path: None,
            bytes_downloaded: 0,
            content_length: None,
            created_at: "2026-01-01".to_string(),
            updated_at: "2026-01-01".to_string(),
        };

        assert_eq!(item.status(), QueueStatus::InProgress);
    }

    #[test]
    fn test_queue_item_status_fallback_on_invalid() {
        let item = QueueItem {
            id: 1,
            url: "https://example.com".to_string(),
            source_type: "direct_url".to_string(),
            original_input: None,
            status_str: "garbage".to_string(),
            priority: 0,
            retry_count: 0,
            last_error: None,
            suggested_filename: None,
            meta_title: None,
            meta_authors: None,
            meta_year: None,
            meta_doi: None,
            topics: None,
            parse_confidence: None,
            parse_confidence_factors: None,
            saved_path: None,
            bytes_downloaded: 0,
            content_length: None,
            created_at: "2026-01-01".to_string(),
            updated_at: "2026-01-01".to_string(),
        };

        assert_eq!(item.status(), QueueStatus::Pending);
    }

    #[test]
    fn test_queue_item_display() {
        let item = QueueItem {
            id: 42,
            url: "https://example.com/file.pdf".to_string(),
            source_type: "direct_url".to_string(),
            original_input: None,
            status_str: "pending".to_string(),
            priority: 0,
            retry_count: 0,
            last_error: None,
            suggested_filename: None,
            meta_title: None,
            meta_authors: None,
            meta_year: None,
            meta_doi: None,
            topics: None,
            parse_confidence: None,
            parse_confidence_factors: None,
            saved_path: None,
            bytes_downloaded: 0,
            content_length: None,
            created_at: "2026-01-01".to_string(),
            updated_at: "2026-01-01".to_string(),
        };

        let display = item.to_string();
        assert!(display.contains("42"));
        assert!(display.contains("example.com"));
        assert!(display.contains("pending"));
    }

    // ==================== Topic Serialization Tests ====================

    #[test]
    fn test_serialize_topics_empty_returns_none() {
        let result = QueueItem::serialize_topics(&[]);
        assert!(result.is_none());
    }

    #[test]
    fn test_serialize_topics_returns_json_array() {
        let topics = vec!["machine learning".to_string(), "climate change".to_string()];
        let result = QueueItem::serialize_topics(&topics).unwrap();
        assert_eq!(result, r#"["machine learning","climate change"]"#);
    }

    #[test]
    fn test_parse_topics_none_returns_empty() {
        let item = QueueItem {
            id: 1,
            url: "https://example.com".to_string(),
            source_type: "direct_url".to_string(),
            original_input: None,
            status_str: "pending".to_string(),
            priority: 0,
            retry_count: 0,
            last_error: None,
            suggested_filename: None,
            meta_title: None,
            meta_authors: None,
            meta_year: None,
            meta_doi: None,
            topics: None,
            parse_confidence: None,
            parse_confidence_factors: None,
            saved_path: None,
            bytes_downloaded: 0,
            content_length: None,
            created_at: "2026-01-01".to_string(),
            updated_at: "2026-01-01".to_string(),
        };
        assert!(item.parse_topics().is_empty());
    }

    #[test]
    fn test_parse_topics_roundtrip() {
        let original = vec!["machine learning".to_string(), "climate change".to_string()];
        let json = QueueItem::serialize_topics(&original).unwrap();
        let item = QueueItem {
            id: 1,
            url: "https://example.com".to_string(),
            source_type: "direct_url".to_string(),
            original_input: None,
            status_str: "pending".to_string(),
            priority: 0,
            retry_count: 0,
            last_error: None,
            suggested_filename: None,
            meta_title: None,
            meta_authors: None,
            meta_year: None,
            meta_doi: None,
            topics: Some(json),
            parse_confidence: None,
            parse_confidence_factors: None,
            saved_path: None,
            bytes_downloaded: 0,
            content_length: None,
            created_at: "2026-01-01".to_string(),
            updated_at: "2026-01-01".to_string(),
        };
        assert_eq!(item.parse_topics(), original);
    }

    #[test]
    fn test_parse_topics_invalid_json_returns_empty() {
        let item = QueueItem {
            id: 1,
            url: "https://example.com".to_string(),
            source_type: "direct_url".to_string(),
            original_input: None,
            status_str: "pending".to_string(),
            priority: 0,
            retry_count: 0,
            last_error: None,
            suggested_filename: None,
            meta_title: None,
            meta_authors: None,
            meta_year: None,
            meta_doi: None,
            topics: Some("not json".to_string()),
            parse_confidence: None,
            parse_confidence_factors: None,
            saved_path: None,
            bytes_downloaded: 0,
            content_length: None,
            created_at: "2026-01-01".to_string(),
            updated_at: "2026-01-01".to_string(),
        };
        assert!(item.parse_topics().is_empty());
    }
}
