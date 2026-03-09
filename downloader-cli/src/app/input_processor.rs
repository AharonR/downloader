//! Input validation, cookie jar loading, and assembly of input text from URLs and/or stdin.

use std::io::{self, IsTerminal, Read};
use std::path::Path;
use std::sync::Arc;

use anyhow::{Context, Result, bail};
use downloader_core::{ParsedItem, load_runtime_cookie_jar, parse_ris_content};
use reqwest::cookie::Jar;
use tracing::info;

use crate::app::validation;
use crate::cli::DownloadArgs;

/// Validates download input, loads the runtime cookie jar, assembles input text from
/// positional URLs and/or stdin, and reads any bibliography files supplied via
/// `--bibliography`. Returns values needed to build RunContext and to decide
/// dry-run / quick-start.
///
/// Returns `(cookie_jar, input_text, piped_stdin_was_empty, bibliography_items)`.
#[allow(clippy::type_complexity)]
pub(crate) fn process_input(
    args: &DownloadArgs,
) -> Result<(Option<Arc<Jar>>, Option<String>, bool, Vec<ParsedItem>)> {
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

    // Process --bibliography files.
    let mut bibliography_items: Vec<ParsedItem> = Vec::new();
    for path in &args.bibliography_files {
        let (bib_segment, mut bib_items) = read_bibliography_file(path)?;
        if let Some(segment) = bib_segment {
            input_segments.push(segment);
        }
        bibliography_items.append(&mut bib_items);
    }

    let input_text = if input_segments.is_empty() {
        None
    } else {
        Some(input_segments.join("\n"))
    };

    Ok((
        cookie_jar,
        input_text,
        piped_stdin_was_empty,
        bibliography_items,
    ))
}

/// Reads a bibliography file and returns either raw text (for `.bib`) or parsed items (for `.ris`).
///
/// - `.bib` files: raw content is returned as a string segment for `parse_input` (which has
///   native BibTeX support). The items `Vec` will be empty.
/// - `.ris` files: content is parsed with [`parse_ris_content`] and the resulting items are
///   returned. The segment `Option` will be `None`.
/// - Other extensions: an error is returned describing supported formats.
///
/// # Errors
///
/// Returns an error when the file cannot be read, or has an unsupported extension.
fn read_bibliography_file(path: &Path) -> Result<(Option<String>, Vec<ParsedItem>)> {
    let extension = path
        .extension()
        .and_then(|ext| ext.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();

    match extension.as_str() {
        "bib" => {
            let content = std::fs::read_to_string(path).with_context(|| {
                format!(
                    "Cannot read BibTeX file '{}'. \
                     Why: the file may not exist or is not readable. \
                     Fix: check the path and file permissions.",
                    path.display()
                )
            })?;
            info!(path = %path.display(), "Read BibTeX bibliography file");
            Ok((Some(content), Vec::new()))
        }
        "ris" => {
            let content = std::fs::read_to_string(path).with_context(|| {
                format!(
                    "Cannot read RIS file '{}'. \
                     Why: the file may not exist or is not readable. \
                     Fix: check the path and file permissions.",
                    path.display()
                )
            })?;
            let result = parse_ris_content(&content);
            info!(
                path = %path.display(),
                entries = result.entries.len(),
                items = result.items.len(),
                skipped = result.skipped.len(),
                "Parsed RIS bibliography file"
            );
            for skip_msg in &result.skipped {
                tracing::warn!(
                    path = %path.display(),
                    message = %skip_msg,
                    "Skipped RIS record"
                );
            }
            Ok((None, result.items))
        }
        other => {
            bail!(
                "What: unsupported bibliography file format '.{other}'. \
                 Why: only BibTeX (.bib) and RIS (.ris) files are supported. \
                 Fix: convert the file to .bib or .ris format, or supply DOIs/URLs directly."
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::process_input;
    use crate::cli::Cli;
    use clap::Parser;
    use std::io::IsTerminal;

    /// With empty urls and when stdin is not read (terminal) or read and empty,
    /// input_text is None. When stdin is terminal we don't read it so we only assert input_text.is_none().
    /// When stdin is non-terminal the test would read stdin and might block in cargo test; so we only run the assertion path when stdin is terminal.
    #[test]
    fn test_process_input_empty_urls_no_stdin_content() {
        let cli = Cli::try_parse_from(["downloader"]).unwrap();
        let args = cli.download;
        assert!(args.urls.is_empty());
        if std::io::stdin().is_terminal() {
            let (_, input_text, piped_stdin_was_empty, bib_items) = process_input(&args).unwrap();
            assert!(input_text.is_none());
            assert!(
                !piped_stdin_was_empty,
                "when stdin is terminal we do not read it, so piped_stdin_was_empty must be false"
            );
            assert!(bib_items.is_empty());
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
        let (_, input_text, piped_stdin_was_empty, _bib_items) = process_input(&args).unwrap();
        let text = input_text.expect("input_text should be Some when urls non-empty");
        assert!(text.contains("https://example.com/foo"));
        assert!(
            !piped_stdin_was_empty,
            "stdin not read when urls provided and terminal"
        );
    }

    /// process_input returns Err when first URL is "auth" (reject_misplaced_auth_namespace).
    #[test]
    fn test_process_input_errors_when_first_url_is_auth() {
        // Use "--" so "auth" is parsed as a positional URL, not the auth subcommand.
        let cli = Cli::try_parse_from(["downloader", "--", "auth"]).unwrap();
        let args = cli.download;
        assert_eq!(args.urls.as_slice(), ["auth"]);
        let result = process_input(&args);
        assert!(
            result.is_err(),
            "process_input should error when first URL is 'auth'"
        );
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

    /// process_input returns Err for unsupported bibliography file extensions.
    #[test]
    fn test_process_input_errors_on_unsupported_bibliography_extension() {
        if !std::io::stdin().is_terminal() {
            return;
        }
        let cli = Cli::try_parse_from(["downloader", "--bibliography", "/tmp/refs.csv"]).unwrap();
        let args = cli.download;
        let result = process_input(&args);
        assert!(
            result.is_err(),
            "unsupported bibliography format should return error"
        );
        let msg = result.unwrap_err().to_string();
        assert!(
            msg.contains("unsupported bibliography file format"),
            "error should mention format"
        );
    }

    /// BibTeX bibliography file content is added to input text.
    #[test]
    fn test_process_input_bib_file_content_added_to_input_text() {
        if !std::io::stdin().is_terminal() {
            return;
        }
        use std::io::Write;
        let mut tmp = tempfile::NamedTempFile::with_suffix(".bib").unwrap();
        write!(
            tmp,
            "@article{{k, title={{T}}, author={{Smith, J.}}, year={{2024}}, doi={{10.1234/bib}}}}"
        )
        .unwrap();
        let path = tmp.path().to_str().unwrap().to_string();

        let cli = Cli::try_parse_from(["downloader", "--bibliography", &path]).unwrap();
        let args = cli.download;
        let (_, input_text, _, bib_items) = process_input(&args).unwrap();
        assert!(
            input_text.is_some(),
            "bib content should be added to input_text"
        );
        assert!(
            input_text.unwrap().contains("10.1234/bib"),
            "input_text should contain the DOI from the bib file"
        );
        assert!(
            bib_items.is_empty(),
            "bib files produce no extra items (handled via parse_input)"
        );
    }

    /// RIS bibliography file items are returned as bibliography_items.
    #[test]
    fn test_process_input_ris_file_items_returned_as_bibliography_items() {
        if !std::io::stdin().is_terminal() {
            return;
        }
        use std::io::Write;
        let mut tmp = tempfile::NamedTempFile::with_suffix(".ris").unwrap();
        write!(
            tmp,
            "TY  - JOUR\nTI  - A Title\nDO  - 10.9999/ris-test\nER  - \n"
        )
        .unwrap();
        let path = tmp.path().to_str().unwrap().to_string();

        let cli = Cli::try_parse_from(["downloader", "--bibliography", &path]).unwrap();
        let args = cli.download;
        let (_, input_text, _, bib_items) = process_input(&args).unwrap();
        assert!(
            input_text.is_none(),
            "RIS files should not add to input_text"
        );
        assert!(
            !bib_items.is_empty(),
            "RIS items should be in bibliography_items"
        );
        assert!(
            bib_items
                .iter()
                .any(|item| item.value == "10.9999/ris-test"),
            "bibliography_items should contain the DOI"
        );
    }
}
