//! Error types for queue operations.

use std::fmt;

use thiserror::Error;

/// Structured classification for queue/database failures.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QueueDbErrorKind {
    /// `SQLite` returned busy/locked under concurrent access.
    BusyOrLocked,
    /// Constraint failure (unique/foreign-key/check/not-null).
    ConstraintViolation,
    /// Connection pool timed out waiting for a free connection.
    PoolTimeout,
    /// Connection pool is closed.
    PoolClosed,
    /// Expected row was not found.
    RowNotFound,
    /// Filesystem or transport IO failure.
    Io,
    /// SQL protocol/driver error.
    Protocol,
    /// Unclassified database failure.
    Other,
}

impl QueueDbErrorKind {
    #[must_use]
    pub fn from_sqlx(error: &sqlx::Error) -> Self {
        match error {
            sqlx::Error::PoolTimedOut => Self::PoolTimeout,
            sqlx::Error::PoolClosed => Self::PoolClosed,
            sqlx::Error::RowNotFound => Self::RowNotFound,
            sqlx::Error::Io(_) => Self::Io,
            sqlx::Error::Protocol(_) => Self::Protocol,
            sqlx::Error::Database(database_error) => {
                classify_database_error(database_error.as_ref())
            }
            _ => Self::Other,
        }
    }
}

impl fmt::Display for QueueDbErrorKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let label = match self {
            Self::BusyOrLocked => "busy_or_locked",
            Self::ConstraintViolation => "constraint_violation",
            Self::PoolTimeout => "pool_timeout",
            Self::PoolClosed => "pool_closed",
            Self::RowNotFound => "row_not_found",
            Self::Io => "io",
            Self::Protocol => "protocol",
            Self::Other => "other",
        };
        write!(f, "{label}")
    }
}

fn classify_database_error(
    database_error: &(dyn sqlx::error::DatabaseError + 'static),
) -> QueueDbErrorKind {
    let code = database_error.code();
    if matches!(
        code.as_deref(),
        Some("SQLITE_BUSY" | "SQLITE_LOCKED" | "5" | "6")
    ) {
        return QueueDbErrorKind::BusyOrLocked;
    }

    if database_error.is_unique_violation()
        || database_error.is_foreign_key_violation()
        || database_error.is_check_violation()
        || code
            .as_deref()
            .is_some_and(|value| value.starts_with("SQLITE_CONSTRAINT"))
    {
        return QueueDbErrorKind::ConstraintViolation;
    }

    let message = database_error.message().to_ascii_lowercase();
    if message.contains("database is locked")
        || message.contains("database table is locked")
        || message.contains("database is busy")
    {
        return QueueDbErrorKind::BusyOrLocked;
    }

    QueueDbErrorKind::Other
}

/// Errors that can occur during queue operations.
#[derive(Debug, Clone, Error)]
pub enum QueueError {
    /// Database operation failed.
    #[error("database error ({kind}): {message}")]
    Database {
        /// Typed classification used for failure handling and KPI gates.
        kind: QueueDbErrorKind,
        /// Human-readable database error text.
        message: String,
    },

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
        Self::Database {
            kind: QueueDbErrorKind::from_sqlx(&err),
            message: err.to_string(),
        }
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

    /// Returns the typed database error kind, when this is a database error.
    #[must_use]
    pub fn database_kind(&self) -> Option<QueueDbErrorKind> {
        match self {
            Self::Database { kind, .. } => Some(*kind),
            Self::ItemNotFound(_) | Self::InvalidStatus { .. } => None,
        }
    }

    /// Returns true when this error is a database busy/locked condition.
    #[must_use]
    pub fn is_busy_or_locked(&self) -> bool {
        self.database_kind() == Some(QueueDbErrorKind::BusyOrLocked)
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn test_queue_error_database_message() {
        let err = QueueError::Database {
            kind: QueueDbErrorKind::Other,
            message: "connection failed".to_string(),
        };
        let msg = err.to_string();
        assert!(msg.contains("database error"));
        assert!(msg.contains("other"));
        assert!(msg.contains("connection failed"));
    }

    #[test]
    fn test_queue_error_database_busy_flag() {
        let err = QueueError::Database {
            kind: QueueDbErrorKind::BusyOrLocked,
            message: "database is locked".to_string(),
        };
        assert_eq!(err.database_kind(), Some(QueueDbErrorKind::BusyOrLocked));
        assert!(err.is_busy_or_locked());
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
