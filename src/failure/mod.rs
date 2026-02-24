//! Failure classification and user-facing descriptors for download/history errors.

use downloader_core::{DownloadAttempt, DownloadErrorType};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum FailureCategory {
    Auth,
    InputSource,
    Network,
    Other,
}

impl FailureCategory {
    #[must_use]
    pub fn icon(self) -> &'static str {
        match self {
            Self::Auth => "🔐",
            Self::InputSource => "❌",
            Self::Network => "🌐",
            Self::Other => "⚠️",
        }
    }

    #[must_use]
    pub fn label(self) -> &'static str {
        match self {
            Self::Auth => "Authentication",
            Self::InputSource => "Input/Source",
            Self::Network => "Network",
            Self::Other => "Other",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FailureDescriptor {
    pub category: FailureCategory,
    pub what: &'static str,
    pub why: &'static str,
    pub fix: &'static str,
}

/// Classifies an error message string into a category and descriptor.
#[must_use]
pub fn classify_failure(error: &str) -> FailureDescriptor {
    if error.starts_with("[AUTH]") {
        if error.contains("(HTTP 407)") {
            FailureDescriptor {
                category: FailureCategory::Auth,
                what: "Proxy authentication required",
                why: "The proxy rejected this request until valid proxy credentials are provided.",
                fix: "Configure your HTTP proxy settings or check proxy credentials.",
            }
        } else {
            FailureDescriptor {
                category: FailureCategory::Auth,
                what: "Authentication required",
                why: "The source requires a valid logged-in session/cookie before access.",
                fix: "Run `downloader auth capture` to authenticate.",
            }
        }
    } else if error.contains("HTTP 404") {
        FailureDescriptor {
            category: FailureCategory::InputSource,
            what: "Source not found",
            why: "The resolved source returned HTTP 404, which usually means the link is stale.",
            fix: "Verify the source URL or reference and retry with an updated link.",
        }
    } else if error.contains("timeout") {
        FailureDescriptor {
            category: FailureCategory::Network,
            what: "Download timed out",
            why: "The remote host did not respond within the request timeout window.",
            fix: "Increase retries or check network stability before retrying.",
        }
    } else if error.contains("network error") {
        FailureDescriptor {
            category: FailureCategory::Network,
            what: "Network request failed",
            why: "Connectivity, DNS, TLS, or VPN conditions interrupted the request.",
            fix: "Check connectivity/VPN settings, then rerun to resume.",
        }
    } else if error.contains("invalid URL")
        || error.contains("invalid DOI")
        || error.contains("could not parse reference")
    {
        FailureDescriptor {
            category: FailureCategory::InputSource,
            what: "Input could not be parsed",
            why: "The provided URL/DOI/reference format could not be interpreted safely.",
            fix: "Review the input format and retry with a valid URL, DOI, or reference.",
        }
    } else {
        FailureDescriptor {
            category: FailureCategory::Other,
            what: "Unhandled failure",
            why: "The error did not match a known category and needs closer inspection.",
            fix: "Inspect logs and rerun; unresolved items stay in the queue.",
        }
    }
}

/// Returns a descriptor for a history attempt (typed error first, then message-based classification).
#[must_use]
pub fn history_failure_descriptor(attempt: &DownloadAttempt) -> FailureDescriptor {
    if let Some(typed_descriptor) = descriptor_from_error_type(attempt) {
        return typed_descriptor;
    }

    if let Some(message) = attempt.error_message.as_deref() {
        return classify_failure(message);
    }

    FailureDescriptor {
        category: FailureCategory::Other,
        what: "Unhandled failure",
        why: "The error did not match a known category and needs closer inspection.",
        fix: "Inspect logs and rerun; unresolved items stay in the queue.",
    }
}

fn descriptor_from_error_type(attempt: &DownloadAttempt) -> Option<FailureDescriptor> {
    match attempt.error_type() {
        Some(DownloadErrorType::Auth) => Some(FailureDescriptor {
            category: FailureCategory::Auth,
            what: if attempt.http_status == Some(407) {
                "Proxy authentication required"
            } else {
                "Authentication required"
            },
            why: "The source requires authenticated access before download is allowed.",
            fix: if attempt.http_status == Some(407) {
                "Configure your HTTP proxy settings or check proxy credentials."
            } else {
                "Run `downloader auth capture` to authenticate."
            },
        }),
        Some(DownloadErrorType::NotFound) => Some(FailureDescriptor {
            category: FailureCategory::InputSource,
            what: "Source not found",
            why: "The source URL/reference no longer resolves to a downloadable resource.",
            fix: "Verify the source URL/DOI/reference and retry with an updated source.",
        }),
        Some(DownloadErrorType::ParseError) => Some(FailureDescriptor {
            category: FailureCategory::InputSource,
            what: "Input could not be parsed",
            why: "The supplied source format could not be interpreted safely.",
            fix: "Check input formatting and rerun with a valid URL/DOI/reference.",
        }),
        Some(DownloadErrorType::Network) => Some(FailureDescriptor {
            category: FailureCategory::Network,
            what: "Network request failed",
            why: "Connectivity, DNS, TLS, or VPN conditions interrupted the request.",
            fix: "Check connectivity/VPN settings, then retry.",
        }),
        None => None,
    }
}

/// Returns a short suggestion string for a history attempt.
#[must_use]
pub fn history_failure_suggestion(attempt: &DownloadAttempt) -> String {
    if let Some(error_type) = attempt.error_type() {
        return typed_history_suggestion(error_type, attempt.http_status).to_string();
    }

    if let Some(message) = &attempt.error_message
        && let Some((_, suggestion)) = message.split_once("Suggestion:")
    {
        let normalized = suggestion.trim();
        if !normalized.is_empty() {
            return normalized.to_string();
        }
    }

    "Check connectivity/VPN settings, then retry.".to_string()
}

fn typed_history_suggestion(
    error_type: DownloadErrorType,
    http_status: Option<i64>,
) -> &'static str {
    match error_type {
        DownloadErrorType::Auth => {
            if http_status == Some(407) {
                "Configure your HTTP proxy settings or check proxy credentials."
            } else {
                "Run `downloader auth capture` to authenticate."
            }
        }
        DownloadErrorType::NotFound => {
            "Verify the source URL/DOI/reference and retry with an updated source."
        }
        DownloadErrorType::ParseError => {
            "Check input formatting and rerun with a valid URL/DOI/reference."
        }
        DownloadErrorType::Network => "Check connectivity/VPN settings, then retry.",
    }
}

/// Extracts the domain from an `[AUTH]`-prefixed error string.
///
/// Expected format: `[AUTH] authentication required for {domain} (HTTP ...`
#[must_use]
pub fn extract_auth_domain(error: &str) -> Option<String> {
    let after_prefix = error.strip_prefix("[AUTH] authentication required for ")?;
    let end = after_prefix.find(" (HTTP")?;
    Some(after_prefix[..end].to_string())
}

/// Returns a generic descriptor for a category (used in summary grouping).
#[must_use]
pub fn category_failure_descriptor(category: FailureCategory) -> FailureDescriptor {
    match category {
        FailureCategory::Auth => FailureDescriptor {
            category,
            what: "Authentication issue",
            why: "The source or proxy requires valid credentials/session state.",
            fix: "Run `downloader auth capture`; for HTTP 407 also verify proxy settings.",
        },
        FailureCategory::InputSource => FailureDescriptor {
            category,
            what: "Input/source issue",
            why: "The source link/reference could not be resolved or no longer exists.",
            fix: "Verify the input URL/DOI/reference and retry with updated source data.",
        },
        FailureCategory::Network => FailureDescriptor {
            category,
            what: "Network issue",
            why: "Connectivity or transport problems interrupted download requests.",
            fix: "Check connectivity/VPN/proxy settings and retry.",
        },
        FailureCategory::Other => FailureDescriptor {
            category,
            what: "Unhandled issue",
            why: "The failure did not match a specific known category.",
            fix: "Inspect logs for details and rerun; unresolved items remain queued.",
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use downloader_core::DownloadAttempt;

    fn history_attempt(error_type: Option<&str>, error_message: Option<&str>) -> DownloadAttempt {
        DownloadAttempt {
            id: 1,
            url: "https://example.com/paper.pdf".to_string(),
            status_str: "failed".to_string(),
            file_path: None,
            title: None,
            authors: None,
            doi: None,
            parse_confidence: None,
            parse_confidence_factors: None,
            project: None,
            started_at: "2026-02-19 00:00:00".to_string(),
            error_message: error_message.map(ToString::to_string),
            error_type: error_type.map(ToString::to_string),
            retry_count: 0,
            last_retry_at: None,
            original_input: None,
            http_status: Some(401),
            duration_ms: Some(42),
        }
    }

    #[test]
    fn test_classify_failure_auth() {
        let d = classify_failure("[AUTH] authentication required for example.com (HTTP 401)");
        assert_eq!(d.category, FailureCategory::Auth);
        assert!(d.what.contains("Authentication"));
    }

    #[test]
    fn test_classify_failure_404() {
        let d = classify_failure("HTTP 404 downloading https://example.com/missing.pdf");
        assert_eq!(d.category, FailureCategory::InputSource);
        assert!(d.what.contains("not found"));
    }

    #[test]
    fn test_classify_failure_timeout() {
        let d = classify_failure("timeout downloading https://example.com/paper.pdf");
        assert_eq!(d.category, FailureCategory::Network);
        assert!(d.what.contains("timed out"));
    }

    #[test]
    fn test_classify_failure_other() {
        let d = classify_failure("HTTP 500 internal server error");
        assert_eq!(d.category, FailureCategory::Other);
    }

    #[test]
    fn test_extract_auth_domain_valid() {
        let msg = "[AUTH] authentication required for sub.example.com (HTTP 401)";
        assert_eq!(extract_auth_domain(msg).as_deref(), Some("sub.example.com"));
    }

    #[test]
    fn test_extract_auth_domain_non_auth_returns_none() {
        assert_eq!(extract_auth_domain("HTTP 404 not found"), None);
    }

    #[test]
    fn test_category_failure_descriptor_auth() {
        let d = category_failure_descriptor(FailureCategory::Auth);
        assert_eq!(d.category, FailureCategory::Auth);
        assert!(!d.what.is_empty());
        assert!(!d.fix.is_empty());
    }

    #[test]
    fn test_history_failure_descriptor_prioritizes_typed_error_over_message() {
        let attempt = history_attempt(
            Some("network"),
            Some("[AUTH] authentication required for example.com (HTTP 401)"),
        );
        let descriptor = history_failure_descriptor(&attempt);
        assert_eq!(descriptor.category, FailureCategory::Network);
    }

    #[test]
    fn test_history_failure_suggestion_prioritizes_typed_error_over_message_suggestion() {
        let attempt = history_attempt(
            Some("auth"),
            Some("timeout downloading file\n  Suggestion: this should not be used"),
        );
        let suggestion = history_failure_suggestion(&attempt);
        assert!(suggestion.contains("auth capture"));
        assert!(!suggestion.contains("should not be used"));
    }
}
