use crate::queue::DownloadErrorType;

use super::DownloadError;

pub(super) fn extract_http_status(error: &DownloadError) -> Option<i64> {
    match error {
        DownloadError::HttpStatus { status, .. } | DownloadError::AuthRequired { status, .. } => {
            Some(i64::from(*status))
        }
        _ => None,
    }
}

pub(super) fn classify_download_error_type(error: &DownloadError) -> DownloadErrorType {
    match error {
        DownloadError::AuthRequired { .. } => DownloadErrorType::Auth,
        DownloadError::HttpStatus { status, .. } => match status {
            401 | 403 | 407 => DownloadErrorType::Auth,
            404 => DownloadErrorType::NotFound,
            _ => DownloadErrorType::Network,
        },
        DownloadError::InvalidUrl { .. } => DownloadErrorType::ParseError,
        DownloadError::Timeout { .. }
        | DownloadError::Network { .. }
        | DownloadError::Io { .. }
        | DownloadError::Integrity { .. } => DownloadErrorType::Network,
    }
}

pub(super) fn build_actionable_error_message(
    error: &DownloadError,
    error_type: DownloadErrorType,
) -> String {
    let base = error.to_string();
    if base.contains("Suggestion:") {
        return base;
    }

    let suggestion = match error_type {
        DownloadErrorType::Network => {
            "Check network connectivity/VPN access, then retry with --max-retries set higher if needed."
        }
        DownloadErrorType::Auth => {
            "Run `downloader auth capture` (or configure proxy credentials for HTTP 407) and retry."
        }
        DownloadErrorType::NotFound => {
            "Verify the source URL/DOI/reference is still valid, then rerun with an updated source."
        }
        DownloadErrorType::ParseError => {
            "Check input formatting for URL/DOI/reference and rerun with a valid source string."
        }
    };

    format!("{base}\n  Suggestion: {suggestion}")
}
