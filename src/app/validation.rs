use anyhow::{Result, bail};

pub(crate) fn ensure_save_cookies_usage(
    save_cookies: bool,
    cookie_source: Option<&str>,
) -> Result<()> {
    if save_cookies && cookie_source.is_none() {
        bail!(
            "--save-cookies requires --cookies FILE for download mode.\n  \
             For browser capture use: downloader auth capture --save-cookies"
        );
    }
    Ok(())
}

pub(crate) fn validate_cookie_stdin_conflict(
    cookie_source: Option<&str>,
    urls: &[String],
    stdin_is_terminal: bool,
) -> Result<bool> {
    let cookies_from_stdin = cookie_source == Some("-");
    if cookies_from_stdin && urls.is_empty() && !stdin_is_terminal {
        bail!(
            "Cannot read both cookies and URLs from stdin.\n  \
             Provide URLs as arguments when using --cookies -"
        );
    }
    Ok(cookies_from_stdin)
}

pub(crate) fn reject_misplaced_auth_namespace(urls: &[String]) -> Result<()> {
    let Some(first) = urls.first().map(String::as_str) else {
        return Ok(());
    };
    if !first.eq_ignore_ascii_case("auth") {
        return Ok(());
    }
    bail!(
        "Auth commands must be invoked as subcommands, not positional download input.\n  \
         Use: downloader auth capture [--save-cookies] or downloader auth clear"
    );
}

pub(crate) fn validate_search_date_range(since: Option<&str>, until: Option<&str>) -> Result<()> {
    if let (Some(since), Some(until)) = (since, until)
        && since > until
    {
        bail!(
            "What: Invalid search date range\nWhy: --since ({since}) is later than --until ({until})\nFix: Use an inclusive range where --since <= --until in SQLite datetime format (YYYY-MM-DD HH:MM:SS)."
        );
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    // --- ensure_save_cookies_usage ---

    #[test]
    fn test_ensure_save_cookies_errors_without_source() {
        let result = ensure_save_cookies_usage(true, None);
        assert!(result.is_err());
        let msg = format!("{}", result.unwrap_err());
        assert!(msg.contains("--save-cookies"));
    }

    #[test]
    fn test_ensure_save_cookies_ok_with_file_source() {
        assert!(ensure_save_cookies_usage(true, Some("cookies.txt")).is_ok());
    }

    #[test]
    fn test_ensure_save_cookies_ok_with_stdin_source() {
        assert!(ensure_save_cookies_usage(true, Some("-")).is_ok());
    }

    #[test]
    fn test_ensure_save_cookies_disabled_without_source_is_ok() {
        assert!(ensure_save_cookies_usage(false, None).is_ok());
    }

    #[test]
    fn test_ensure_save_cookies_disabled_with_source_is_ok() {
        assert!(ensure_save_cookies_usage(false, Some("cookies.txt")).is_ok());
    }

    // --- validate_cookie_stdin_conflict ---

    #[test]
    fn test_cookie_stdin_conflict_when_urls_empty_and_not_terminal() {
        let result = validate_cookie_stdin_conflict(Some("-"), &[], false);
        assert!(result.is_err());
        let msg = format!("{}", result.unwrap_err());
        assert!(msg.contains("stdin"));
    }

    #[test]
    fn test_cookie_stdin_no_conflict_when_stdin_is_terminal() {
        let result = validate_cookie_stdin_conflict(Some("-"), &[], true);
        assert!(result.is_ok());
        assert!(
            result.unwrap(),
            "should return true when cookies are from stdin"
        );
    }

    #[test]
    fn test_cookie_stdin_no_conflict_when_urls_provided() {
        let urls = vec!["https://example.com".to_string()];
        let result = validate_cookie_stdin_conflict(Some("-"), &urls, false);
        assert!(result.is_ok());
        assert!(result.unwrap());
    }

    #[test]
    fn test_cookie_stdin_returns_false_when_source_is_not_stdin() {
        let result = validate_cookie_stdin_conflict(Some("cookies.txt"), &[], false);
        assert!(result.is_ok());
        assert!(
            !result.unwrap(),
            "should return false when cookie source is a file"
        );
    }

    #[test]
    fn test_cookie_stdin_returns_false_when_no_source() {
        let result = validate_cookie_stdin_conflict(None, &[], false);
        assert!(result.is_ok());
        assert!(!result.unwrap());
    }

    // --- reject_misplaced_auth_namespace ---

    #[test]
    fn test_reject_auth_as_first_url_lowercase() {
        let urls = vec!["auth".to_string()];
        let result = reject_misplaced_auth_namespace(&urls);
        assert!(result.is_err());
        let msg = format!("{}", result.unwrap_err());
        assert!(msg.contains("subcommand"));
    }

    #[test]
    fn test_reject_auth_as_first_url_uppercase() {
        let urls = vec!["AUTH".to_string()];
        assert!(reject_misplaced_auth_namespace(&urls).is_err());
    }

    #[test]
    fn test_reject_auth_as_first_url_mixed_case() {
        let urls = vec!["Auth".to_string()];
        assert!(reject_misplaced_auth_namespace(&urls).is_err());
    }

    #[test]
    fn test_accept_non_auth_first_url() {
        let urls = vec!["https://example.com".to_string()];
        assert!(reject_misplaced_auth_namespace(&urls).is_ok());
    }

    #[test]
    fn test_accept_empty_urls() {
        assert!(reject_misplaced_auth_namespace(&[]).is_ok());
    }

    #[test]
    fn test_auth_only_rejected_as_first_not_second() {
        let urls = vec!["https://example.com".to_string(), "auth".to_string()];
        assert!(reject_misplaced_auth_namespace(&urls).is_ok());
    }

    // --- validate_search_date_range ---

    #[test]
    fn test_valid_date_range_since_before_until() {
        assert!(validate_search_date_range(Some("2024-01-01"), Some("2024-12-31")).is_ok());
    }

    #[test]
    fn test_valid_date_range_equal_dates() {
        assert!(validate_search_date_range(Some("2024-06-15"), Some("2024-06-15")).is_ok());
    }

    #[test]
    fn test_invalid_date_range_since_after_until() {
        let result = validate_search_date_range(Some("2024-12-31"), Some("2024-01-01"));
        assert!(result.is_err());
        let msg = format!("{}", result.unwrap_err());
        assert!(msg.contains("--since"));
        assert!(msg.contains("--until"));
    }

    #[test]
    fn test_valid_date_range_only_since() {
        assert!(validate_search_date_range(Some("2024-01-01"), None).is_ok());
    }

    #[test]
    fn test_valid_date_range_only_until() {
        assert!(validate_search_date_range(None, Some("2024-12-31")).is_ok());
    }

    #[test]
    fn test_valid_date_range_both_none() {
        assert!(validate_search_date_range(None, None).is_ok());
    }

    #[test]
    fn test_valid_date_range_with_time_component() {
        assert!(
            validate_search_date_range(Some("2024-01-01 00:00:00"), Some("2024-12-31 23:59:59"))
                .is_ok()
        );
    }

    #[test]
    fn test_invalid_date_range_error_includes_values() {
        let result = validate_search_date_range(Some("2025-01-01"), Some("2024-01-01"));
        assert!(result.is_err());
        let msg = format!("{}", result.unwrap_err());
        assert!(msg.contains("2025-01-01"));
        assert!(msg.contains("2024-01-01"));
    }
}
