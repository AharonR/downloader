//! Download history persistence and query helpers.
//!
//! Story 6.1 introduces download-attempt logging as a first-class queue-adjacent
//! capability because queue processing is the source of truth for terminal states.

use std::fmt;

use sqlx::FromRow;
use tracing::instrument;
use url::Url;

use super::{Queue, Result};

const DEFAULT_HISTORY_LIMIT: usize = 200;
const MAX_HISTORY_LIMIT: usize = 10_000;

/// Persisted status for a download attempt history row.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DownloadAttemptStatus {
    /// Download finished successfully.
    Success,
    /// Download failed after processing.
    Failed,
    /// Download was skipped by workflow logic.
    Skipped,
}

impl DownloadAttemptStatus {
    /// Returns the storage representation.
    #[must_use]
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Success => "success",
            Self::Failed => "failed",
            Self::Skipped => "skipped",
        }
    }
}

impl fmt::Display for DownloadAttemptStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl std::str::FromStr for DownloadAttemptStatus {
    type Err = String;

    fn from_str(value: &str) -> std::result::Result<Self, Self::Err> {
        match value {
            "success" => Ok(Self::Success),
            "failed" => Ok(Self::Failed),
            "skipped" => Ok(Self::Skipped),
            _ => Err(format!("invalid download attempt status: {value}")),
        }
    }
}

/// Persisted failure category for failed download attempts.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DownloadErrorType {
    /// Network/transport, timeout, HTTP-server, or IO-like failures.
    Network,
    /// Authentication/authorization failures.
    Auth,
    /// Resource could not be found.
    NotFound,
    /// Input parsing/validation failures.
    ParseError,
}

impl DownloadErrorType {
    /// Returns the storage representation.
    #[must_use]
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Network => "network",
            Self::Auth => "auth",
            Self::NotFound => "not_found",
            Self::ParseError => "parse_error",
        }
    }
}

impl fmt::Display for DownloadErrorType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl std::str::FromStr for DownloadErrorType {
    type Err = String;

    fn from_str(value: &str) -> std::result::Result<Self, Self::Err> {
        match value {
            "network" => Ok(Self::Network),
            "auth" => Ok(Self::Auth),
            "not_found" => Ok(Self::NotFound),
            "parse_error" => Ok(Self::ParseError),
            _ => Err(format!("invalid download error type: {value}")),
        }
    }
}

/// Insert payload for a single persisted download attempt.
#[derive(Debug, Clone)]
pub struct NewDownloadAttempt<'a> {
    /// Requested URL.
    pub url: &'a str,
    /// Final URL after redirects when known.
    pub final_url: Option<&'a str>,
    /// Attempt status.
    pub status: DownloadAttemptStatus,
    /// Saved file path for successful attempts.
    pub file_path: Option<&'a str>,
    /// Saved file size in bytes.
    pub file_size: Option<i64>,
    /// Response content-type if captured.
    pub content_type: Option<&'a str>,
    /// Error text for failures.
    pub error_message: Option<&'a str>,
    /// Categorized failure type.
    pub error_type: Option<DownloadErrorType>,
    /// Number of retries used before terminal state.
    pub retry_count: i64,
    /// Project key/path associated with this attempt.
    pub project: Option<&'a str>,
    /// Original user input that produced this attempt.
    pub original_input: Option<&'a str>,
    /// HTTP status code when available.
    pub http_status: Option<i64>,
    /// Duration in milliseconds.
    pub duration_ms: Option<i64>,
    /// Metadata title when available.
    pub title: Option<&'a str>,
    /// Metadata authors when available.
    pub authors: Option<&'a str>,
    /// Metadata DOI when available.
    pub doi: Option<&'a str>,
    /// JSON-encoded topic list when available.
    pub topics: Option<&'a str>,
    /// Parser confidence classification for reference-derived inputs.
    pub parse_confidence: Option<&'a str>,
    /// JSON payload of parser confidence factors.
    pub parse_confidence_factors: Option<&'a str>,
}

/// Query filters for download history reads.
#[derive(Debug, Clone)]
pub struct DownloadAttemptQuery {
    /// Optional lower timestamp bound (`started_at >= since`).
    pub since: Option<String>,
    /// Optional upper timestamp bound (`started_at <= until`).
    pub until: Option<String>,
    /// Optional status filter.
    pub status: Option<DownloadAttemptStatus>,
    /// Optional project filter.
    pub project: Option<String>,
    /// Optional row-id lower bound (`id > after_id`).
    pub after_id: Option<i64>,
    /// Optional row-id upper bound (`id < before_id`) for pagination.
    pub before_id: Option<i64>,
    /// Optional domain filter (case-insensitive host match).
    pub domain: Option<String>,
    /// Restrict rows to low-confidence reference parses.
    pub uncertain_only: bool,
    /// Max rows to return (0 uses default).
    pub limit: usize,
}

impl Default for DownloadAttemptQuery {
    fn default() -> Self {
        Self {
            since: None,
            until: None,
            status: None,
            project: None,
            after_id: None,
            before_id: None,
            domain: None,
            uncertain_only: false,
            limit: DEFAULT_HISTORY_LIMIT,
        }
    }
}

/// Read model for persisted download attempt rows.
#[derive(Debug, Clone, FromRow)]
pub struct DownloadAttempt {
    /// Row id.
    pub id: i64,
    /// Requested URL.
    pub url: String,
    /// Stored status text.
    #[sqlx(rename = "status")]
    pub status_str: String,
    /// Saved path for successful attempts.
    pub file_path: Option<String>,
    /// Metadata title.
    pub title: Option<String>,
    /// Metadata authors.
    pub authors: Option<String>,
    /// Metadata DOI.
    pub doi: Option<String>,
    /// Parser confidence classification when present.
    pub parse_confidence: Option<String>,
    /// JSON payload of parser confidence factors when present.
    pub parse_confidence_factors: Option<String>,
    /// Project key/path.
    pub project: Option<String>,
    /// Start timestamp in `SQLite` datetime text format.
    pub started_at: String,
    /// Failure message.
    pub error_message: Option<String>,
    /// Failure category text.
    pub error_type: Option<String>,
    /// Retry count captured at terminal state.
    pub retry_count: i64,
    /// Last retry timestamp in `SQLite` datetime text format.
    pub last_retry_at: Option<String>,
    /// Original user input before resolution.
    pub original_input: Option<String>,
    /// HTTP status when captured.
    pub http_status: Option<i64>,
    /// Duration in milliseconds.
    pub duration_ms: Option<i64>,
}

impl DownloadAttempt {
    /// Parses `status_str` into a typed status; unknown values map to `failed`.
    #[must_use]
    pub fn status(&self) -> DownloadAttemptStatus {
        self.status_str
            .parse()
            .unwrap_or(DownloadAttemptStatus::Failed)
    }

    /// Parses `error_type` into a typed category; unknown values map to `network`.
    #[must_use]
    pub fn error_type(&self) -> Option<DownloadErrorType> {
        self.error_type
            .as_deref()
            .map(|value| value.parse().unwrap_or(DownloadErrorType::Network))
    }
}

/// Query filters for search candidate reads from persisted download history.
#[derive(Debug, Clone)]
pub struct DownloadSearchQuery {
    /// Optional lower timestamp bound (`started_at >= since`).
    pub since: Option<String>,
    /// Optional upper timestamp bound (`started_at <= until`).
    pub until: Option<String>,
    /// Optional project scope key.
    pub project: Option<String>,
    /// Restrict search candidates to openable successful rows.
    pub openable_only: bool,
    /// Max candidates to return (0 uses default).
    pub limit: usize,
}

impl Default for DownloadSearchQuery {
    fn default() -> Self {
        Self {
            since: None,
            until: None,
            project: None,
            openable_only: true,
            limit: DEFAULT_HISTORY_LIMIT,
        }
    }
}

/// Read model for search candidates from persisted history.
#[derive(Debug, Clone, FromRow)]
pub struct DownloadSearchCandidate {
    /// Row id.
    pub id: i64,
    /// Requested URL.
    pub url: String,
    /// Stored status text.
    #[sqlx(rename = "status")]
    pub status_str: String,
    /// Saved path for successful attempts.
    pub file_path: Option<String>,
    /// Metadata title.
    pub title: Option<String>,
    /// Metadata authors.
    pub authors: Option<String>,
    /// Metadata DOI.
    pub doi: Option<String>,
    /// Start timestamp in `SQLite` datetime text format.
    pub started_at: String,
}

impl DownloadSearchCandidate {
    /// Parses `status_str` into a typed status; unknown values map to `failed`.
    #[must_use]
    pub fn status(&self) -> DownloadAttemptStatus {
        self.status_str
            .parse()
            .unwrap_or(DownloadAttemptStatus::Failed)
    }
}

impl Queue {
    /// Returns latest persisted download history row id.
    ///
    /// # Errors
    ///
    /// Returns database errors when query execution fails.
    #[instrument(skip(self))]
    pub async fn latest_download_attempt_id(&self) -> Result<Option<i64>> {
        let latest = sqlx::query_scalar::<_, Option<i64>>(r"SELECT MAX(id) FROM download_log")
            .fetch_one(self.db.pool())
            .await?;
        Ok(latest)
    }

    /// Persists one terminal download attempt row.
    ///
    /// # Errors
    ///
    /// Returns database errors when insert fails.
    #[instrument(skip(self, attempt), fields(url = %attempt.url, status = %attempt.status))]
    pub async fn log_download_attempt(&self, attempt: &NewDownloadAttempt<'_>) -> Result<i64> {
        let row = sqlx::query(
            r"INSERT INTO download_log (
                url,
                final_url,
                status,
                file_path,
                file_size,
                content_type,
                started_at,
                completed_at,
                error_message,
                error_type,
                retry_count,
                last_retry_at,
                project,
                original_input,
                http_status,
                duration_ms,
                title,
                authors,
                doi,
                topics,
                parse_confidence,
                parse_confidence_factors
              )
              VALUES (
                ?, ?, ?, ?, ?, ?, datetime('now'), datetime('now'),
                ?, ?, ?,
                CASE WHEN ? = 'failed' AND ? > 0 THEN datetime('now') ELSE NULL END,
                ?, ?, ?, ?, ?, ?, ?, ?, ?, ?
              )
              RETURNING id",
        )
        .bind(attempt.url)
        .bind(attempt.final_url)
        .bind(attempt.status.as_str())
        .bind(attempt.file_path)
        .bind(attempt.file_size)
        .bind(attempt.content_type)
        .bind(attempt.error_message)
        .bind(attempt.error_type.map(|error_type| error_type.as_str()))
        .bind(attempt.retry_count)
        .bind(attempt.status.as_str())
        .bind(attempt.retry_count)
        .bind(attempt.project)
        .bind(attempt.original_input)
        .bind(attempt.http_status)
        .bind(attempt.duration_ms)
        .bind(attempt.title)
        .bind(attempt.authors)
        .bind(attempt.doi)
        .bind(attempt.topics)
        .bind(attempt.parse_confidence)
        .bind(attempt.parse_confidence_factors)
        .fetch_one(self.db.pool())
        .await?;

        Ok(sqlx::Row::get(&row, "id"))
    }

    /// Queries persisted download attempts by optional date/status/project filters.
    ///
    /// # Errors
    ///
    /// Returns database errors when query execution fails.
    #[instrument(skip(self, query))]
    pub async fn query_download_attempts(
        &self,
        query: &DownloadAttemptQuery,
    ) -> Result<Vec<DownloadAttempt>> {
        let requested_limit = normalize_history_limit(query.limit);
        let normalized_domain = query
            .domain
            .as_deref()
            .map(normalize_domain_filter)
            .filter(|value| !value.is_empty());

        // Domain filtering is host-aware and handled in Rust. Page through results so
        // we never apply SQL LIMIT before domain matching.
        if let Some(domain) = normalized_domain {
            let mut matched = Vec::new();
            let mut cursor_before = query.before_id;
            let page_size = normalize_history_limit(MAX_HISTORY_LIMIT);
            let requested_limit_usize =
                usize::try_from(requested_limit).unwrap_or(MAX_HISTORY_LIMIT);

            loop {
                let page = query_download_attempts_page(
                    self,
                    query.status,
                    query.project.as_deref(),
                    query.since.as_deref(),
                    query.until.as_deref(),
                    query.after_id,
                    cursor_before,
                    query.uncertain_only,
                    page_size,
                )
                .await?;

                if page.is_empty() {
                    break;
                }

                cursor_before = page.last().map(|attempt| attempt.id);
                matched.extend(
                    page.into_iter()
                        .filter(|attempt| url_matches_domain(&attempt.url, &domain)),
                );

                if matched.len() >= requested_limit_usize {
                    break;
                }
            }

            if matched.len() > requested_limit_usize {
                matched.truncate(requested_limit_usize);
            }

            return Ok(matched);
        }

        let mut attempts = query_download_attempts_page(
            self,
            query.status,
            query.project.as_deref(),
            query.since.as_deref(),
            query.until.as_deref(),
            query.after_id,
            query.before_id,
            query.uncertain_only,
            requested_limit,
        )
        .await?;

        let requested_limit_usize = usize::try_from(requested_limit).unwrap_or(MAX_HISTORY_LIMIT);
        if attempts.len() > requested_limit_usize {
            attempts.truncate(requested_limit_usize);
        }

        Ok(attempts)
    }

    /// Queries persisted history rows as search candidates.
    ///
    /// # Errors
    ///
    /// Returns database errors when query execution fails.
    #[instrument(skip(self, query))]
    pub async fn query_download_search_candidates(
        &self,
        query: &DownloadSearchQuery,
    ) -> Result<Vec<DownloadSearchCandidate>> {
        let requested_limit = normalize_history_limit(query.limit);
        let candidates = sqlx::query_as::<_, DownloadSearchCandidate>(
            r"SELECT
                id,
                url,
                status,
                file_path,
                title,
                authors,
                doi,
                started_at
              FROM download_log
              WHERE (?1 IS NULL OR project = ?1)
                AND (?2 IS NULL OR started_at >= ?2)
                AND (?3 IS NULL OR started_at <= ?3)
                AND (title IS NOT NULL OR authors IS NOT NULL OR doi IS NOT NULL)
                AND (?4 = 0 OR (status = 'success' AND file_path IS NOT NULL))
              ORDER BY started_at DESC, id DESC
              LIMIT ?5",
        )
        .bind(query.project.as_deref())
        .bind(query.since.as_deref())
        .bind(query.until.as_deref())
        .bind(i64::from(query.openable_only))
        .bind(requested_limit)
        .fetch_all(self.db.pool())
        .await?;

        Ok(candidates)
    }
}

#[allow(clippy::too_many_arguments)]
async fn query_download_attempts_page(
    queue: &Queue,
    status: Option<DownloadAttemptStatus>,
    project: Option<&str>,
    since: Option<&str>,
    until: Option<&str>,
    after_id: Option<i64>,
    before_id: Option<i64>,
    uncertain_only: bool,
    limit: i64,
) -> Result<Vec<DownloadAttempt>> {
    let attempts = if uncertain_only {
        sqlx::query_as::<_, DownloadAttempt>(
            r"SELECT
                id,
                url,
                status,
                file_path,
                title,
                authors,
                doi,
                parse_confidence,
                parse_confidence_factors,
                project,
                started_at,
                error_message,
                error_type,
                retry_count,
                last_retry_at,
                original_input,
                http_status,
                duration_ms
              FROM download_log
              WHERE (?1 IS NULL OR status = ?1)
                AND (?2 IS NULL OR project = ?2)
                AND (?3 IS NULL OR started_at >= ?3)
                AND (?4 IS NULL OR started_at <= ?4)
                AND (?5 IS NULL OR id > ?5)
                AND (?6 IS NULL OR id < ?6)
                AND parse_confidence = 'low'
              ORDER BY id DESC
              LIMIT ?7",
        )
        .bind(status.map(|value| value.as_str()))
        .bind(project)
        .bind(since)
        .bind(until)
        .bind(after_id)
        .bind(before_id)
        .bind(limit)
        .fetch_all(queue.db.pool())
        .await?
    } else {
        sqlx::query_as::<_, DownloadAttempt>(
            r"SELECT
                id,
                url,
                status,
                file_path,
                title,
                authors,
                doi,
                parse_confidence,
                parse_confidence_factors,
                project,
                started_at,
                error_message,
                error_type,
                retry_count,
                last_retry_at,
                original_input,
                http_status,
                duration_ms
              FROM download_log
              WHERE (?1 IS NULL OR status = ?1)
                AND (?2 IS NULL OR project = ?2)
                AND (?3 IS NULL OR started_at >= ?3)
                AND (?4 IS NULL OR started_at <= ?4)
                AND (?5 IS NULL OR id > ?5)
                AND (?6 IS NULL OR id < ?6)
              ORDER BY id DESC
              LIMIT ?7",
        )
        .bind(status.map(|value| value.as_str()))
        .bind(project)
        .bind(since)
        .bind(until)
        .bind(after_id)
        .bind(before_id)
        .bind(limit)
        .fetch_all(queue.db.pool())
        .await?
    };

    Ok(attempts)
}

fn normalize_history_limit(limit: usize) -> i64 {
    let clamped = if limit == 0 {
        DEFAULT_HISTORY_LIMIT
    } else {
        limit.min(MAX_HISTORY_LIMIT)
    };
    i64::try_from(clamped).unwrap_or(i64::MAX)
}

fn normalize_domain_filter(raw: &str) -> String {
    raw.trim()
        .trim_start_matches('.')
        .trim()
        .to_ascii_lowercase()
}

fn url_matches_domain(url: &str, domain: &str) -> bool {
    if domain.is_empty() {
        return true;
    }

    if let Ok(parsed) = Url::parse(url)
        && let Some(host) = parsed.host_str()
    {
        let normalized_host = host.to_ascii_lowercase();
        return normalized_host == domain || normalized_host.ends_with(&format!(".{domain}"));
    }

    url.to_ascii_lowercase().contains(domain)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_download_attempt_status_as_str() {
        assert_eq!(DownloadAttemptStatus::Success.as_str(), "success");
        assert_eq!(DownloadAttemptStatus::Failed.as_str(), "failed");
        assert_eq!(DownloadAttemptStatus::Skipped.as_str(), "skipped");
    }

    #[test]
    fn test_download_attempt_status_from_str() {
        assert_eq!(
            "success".parse::<DownloadAttemptStatus>().ok(),
            Some(DownloadAttemptStatus::Success)
        );
        assert_eq!(
            "failed".parse::<DownloadAttemptStatus>().ok(),
            Some(DownloadAttemptStatus::Failed)
        );
        assert_eq!(
            "skipped".parse::<DownloadAttemptStatus>().ok(),
            Some(DownloadAttemptStatus::Skipped)
        );
        assert!("unknown".parse::<DownloadAttemptStatus>().is_err());
    }

    #[test]
    fn test_download_error_type_as_str() {
        assert_eq!(DownloadErrorType::Network.as_str(), "network");
        assert_eq!(DownloadErrorType::Auth.as_str(), "auth");
        assert_eq!(DownloadErrorType::NotFound.as_str(), "not_found");
        assert_eq!(DownloadErrorType::ParseError.as_str(), "parse_error");
    }

    #[test]
    fn test_download_error_type_from_str() {
        assert_eq!(
            "network".parse::<DownloadErrorType>().ok(),
            Some(DownloadErrorType::Network)
        );
        assert_eq!(
            "auth".parse::<DownloadErrorType>().ok(),
            Some(DownloadErrorType::Auth)
        );
        assert_eq!(
            "not_found".parse::<DownloadErrorType>().ok(),
            Some(DownloadErrorType::NotFound)
        );
        assert_eq!(
            "parse_error".parse::<DownloadErrorType>().ok(),
            Some(DownloadErrorType::ParseError)
        );
        assert!("other".parse::<DownloadErrorType>().is_err());
    }

    #[test]
    fn test_normalize_history_limit_defaults_for_zero() {
        assert_eq!(normalize_history_limit(0), 200);
    }

    #[test]
    fn test_normalize_history_limit_clamps_max() {
        assert_eq!(normalize_history_limit(20_000), 10_000);
    }

    #[test]
    fn test_normalize_domain_filter() {
        assert_eq!(normalize_domain_filter(" Example.COM "), "example.com");
        assert_eq!(normalize_domain_filter(".science.org"), "science.org");
    }

    #[test]
    fn test_url_matches_domain_exact_and_subdomain() {
        assert!(url_matches_domain(
            "https://science.org/paper.pdf",
            "science.org"
        ));
        assert!(url_matches_domain(
            "https://cdn.science.org/paper.pdf",
            "science.org"
        ));
        assert!(!url_matches_domain(
            "https://othersite.org/paper.pdf",
            "science.org"
        ));
    }
}
