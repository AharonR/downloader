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
