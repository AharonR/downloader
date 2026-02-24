//! Input validation, cookie jar loading, and assembly of input text from URLs and/or stdin.

use std::io::{self, IsTerminal, Read};
use std::sync::Arc;

use anyhow::Result;
use downloader_core::load_runtime_cookie_jar;
use reqwest::cookie::Jar;

use crate::app::validation;
use crate::cli::DownloadArgs;

/// Validates download input, loads the runtime cookie jar, assembles input text from
/// positional URLs and/or stdin. Returns values needed to build RunContext and to decide
/// dry-run / quick-start.
///
/// Returns `(cookie_jar, input_text, piped_stdin_was_empty)`.
pub(crate) fn process_input(
    args: &DownloadArgs,
) -> Result<(Option<Arc<Jar>>, Option<String>, bool)> {
    validation::reject_misplaced_auth_namespace(&args.urls)?;
    let stdin_is_terminal = io::stdin().is_terminal();
    validation::ensure_save_cookies_usage(args.save_cookies, args.cookies.as_deref())?;
    let cookies_from_stdin = validation::validate_cookie_stdin_conflict(
        args.cookies.as_deref(),
        &args.urls,
        stdin_is_terminal,
    )?;

    let cookie_jar = load_runtime_cookie_jar(args.cookies.as_deref(), args.save_cookies)?;

    let mut input_segments = Vec::new();
    if !args.urls.is_empty() {
        input_segments.push(args.urls.join("\n"));
    }

    let mut piped_stdin_was_empty = false;
    if !cookies_from_stdin && !stdin_is_terminal {
        let mut buffer = String::new();
        io::stdin().read_to_string(&mut buffer)?;
        if buffer.trim().is_empty() {
            piped_stdin_was_empty = true;
        } else {
            input_segments.push(buffer);
        }
    }

    let input_text = if input_segments.is_empty() {
        None
    } else {
        Some(input_segments.join("\n"))
    };

    Ok((cookie_jar, input_text, piped_stdin_was_empty))
}

#[cfg(test)]
mod tests {
    use super::process_input;
    use clap::Parser;
    use std::io::IsTerminal;
    use crate::cli::Cli;

    /// With empty urls and when stdin is not read (terminal) or read and empty,
    /// input_text is None. When stdin is terminal we don't read it so we only assert input_text.is_none().
    /// When stdin is non-terminal the test would read stdin and might block in cargo test; so we only run the assertion path when stdin is terminal.
    #[test]
    fn test_process_input_empty_urls_no_stdin_content() {
        let cli = Cli::try_parse_from(["downloader"]).unwrap();
        let args = cli.download;
        assert!(args.urls.is_empty());
        if std::io::stdin().is_terminal() {
            let (_, input_text, piped_stdin_was_empty) = process_input(&args).unwrap();
            assert!(input_text.is_none());
            assert!(!piped_stdin_was_empty, "when stdin is terminal we do not read it, so piped_stdin_was_empty must be false");
        }
        // When stdin is non-terminal, process_input would read stdin (and in cargo test might block). So we only run the empty-args path when stdin is terminal.
    }

    /// With URLs provided and stdin terminal (so stdin not read), input_text is Some and contains the URLs.
    #[test]
    fn test_process_input_with_urls_returns_input_text() {
        if !std::io::stdin().is_terminal() {
            return; // would read stdin and block in cargo test
        }
        let cli = Cli::try_parse_from(["downloader", "https://example.com/foo"]).unwrap();
        let args = cli.download;
        let (_, input_text, piped_stdin_was_empty) = process_input(&args).unwrap();
        let text = input_text.expect("input_text should be Some when urls non-empty");
        assert!(text.contains("https://example.com/foo"));
        assert!(!piped_stdin_was_empty, "stdin not read when urls provided and terminal");
    }

    /// process_input returns Err when first URL is "auth" (reject_misplaced_auth_namespace).
    #[test]
    fn test_process_input_errors_when_first_url_is_auth() {
        // Use "--" so "auth" is parsed as a positional URL, not the auth subcommand.
        let cli = Cli::try_parse_from(["downloader", "--", "auth"]).unwrap();
        let args = cli.download;
        assert_eq!(args.urls.as_slice(), ["auth"]);
        let result = process_input(&args);
        assert!(result.is_err(), "process_input should error when first URL is 'auth'");
    }

    /// process_input returns Err when --save-cookies is set without --cookies (ensure_save_cookies_usage).
    #[test]
    fn test_process_input_errors_when_save_cookies_without_cookies() {
        let cli = Cli::try_parse_from(["downloader", "--save-cookies"]).unwrap();
        let args = cli.download;
        assert!(args.save_cookies);
        assert!(args.cookies.is_none());
        let result = process_input(&args);
        assert!(
            result.is_err(),
            "process_input should error when --save-cookies without --cookies"
        );
    }
}
