//! Error types for queue operations.

use thiserror::Error;

/// Errors that can occur during queue operations.
#[derive(Debug, Clone, Error)]
pub enum QueueError {
    /// Database operation failed.
    #[error("database error: {0}")]
    Database(String),

    /// Queue item not found.
    #[error(
        "queue item not found: id {0}\n  Suggestion: The item may have been deleted or the ID is incorrect"
    )]
    ItemNotFound(i64),

    /// Invalid status transition or value.
    /// Reserved for future validation in Story 1.5+ when status transitions are enforced.
    #[allow(dead_code)] // Used in tests, will be used when status transition validation is added
    #[error(
        "invalid status '{status}': {reason}\n  Suggestion: Use one of: pending, in_progress, completed, failed"
    )]
    InvalidStatus {
        /// The invalid status value
        status: String,
        /// Why it's invalid
        reason: String,
    },
}

impl From<sqlx::Error> for QueueError {
    fn from(err: sqlx::Error) -> Self {
        Self::Database(err.to_string())
    }
}

impl QueueError {
    /// Creates an `InvalidStatus` error for an unrecognized status string.
    #[must_use]
    pub fn invalid_status(status: &str) -> Self {
        Self::InvalidStatus {
            status: status.to_string(),
            reason: "unrecognized status value".to_string(),
        }
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn test_queue_error_database_message() {
        let err = QueueError::Database("connection failed".to_string());
        let msg = err.to_string();
        assert!(msg.contains("database error"));
        assert!(msg.contains("connection failed"));
    }

    #[test]
    fn test_queue_error_item_not_found_message() {
        let err = QueueError::ItemNotFound(42);
        let msg = err.to_string();
        assert!(msg.contains("not found"));
        assert!(msg.contains("42"));
        assert!(msg.contains("Suggestion"));
    }

    #[test]
    fn test_queue_error_invalid_status_message() {
        let err = QueueError::invalid_status("unknown");
        let msg = err.to_string();
        assert!(msg.contains("invalid status"));
        assert!(msg.contains("unknown"));
        assert!(msg.contains("pending"));
    }

    #[test]
    fn test_queue_error_clone() {
        let err = QueueError::ItemNotFound(123);
        let cloned = err.clone();
        assert_eq!(err.to_string(), cloned.to_string());
    }
}
