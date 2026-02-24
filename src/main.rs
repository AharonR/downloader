//! CLI entry point for the downloader tool.

use std::io;
use std::path::Path;
use std::process::{Command as ProcessCommand, ExitCode, ExitStatus};

use anyhow::{Result, anyhow, bail};
use downloader_core::DownloadSearchCandidate;

mod app;
mod app_config;
mod cli;
mod commands;
mod failure;
mod output;
mod project;
mod search;

#[cfg(test)]
use std::collections::HashSet;

#[cfg(test)]
use app::config_runtime::HttpTimeoutSettings;
use app_config::FileConfig;
#[cfg(test)]
use app_config::VerbositySetting;
use cli::{DownloadArgs, HistoryStatusArg};

#[cfg(test)]
use downloader_core::Queue;

pub(crate) use app::config_runtime::CliValueSources;

#[tokio::main]
async fn main() -> ExitCode {
    match run_downloader().await {
        Ok(outcome) => ExitCode::from(outcome.code()),
        Err(error) => {
            eprintln!("{error}");
            ExitCode::from(ProcessExit::Failure.code())
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ProcessExit {
    Success,
    Partial,
    Failure,
}

impl ProcessExit {
    const fn code(self) -> u8 {
        match self {
            Self::Success => 0,
            Self::Partial => 1,
            Self::Failure => 2,
        }
    }
}

async fn run_downloader() -> Result<ProcessExit> {
    app::runtime::run_downloader().await
}

pub(crate) fn apply_config_defaults(
    args: DownloadArgs,
    cli_sources: &CliValueSources,
    file_config: Option<&FileConfig>,
) -> Result<DownloadArgs> {
    app::config_runtime::apply_config_defaults(args, cli_sources, file_config)
}

#[cfg(test)]
fn resolve_http_timeouts(file_config: Option<&FileConfig>) -> HttpTimeoutSettings {
    app::config_runtime::resolve_http_timeouts(file_config)
}

#[cfg(test)]
fn resolve_default_log_level(args: &DownloadArgs) -> &'static str {
    app::config_runtime::resolve_default_log_level(args)
}

#[cfg(test)]
fn should_force_cli_log_level(cli_sources: &CliValueSources) -> bool {
    app::config_runtime::should_force_cli_log_level(cli_sources)
}

#[cfg(test)]
fn should_disable_color(no_color_flag: bool, no_color_env: bool, dumb_terminal: bool) -> bool {
    app::terminal::should_disable_color(no_color_flag, no_color_env, dumb_terminal)
}

#[cfg(test)]
fn should_use_spinner(stderr_is_terminal: bool, quiet: bool, dumb_terminal: bool) -> bool {
    app::terminal::should_use_spinner(stderr_is_terminal, quiet, dumb_terminal)
}

#[cfg(test)]
fn determine_exit_outcome(completed: usize, failed: usize) -> ProcessExit {
    app::exit_handler::determine_exit_outcome(completed, failed)
}

pub(crate) fn verbosity_label(verbose: u8, quiet: bool, debug: bool) -> &'static str {
    app::config_runtime::verbosity_label(verbose, quiet, debug)
}

#[cfg(test)]
fn ensure_save_cookies_usage(save_cookies: bool, cookie_source: Option<&str>) -> Result<()> {
    app::validation::ensure_save_cookies_usage(save_cookies, cookie_source)
}

#[cfg(test)]
fn validate_cookie_stdin_conflict(
    cookie_source: Option<&str>,
    urls: &[String],
    stdin_is_terminal: bool,
) -> Result<bool> {
    app::validation::validate_cookie_stdin_conflict(cookie_source, urls, stdin_is_terminal)
}

#[cfg(test)]
fn reject_misplaced_auth_namespace(urls: &[String]) -> Result<()> {
    app::validation::reject_misplaced_auth_namespace(urls)
}

#[derive(Debug, Clone)]
struct OpenCommandInvocation {
    program: &'static str,
    args: Vec<String>,
}

pub(crate) fn validate_search_date_range(since: Option<&str>, until: Option<&str>) -> Result<()> {
    app::validation::validate_search_date_range(since, until)
}

pub(crate) fn resolve_search_candidate_file_path(
    candidate: &mut DownloadSearchCandidate,
    db_path: &Path,
) {
    let Some(raw_path) = candidate.file_path.as_deref() else {
        return;
    };

    let stored_path = Path::new(raw_path);
    if stored_path.is_absolute() {
        return;
    }

    let Some(downloader_dir) = db_path.parent() else {
        return;
    };
    let Some(history_root) = downloader_dir.parent() else {
        return;
    };

    candidate.file_path = Some(history_root.join(stored_path).to_string_lossy().to_string());
}

pub(crate) fn render_search_cli_row(result: &search::RankedSearchResult, width: usize) -> String {
    let title_or_file = search_result_title_or_file(result);
    let path = result.candidate.file_path.as_deref().unwrap_or("n/a");
    let base_line = format!(
        "{} | {} | match={} | {}",
        result.candidate.started_at, title_or_file, result.matched_field, path
    );
    output::truncate_to_width(&base_line, width)
}

fn search_result_title_or_file(result: &search::RankedSearchResult) -> String {
    result
        .candidate
        .title
        .clone()
        .or_else(|| {
            result
                .candidate
                .file_path
                .as_deref()
                .and_then(|path| Path::new(path).file_name().and_then(|name| name.to_str()))
                .map(std::string::ToString::to_string)
        })
        .unwrap_or_else(|| "n/a".to_string())
}

pub(crate) fn open_path_in_default_app(path: &Path) -> Result<()> {
    open_path_with_runner(path, |invocation| {
        ProcessCommand::new(invocation.program)
            .args(&invocation.args)
            .status()
    })
}

fn open_path_with_runner<F>(path: &Path, mut runner: F) -> Result<()>
where
    F: FnMut(&OpenCommandInvocation) -> io::Result<ExitStatus>,
{
    if !path.exists() {
        bail!(
            "What: Cannot open search result file\nWhy: File does not exist at {}\nFix: Re-run without --open or redownload the item.",
            path.display()
        );
    }

    let invocation = build_open_command_invocation(path);
    let status = runner(&invocation).map_err(|error| {
        anyhow!(
            "What: Failed to launch system opener\nWhy: '{}' could not execute ({error})\nFix: Open the file manually at {}.",
            invocation.program,
            path.display()
        )
    })?;

    if !status.success() {
        bail!(
            "What: System opener returned a non-zero exit status\nWhy: '{}' failed while opening {}\nFix: Open the file manually at {}.",
            invocation.program,
            path.display(),
            path.display()
        );
    }

    Ok(())
}

fn build_open_command_invocation(path: &Path) -> OpenCommandInvocation {
    let path_arg = path.to_string_lossy().to_string();
    if cfg!(target_os = "macos") {
        OpenCommandInvocation {
            program: "open",
            args: vec![path_arg],
        }
    } else if cfg!(target_os = "windows") {
        OpenCommandInvocation {
            program: "explorer",
            args: vec![path_arg],
        }
    } else {
        OpenCommandInvocation {
            program: "xdg-open",
            args: vec![path_arg],
        }
    }
}

pub(crate) fn log_parse_feedback(parse_result: &downloader_core::ParseResult) {
    output::log_parse_feedback(parse_result);
}

#[cfg(test)]
fn build_parse_feedback_summary(parse_result: &downloader_core::ParseResult) -> String {
    output::build_parse_feedback_summary(parse_result)
}

pub(crate) fn map_history_status(
    status: HistoryStatusArg,
) -> downloader_core::DownloadAttemptStatus {
    output::map_history_status(status)
}

pub(crate) fn render_history_cli_row(
    attempt: &downloader_core::DownloadAttempt,
    failed_only: bool,
    width: usize,
) -> String {
    output::render_history_cli_row(attempt, failed_only, width)
}

#[cfg(test)]
fn uncertain_reference_summary_line(uncertain_references_in_run: usize) -> Option<String> {
    output::uncertain_reference_summary_line(uncertain_references_in_run)
}

#[cfg(test)]
fn render_failure_summary_lines(failed_reasons: &[&str], width: usize) -> Vec<String> {
    output::render_failure_summary_lines(failed_reasons, width)
}

#[cfg(test)]
async fn append_project_download_log(
    queue: &Queue,
    output_dir: &Path,
    history_start_id: Option<i64>,
) -> Result<()> {
    project::append_project_download_log(queue, output_dir, history_start_id).await
}

#[cfg(test)]
fn render_project_download_log_section(
    session_label: &str,
    attempts: &[downloader_core::DownloadAttempt],
) -> String {
    project::render_project_download_log_section(session_label, attempts)
}

#[cfg(test)]
async fn append_project_index(
    queue: &Queue,
    output_dir: &Path,
    completed_before: &HashSet<i64>,
) -> Result<()> {
    project::append_project_index(queue, output_dir, completed_before).await
}

/// Generates JSON-LD sidecar files for items completed in this run with saved paths.
///
/// Idempotent by design: `generate_sidecar()` fast-returns `None` when the sidecar
/// already exists on disk, so re-running is safe.
///
/// Sidecar failures are logged at `warn!` level and never propagate — they MUST NOT
/// affect the download exit code.
///
/// Returns the count of sidecar files newly written (skipped items not counted).
#[cfg(test)]
async fn generate_sidecars_for_completed(queue: &Queue, completed_before: &HashSet<i64>) -> usize {
    project::generate_sidecars_for_completed(queue, completed_before).await
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;
    use std::path::{Path, PathBuf};

    use super::{failure, output, project, search};
    use clap::Parser;
    use downloader_core::{
        Database, DownloadAttempt, DownloadAttemptStatus, DownloadErrorType,
        DownloadSearchCandidate, InputType, NewDownloadAttempt, ParseResult, ParsedItem, Queue,
        QueueMetadata,
    };
    use tempfile::TempDir;

    use super::{
        CliValueSources, FileConfig, ProcessExit, VerbositySetting, append_project_download_log,
        append_project_index, apply_config_defaults, build_open_command_invocation,
        build_parse_feedback_summary, determine_exit_outcome, ensure_save_cookies_usage,
        generate_sidecars_for_completed, map_history_status, open_path_with_runner,
        reject_misplaced_auth_namespace, render_failure_summary_lines, render_history_cli_row,
        render_project_download_log_section, render_search_cli_row, resolve_default_log_level,
        resolve_http_timeouts, resolve_search_candidate_file_path, search_result_title_or_file,
        should_disable_color, should_force_cli_log_level, should_use_spinner,
        validate_cookie_stdin_conflict, validate_search_date_range,
    };

    fn parse_download_args(
        args: impl IntoIterator<Item = &'static str>,
    ) -> crate::cli::DownloadArgs {
        let cli = crate::cli::Cli::parse_from(args);
        assert!(cli.command.is_none());
        cli.download
    }

    #[test]
    fn test_build_parse_feedback_summary_includes_type_counts() {
        let mut result = ParseResult::new();
        result.add_item(ParsedItem::url(
            "https://example.com",
            "https://example.com",
        ));
        result.add_item(ParsedItem::doi("10.1234/example", "10.1234/example"));
        result.add_item(ParsedItem::reference(
            "Smith, J. (2024). Paper Title. Journal.",
            "Smith, J. (2024). Paper Title. Journal.",
        ));
        result.add_item(ParsedItem::new(
            "@article{k}",
            InputType::BibTex,
            "@article{k}",
        ));

        let summary = build_parse_feedback_summary(&result);
        assert!(summary.contains("Parsed 4 items: 1 URLs, 1 DOIs, 1 references, 1 BibTeX"));
        assert!(summary.contains("[reference confidence: high=1, medium=0, low=0]"));
    }

    #[test]
    fn test_build_parse_feedback_summary_adds_uncertain_reference_suffix() {
        let mut result = ParseResult::new();
        result.add_item(ParsedItem::reference(
            "Weak candidate text with little structure",
            "Weak candidate text with little structure",
        ));

        let summary = build_parse_feedback_summary(&result);
        assert!(summary.contains("(1 references need verification)"));
    }

    #[test]
    fn test_build_parse_feedback_summary_does_not_flag_medium_confidence_reference() {
        let mut result = ParseResult::new();
        result.add_item(ParsedItem::reference(
            "Smith, J. (2024).",
            "Smith, J. (2024).",
        ));

        let summary = build_parse_feedback_summary(&result);
        assert!(!summary.contains("need verification"));
        assert!(summary.contains("[reference confidence: high=0, medium=1, low=0]"));
    }

    #[test]
    fn test_uncertain_reference_summary_line_returns_message_when_positive() {
        let line = super::uncertain_reference_summary_line(2);
        assert_eq!(
            line.as_deref(),
            Some("2 low-confidence references need manual verification")
        );
    }

    #[test]
    fn test_uncertain_reference_summary_line_returns_none_when_zero() {
        let line = super::uncertain_reference_summary_line(0);
        assert!(line.is_none());
    }

    #[test]
    fn test_truncate_to_width_exact_fit_returns_original() {
        let text = "Parsed 10 items";
        assert_eq!(output::truncate_to_width(text, text.len()), text);
    }

    #[test]
    fn test_truncate_to_width_truncates_and_appends_ellipsis() {
        assert_eq!(output::truncate_to_width("1234567890", 6), "12345…");
    }

    #[test]
    fn test_truncate_to_width_handles_tiny_widths() {
        assert_eq!(output::truncate_to_width("abcdef", 1), "…");
        assert_eq!(output::truncate_to_width("abcdef", 0), "");
    }

    #[test]
    fn test_quick_start_guidance_lines_include_headline_and_examples() {
        let lines = output::quick_start_guidance_lines(false, 80);
        assert_eq!(lines.len(), 3);
        assert!(lines[0].contains("No input provided"));
        assert!(lines[1].contains("echo 'https://example.com/file.pdf' | downloader"));
        assert!(lines[2].contains("downloader https://example.com/file.pdf"));
    }

    #[test]
    fn test_quick_start_guidance_lines_respect_width_cap() {
        let lines = output::quick_start_guidance_lines(true, 80);
        assert!(lines.iter().all(|line| line.chars().count() <= 80));
    }

    #[test]
    fn test_quick_start_guidance_lines_no_input_branch_respect_width_cap() {
        let lines = output::quick_start_guidance_lines(false, 80);
        assert!(lines.iter().all(|line| line.chars().count() <= 80));
    }

    #[test]
    fn test_resolve_default_log_level_default_is_info() {
        let args = parse_download_args(["downloader"]);
        assert_eq!(resolve_default_log_level(&args), "info");
    }

    #[test]
    fn test_resolve_default_log_level_verbose_is_debug() {
        let args = parse_download_args(["downloader", "-v"]);
        assert_eq!(resolve_default_log_level(&args), "debug");
    }

    #[test]
    fn test_resolve_default_log_level_debug_flag_is_trace() {
        let args = parse_download_args(["downloader", "--debug"]);
        assert_eq!(resolve_default_log_level(&args), "trace");
    }

    #[test]
    fn test_resolve_default_log_level_quiet_is_error() {
        let args = parse_download_args(["downloader", "-q"]);
        assert_eq!(resolve_default_log_level(&args), "error");
    }

    #[test]
    fn test_should_force_cli_log_level_when_debug_flag_present() {
        let sources = CliValueSources {
            debug: true,
            ..CliValueSources::default()
        };
        assert!(should_force_cli_log_level(&sources));
    }

    #[test]
    fn test_should_force_cli_log_level_false_without_verbosity_flags() {
        let sources = CliValueSources::default();
        assert!(!should_force_cli_log_level(&sources));
    }

    #[test]
    fn test_should_disable_color_when_flag_set() {
        assert!(should_disable_color(true, false, false));
    }

    #[test]
    fn test_should_disable_color_when_no_color_env_set() {
        assert!(should_disable_color(false, true, false));
    }

    #[test]
    fn test_should_disable_color_when_term_is_dumb() {
        assert!(should_disable_color(false, false, true));
    }

    #[test]
    fn test_should_use_spinner_only_when_interactive_and_not_quiet_or_dumb() {
        assert!(should_use_spinner(true, false, false));
        assert!(!should_use_spinner(false, false, false));
        assert!(!should_use_spinner(true, true, false));
        assert!(!should_use_spinner(true, false, true));
    }

    #[test]
    fn test_determine_exit_outcome_success_when_no_failures() {
        assert_eq!(determine_exit_outcome(3, 0), ProcessExit::Success);
    }

    #[test]
    fn test_determine_exit_outcome_partial_when_mixed_results() {
        assert_eq!(determine_exit_outcome(2, 1), ProcessExit::Partial);
    }

    #[test]
    fn test_determine_exit_outcome_failure_when_all_failed() {
        assert_eq!(determine_exit_outcome(0, 2), ProcessExit::Failure);
    }

    /// Process exit code contract (Epic 7 / help text): 0 = success, 1 = partial, 2 = failure.
    #[test]
    fn test_process_exit_code_contract() {
        assert_eq!(ProcessExit::Success.code(), 0);
        assert_eq!(ProcessExit::Partial.code(), 1);
        assert_eq!(ProcessExit::Failure.code(), 2);
    }

    #[test]
    fn test_apply_config_verbosity_debug_when_cli_omits_verbosity_flags() {
        let args = parse_download_args(["downloader"]);
        let sources = CliValueSources::default();
        let file_config = FileConfig {
            verbosity: Some(VerbositySetting::Debug),
            ..FileConfig::default()
        };

        let merged = apply_config_defaults(args, &sources, Some(&file_config)).unwrap();
        assert!(merged.debug);
        assert!(!merged.quiet);
        assert_eq!(merged.verbose, 0);
    }

    #[test]
    fn test_apply_config_verbosity_does_not_override_cli_debug_flag() {
        let args = parse_download_args(["downloader", "--debug"]);
        let sources = CliValueSources {
            debug: true,
            ..CliValueSources::default()
        };
        let file_config = FileConfig {
            verbosity: Some(VerbositySetting::Quiet),
            ..FileConfig::default()
        };

        let merged = apply_config_defaults(args, &sources, Some(&file_config)).unwrap();
        assert!(merged.debug);
        assert!(!merged.quiet);
    }

    // --- Regression tests for Story 8.1 code-review bug fixes ---

    /// Regression: detect_topics from config was not applied by apply_config_defaults.
    /// Bug: the merging block was missing, so config-level detect_topics was silently ignored.
    #[test]
    fn test_apply_config_defaults_merges_detect_topics_from_config() {
        let args = parse_download_args(["downloader"]);
        let sources = CliValueSources::default(); // CLI did not set detect_topics
        let file_config = FileConfig {
            detect_topics: Some(true),
            ..FileConfig::default()
        };

        let merged = apply_config_defaults(args, &sources, Some(&file_config)).unwrap();
        assert!(
            merged.detect_topics,
            "detect_topics=true from config should be applied when not set by CLI"
        );
    }

    /// Regression: topics_file from config was not applied by apply_config_defaults.
    /// Bug: the merging block was missing, so config-level topics_file was silently ignored.
    #[test]
    fn test_apply_config_defaults_merges_topics_file_from_config() {
        let args = parse_download_args(["downloader"]);
        let sources = CliValueSources::default(); // CLI did not set topics_file
        let file_config = FileConfig {
            topics_file: Some(PathBuf::from("/etc/topics.txt")),
            ..FileConfig::default()
        };

        let merged = apply_config_defaults(args, &sources, Some(&file_config)).unwrap();
        assert_eq!(
            merged.topics_file,
            Some(PathBuf::from("/etc/topics.txt")),
            "topics_file from config should be applied when not set by CLI"
        );
    }

    /// Regression: CLI --detect-topics should take precedence over config detect_topics=false.
    #[test]
    fn test_apply_config_defaults_cli_detect_topics_overrides_config() {
        let args = parse_download_args(["downloader", "--detect-topics"]);
        let sources = CliValueSources {
            detect_topics: true,
            ..CliValueSources::default()
        };
        let file_config = FileConfig {
            detect_topics: Some(false),
            ..FileConfig::default()
        };

        let merged = apply_config_defaults(args, &sources, Some(&file_config)).unwrap();
        assert!(
            merged.detect_topics,
            "CLI --detect-topics should win over config detect_topics=false"
        );
    }

    /// When --respectful is set, effective concurrency, rate_limit, max_retries are overridden.
    #[test]
    fn test_apply_config_defaults_respectful_overrides_c_l_r() {
        use crate::app::config_runtime::{
            RESPECTFUL_CONCURRENCY, RESPECTFUL_MAX_RETRIES, RESPECTFUL_RATE_LIMIT_MS,
        };

        let args = parse_download_args([
            "downloader",
            "--respectful",
            "-c",
            "20",
            "-l",
            "1000",
            "-r",
            "5",
        ]);
        let sources = CliValueSources {
            respectful: true,
            concurrency: true,
            rate_limit: true,
            ..CliValueSources::default()
        };

        let merged = apply_config_defaults(args, &sources, None).unwrap();
        assert!(merged.respectful);
        assert_eq!(merged.concurrency, RESPECTFUL_CONCURRENCY);
        assert_eq!(merged.rate_limit, RESPECTFUL_RATE_LIMIT_MS);
        assert_eq!(merged.max_retries, RESPECTFUL_MAX_RETRIES);
        assert!(merged.check_robots);
    }

    /// Regression: config detect_topics=false should not override the CLI flag.
    /// Also verifies that when config is None, no change is applied.
    #[test]
    fn test_apply_config_defaults_no_config_leaves_detect_topics_unchanged() {
        let args = parse_download_args(["downloader"]);
        let sources = CliValueSources::default();

        let merged = apply_config_defaults(args, &sources, None).unwrap();
        assert!(
            !merged.detect_topics,
            "detect_topics should remain false when no config present"
        );
    }

    #[test]
    fn test_apply_config_defaults_merges_sidecar_from_config() {
        let args = parse_download_args(["downloader"]);
        let sources = CliValueSources::default();
        let file_config = FileConfig {
            sidecar: Some(true),
            ..FileConfig::default()
        };
        let merged = apply_config_defaults(args, &sources, Some(&file_config)).unwrap();
        assert!(
            merged.sidecar,
            "sidecar=true from config should be applied when not set by CLI"
        );
    }

    #[test]
    fn test_apply_config_defaults_cli_sidecar_overrides_config() {
        let args = parse_download_args(["downloader", "--sidecar"]);
        let sources = CliValueSources {
            sidecar: true,
            ..CliValueSources::default()
        };
        let file_config = FileConfig {
            sidecar: Some(false),
            ..FileConfig::default()
        };
        let merged = apply_config_defaults(args, &sources, Some(&file_config)).unwrap();
        assert!(
            merged.sidecar,
            "CLI --sidecar should win over config sidecar=false"
        );
    }

    #[test]
    fn test_resolve_http_timeouts_defaults_when_no_config() {
        let settings = resolve_http_timeouts(None);
        assert_eq!(settings.download_connect_secs, 30);
        assert_eq!(settings.download_read_secs, 300);
        assert_eq!(settings.resolver_connect_secs, 10);
        assert_eq!(settings.resolver_read_secs, 30);
    }

    #[test]
    fn test_resolve_http_timeouts_uses_config_overrides() {
        let file_config = FileConfig {
            download_connect_timeout_secs: Some(12),
            download_read_timeout_secs: Some(180),
            resolver_connect_timeout_secs: Some(8),
            resolver_read_timeout_secs: Some(45),
            ..FileConfig::default()
        };
        let settings = resolve_http_timeouts(Some(&file_config));
        assert_eq!(settings.download_connect_secs, 12);
        assert_eq!(settings.download_read_secs, 180);
        assert_eq!(settings.resolver_connect_secs, 8);
        assert_eq!(settings.resolver_read_secs, 45);
    }

    #[test]
    fn test_classify_failure_auth_prefix() {
        let descriptor = failure::classify_failure(
            "[AUTH] authentication required for example.com (HTTP 401) downloading https://example.com/paper.pdf\n  Suggestion: Run `downloader auth capture` to authenticate",
        );
        assert_eq!(descriptor.category, failure::FailureCategory::Auth);
        assert_eq!(descriptor.what, "Authentication required");
        assert!(descriptor.fix.contains("downloader auth capture"));
    }

    #[test]
    fn test_classify_failure_407_proxy() {
        let descriptor = failure::classify_failure(
            "[AUTH] authentication required for proxy.corp.net (HTTP 407) downloading https://example.com/file.pdf\n  Suggestion: Configure your HTTP proxy settings or check proxy credentials.",
        );
        assert_eq!(descriptor.category, failure::FailureCategory::Auth);
        assert_eq!(descriptor.what, "Proxy authentication required");
        assert!(
            descriptor.fix.contains("proxy"),
            "407 should suggest proxy config, got: {}",
            descriptor.fix
        );
        assert!(
            !descriptor.fix.contains("downloader auth capture"),
            "407 should NOT suggest auth capture"
        );
    }

    #[test]
    fn test_classify_failure_404() {
        let descriptor =
            failure::classify_failure("HTTP 404 downloading https://example.com/missing.pdf");
        assert_eq!(descriptor.category, failure::FailureCategory::InputSource);
        assert_eq!(descriptor.what, "Source not found");
    }

    #[test]
    fn test_classify_failure_old_401_format_falls_through() {
        // Old format without [AUTH] prefix should not match auth classification
        let descriptor =
            failure::classify_failure("HTTP 401 downloading https://example.com/paper.pdf");
        assert_eq!(descriptor.category, failure::FailureCategory::Other);
        assert_eq!(descriptor.what, "Unhandled failure");
    }

    #[test]
    fn test_classify_failure_network_timeout() {
        let descriptor =
            failure::classify_failure("timeout downloading https://example.com/paper.pdf");
        assert_eq!(descriptor.category, failure::FailureCategory::Network);
        assert_eq!(descriptor.what, "Download timed out");
    }

    #[test]
    fn test_extract_auth_domain_valid() {
        let error = "[AUTH] authentication required for example.com (HTTP 401) downloading https://example.com/paper.pdf";
        assert_eq!(
            failure::extract_auth_domain(error),
            Some("example.com".to_string())
        );
    }

    #[test]
    fn test_extract_auth_domain_with_subdomain() {
        let error = "[AUTH] authentication required for idp.university.edu (HTTP 0) downloading https://sciencedirect.com/paper.pdf";
        assert_eq!(
            failure::extract_auth_domain(error),
            Some("idp.university.edu".to_string())
        );
    }

    #[test]
    fn test_extract_auth_domain_non_auth_error() {
        assert_eq!(
            failure::extract_auth_domain("HTTP 404 downloading foo"),
            None
        );
    }

    #[test]
    fn test_classify_failure_auth_category_401() {
        let descriptor = failure::classify_failure(
            "[AUTH] authentication required for example.com (HTTP 401) downloading https://example.com/paper.pdf",
        );
        assert_eq!(descriptor.category, failure::FailureCategory::Auth);
    }

    #[test]
    fn test_classify_failure_auth_category_407_proxy() {
        let descriptor = failure::classify_failure(
            "[AUTH] authentication required for proxy.corp.net (HTTP 407) downloading https://example.com/file.pdf",
        );
        assert_eq!(descriptor.category, failure::FailureCategory::Auth);
    }

    #[test]
    fn test_failure_category_icons_are_expected() {
        assert_eq!(failure::FailureCategory::Auth.icon(), "🔐");
        assert_eq!(failure::FailureCategory::InputSource.icon(), "❌");
        assert_eq!(failure::FailureCategory::Network.icon(), "🌐");
        assert_eq!(failure::FailureCategory::Other.icon(), "⚠️");
    }

    #[test]
    fn test_render_failure_summary_lines_groups_counts_by_category() {
        let reasons = vec![
            "[AUTH] authentication required for example.com (HTTP 401) downloading https://example.com/paper.pdf",
            "[AUTH] authentication required for proxy.corp.net (HTTP 407) downloading https://example.com/file.pdf",
            "HTTP 404 downloading https://example.com/missing.pdf",
            "network error downloading https://example.com/file.pdf: connection reset",
        ];
        let lines = render_failure_summary_lines(&reasons, 200);
        let rendered = lines.join("\n");

        assert!(rendered.contains("- 🔐 Authentication: 2"));
        assert!(rendered.contains("- ❌ Input/Source: 1"));
        assert!(rendered.contains("- 🌐 Network: 1"));
    }

    #[test]
    fn test_render_failure_summary_lines_include_what_why_fix_triplet() {
        let reasons = vec!["HTTP 404 downloading https://example.com/missing.pdf"];
        let lines = render_failure_summary_lines(&reasons, 200);
        let rendered = lines.join("\n");

        assert!(rendered.contains("What: Input/source issue"));
        assert!(rendered.contains("Why:"));
        assert!(rendered.contains("Fix: Verify the input URL/DOI/reference and retry"));
    }

    #[test]
    fn test_render_failure_summary_lines_uses_stable_auth_category_descriptor() {
        let reasons = vec![
            "[AUTH] authentication required for example.com (HTTP 401) downloading https://example.com/paper.pdf",
            "[AUTH] authentication required for proxy.corp.net (HTTP 407) downloading https://proxy.example.com/file.pdf",
        ];
        let lines = render_failure_summary_lines(&reasons, 200);
        let rendered = lines.join("\n");

        assert!(rendered.contains("- 🔐 Authentication: 2"));
        assert!(rendered.contains("What: Authentication issue"));
        assert!(rendered.contains(
            "Fix: Run `downloader auth capture`; for HTTP 407 also verify proxy settings."
        ));
    }

    #[test]
    fn test_ensure_save_cookies_usage_requires_cookie_source() {
        let err = ensure_save_cookies_usage(true, None).expect_err("missing cookie source");
        assert!(
            err.to_string()
                .contains("--save-cookies requires --cookies FILE")
        );
    }

    #[test]
    fn test_ensure_save_cookies_usage_accepts_cookie_source() {
        ensure_save_cookies_usage(true, Some("cookies.txt")).expect("valid cookie source");
        ensure_save_cookies_usage(false, None).expect("save disabled should be valid");
    }

    #[test]
    fn test_validate_cookie_stdin_conflict_rejects_dual_stdin_use() {
        let urls = Vec::new();
        let err = validate_cookie_stdin_conflict(Some("-"), &urls, false)
            .expect_err("stdin conflict should fail");
        assert!(
            err.to_string()
                .contains("Cannot read both cookies and URLs from stdin")
        );
    }

    #[test]
    fn test_validate_cookie_stdin_conflict_accepts_when_urls_are_args() {
        let urls = vec!["https://example.com/file.pdf".to_string()];
        let from_stdin = validate_cookie_stdin_conflict(Some("-"), &urls, false)
            .expect("urls as args should avoid stdin conflict");
        assert!(from_stdin);
    }

    #[test]
    fn test_validate_cookie_stdin_conflict_false_for_regular_cookie_file() {
        let urls = Vec::new();
        let from_stdin = validate_cookie_stdin_conflict(Some("cookies.txt"), &urls, false)
            .expect("regular cookie file should not use stdin");
        assert!(!from_stdin);
    }

    #[test]
    fn test_reject_misplaced_auth_namespace_for_capture_pattern() {
        let urls = vec!["auth".to_string(), "capture".to_string()];
        let err = reject_misplaced_auth_namespace(&urls)
            .expect_err("misplaced auth namespace should be rejected");
        assert!(
            err.to_string()
                .contains("Auth commands must be invoked as subcommands")
        );
    }

    #[test]
    fn test_reject_misplaced_auth_namespace_for_clear_pattern() {
        let urls = vec!["auth".to_string(), "clear".to_string()];
        reject_misplaced_auth_namespace(&urls)
            .expect_err("misplaced auth namespace should be rejected");
    }

    #[test]
    fn test_reject_misplaced_auth_namespace_for_missing_subcommand_pattern() {
        let urls = vec!["auth".to_string()];
        reject_misplaced_auth_namespace(&urls)
            .expect_err("misplaced auth namespace should be rejected");
    }

    #[test]
    fn test_reject_misplaced_auth_namespace_for_unknown_subcommand_pattern() {
        let urls = vec!["auth".to_string(), "foo".to_string()];
        reject_misplaced_auth_namespace(&urls)
            .expect_err("misplaced auth namespace should be rejected");
    }

    #[test]
    fn test_reject_misplaced_auth_namespace_for_help_like_pattern() {
        let urls = vec!["auth".to_string(), "help".to_string()];
        reject_misplaced_auth_namespace(&urls)
            .expect_err("misplaced auth namespace should be rejected");
    }

    #[test]
    fn test_reject_misplaced_auth_namespace_for_mixed_case_auth_token() {
        let urls = vec!["Auth".to_string(), "clear".to_string()];
        reject_misplaced_auth_namespace(&urls)
            .expect_err("misplaced auth namespace should be rejected");
    }

    #[test]
    fn test_reject_misplaced_auth_namespace_allows_unrelated_auth_prefixed_text() {
        let urls = vec!["authoritative".to_string(), "clear".to_string()];
        reject_misplaced_auth_namespace(&urls)
            .expect("non-auth leading token should remain allowed");
    }

    #[test]
    fn test_reject_misplaced_auth_namespace_allows_non_auth_download_input() {
        let urls = vec!["https://example.com/file.pdf".to_string()];
        reject_misplaced_auth_namespace(&urls)
            .expect("non-auth positional input should remain allowed");
    }

    #[test]
    fn test_sanitize_project_name_replaces_spaces_with_hyphen() {
        assert_eq!(
            project::sanitize_project_name("Climate Research"),
            "Climate-Research"
        );
    }

    #[test]
    fn test_sanitize_project_name_removes_unsafe_chars() {
        assert_eq!(
            project::sanitize_project_name("Lab: Q1 / Papers?"),
            "Lab-Q1-Papers"
        );
    }

    #[test]
    fn test_resolve_project_output_dir_joins_sanitized_name() {
        let base = Path::new("/tmp/downloads");
        let output = project::resolve_project_output_dir(base, Some("Climate Research"))
            .expect("should resolve");
        assert_eq!(output, PathBuf::from("/tmp/downloads/Climate-Research"));
    }

    #[test]
    fn test_resolve_project_output_dir_supports_nested_segments() {
        let base = Path::new("/tmp/downloads");
        let output = project::resolve_project_output_dir(base, Some("Climate/Emissions/2024"))
            .expect("nested project paths should be supported");
        assert_eq!(
            output,
            PathBuf::from("/tmp/downloads/Climate/Emissions/2024")
        );
    }

    #[test]
    fn test_resolve_project_output_dir_rejects_traversal_tokens() {
        let base = Path::new("/tmp/downloads");
        let err = project::resolve_project_output_dir(base, Some("../secret"))
            .expect_err("traversal token invalid");
        assert!(err.to_string().contains("cannot contain '.' or '..'"));
    }

    #[test]
    fn test_resolve_project_output_dir_rejects_empty_segments() {
        let base = Path::new("/tmp/downloads");
        let err = project::resolve_project_output_dir(base, Some("Climate//2024"))
            .expect_err("empty segments should fail");
        assert!(err.to_string().contains("empty path segment"));
    }

    #[test]
    fn test_windows_reserved_project_name_is_normalized() {
        let base = Path::new("/tmp/downloads");
        let output = project::resolve_project_output_dir(base, Some("CON"))
            .expect("reserved project names should be normalized");
        assert_eq!(output, PathBuf::from("/tmp/downloads/CON-project"));
        assert!(project::is_windows_reserved_name("con"));
    }

    #[test]
    fn test_leading_dot_project_name_is_normalized() {
        let base = Path::new("/tmp/downloads");
        let output = project::resolve_project_output_dir(base, Some(".Climate Research"))
            .expect("should resolve");
        assert_eq!(output, PathBuf::from("/tmp/downloads/Climate-Research"));
    }

    #[test]
    fn test_project_name_is_truncated_to_safe_length() {
        let base = Path::new("/tmp/downloads");
        let long_name = "A".repeat(120);
        let output = project::resolve_project_output_dir(base, Some(&long_name))
            .expect("long names should resolve");
        let folder_name = output
            .file_name()
            .and_then(|v: &std::ffi::OsStr| v.to_str())
            .unwrap_or("");
        assert_eq!(
            folder_name.chars().count(),
            project::MAX_PROJECT_FOLDER_CHARS
        );
    }

    #[test]
    fn test_resolve_project_output_dir_rejects_depth_over_limit() {
        let base = Path::new("/tmp/downloads");
        let deep = (0..=project::MAX_PROJECT_SEGMENTS)
            .map(|i| format!("n{i}"))
            .collect::<Vec<String>>()
            .join("/");
        let err = project::resolve_project_output_dir(base, Some(&deep))
            .expect_err("depth over limit should fail");
        assert!(err.to_string().contains("nesting depth"));
    }

    #[test]
    fn test_discover_history_db_paths_finds_root_and_nested_projects() {
        let root = TempDir::new().unwrap();
        let root_db = root.path().join(".downloader/queue.db");
        let nested_db = root.path().join("ProjectA/Study/.downloader/queue.db");

        std::fs::create_dir_all(root_db.parent().unwrap()).unwrap();
        std::fs::create_dir_all(nested_db.parent().unwrap()).unwrap();
        std::fs::write(&root_db, b"").unwrap();
        std::fs::write(&nested_db, b"").unwrap();

        let discovered = project::discover_history_db_paths(root.path()).unwrap();

        assert_eq!(discovered.len(), 2);
        assert!(discovered.contains(&root_db));
        assert!(discovered.contains(&nested_db));
    }

    #[tokio::test]
    async fn test_append_project_index_creates_index_with_entries() {
        let db = Database::new_in_memory().await.unwrap();
        let queue = Queue::new(db);
        let output_dir = TempDir::new().unwrap();

        let metadata = QueueMetadata {
            suggested_filename: Some("Smith_2024_Climate_Study.pdf".to_string()),
            title: Some("Climate Study".to_string()),
            authors: Some("Smith, John".to_string()),
            year: Some("2024".to_string()),
            doi: Some("10.1000/test".to_string()),
            topics: None,
            parse_confidence: None,
            parse_confidence_factors: None,
        };
        let id = queue
            .enqueue_with_metadata(
                "https://example.com/paper.pdf",
                "doi",
                Some("10.1000/test"),
                Some(&metadata),
            )
            .await
            .unwrap();
        let saved_path = output_dir.path().join("Smith_2024_Climate_Study.pdf");
        std::fs::write(&saved_path, b"pdf").unwrap();
        queue
            .mark_completed_with_path(id, Some(&saved_path))
            .await
            .unwrap();

        append_project_index(&queue, output_dir.path(), &HashSet::new())
            .await
            .unwrap();

        let index = std::fs::read_to_string(output_dir.path().join("index.md")).unwrap();
        assert!(index.contains("# Project Index"));
        assert!(index.contains("| Filename | Title | Authors | Source URL |"));
        assert!(index.contains("Smith_2024_Climate_Study.pdf"));
        assert!(index.contains("Climate Study"));
        assert!(index.contains("https://example.com/paper.pdf"));
    }

    #[tokio::test]
    async fn test_append_project_index_preserves_existing_content() {
        let db = Database::new_in_memory().await.unwrap();
        let queue = Queue::new(db);
        let output_dir = TempDir::new().unwrap();
        let index_path = output_dir.path().join("index.md");
        std::fs::write(
            &index_path,
            "# Project Index\n\n## Session existing\n\nold row\n",
        )
        .unwrap();

        let metadata = QueueMetadata {
            suggested_filename: Some("Doe_2025_Energy.pdf".to_string()),
            title: Some("Energy Analysis".to_string()),
            authors: Some("Doe, Jane".to_string()),
            year: Some("2025".to_string()),
            doi: Some("10.1000/energy".to_string()),
            topics: None,
            parse_confidence: None,
            parse_confidence_factors: None,
        };
        let id = queue
            .enqueue_with_metadata(
                "https://example.org/energy.pdf",
                "doi",
                Some("10.1000/energy"),
                Some(&metadata),
            )
            .await
            .unwrap();
        let saved_path = output_dir.path().join("Doe_2025_Energy.pdf");
        std::fs::write(&saved_path, b"pdf").unwrap();
        queue
            .mark_completed_with_path(id, Some(&saved_path))
            .await
            .unwrap();

        append_project_index(&queue, output_dir.path(), &HashSet::new())
            .await
            .unwrap();

        let index = std::fs::read_to_string(index_path).unwrap();
        assert!(index.contains("## Session existing"));
        assert!(index.contains("old row"));
        assert!(index.contains("Doe_2025_Energy.pdf"));
    }

    #[tokio::test]
    async fn test_append_project_index_only_writes_newly_completed_items() {
        let db = Database::new_in_memory().await.unwrap();
        let queue = Queue::new(db);
        let output_dir = TempDir::new().unwrap();

        let old_metadata = QueueMetadata {
            suggested_filename: Some("Old_2023_Item.pdf".to_string()),
            title: Some("Old Item".to_string()),
            authors: Some("Legacy, User".to_string()),
            year: Some("2023".to_string()),
            doi: Some("10.1000/old".to_string()),
            topics: None,
            parse_confidence: None,
            parse_confidence_factors: None,
        };
        let old_id = queue
            .enqueue_with_metadata(
                "https://example.com/old.pdf",
                "doi",
                Some("10.1000/old"),
                Some(&old_metadata),
            )
            .await
            .unwrap();
        let old_saved = output_dir.path().join("Old_2023_Item.pdf");
        std::fs::write(&old_saved, b"pdf").unwrap();
        queue
            .mark_completed_with_path(old_id, Some(&old_saved))
            .await
            .unwrap();

        let mut completed_before = HashSet::new();
        completed_before.insert(old_id);

        let new_metadata = QueueMetadata {
            suggested_filename: Some("New_2026_Item.pdf".to_string()),
            title: Some("New Item".to_string()),
            authors: Some("Current, User".to_string()),
            year: Some("2026".to_string()),
            doi: Some("10.1000/new".to_string()),
            topics: None,
            parse_confidence: None,
            parse_confidence_factors: None,
        };
        let new_id = queue
            .enqueue_with_metadata(
                "https://example.com/new.pdf",
                "doi",
                Some("10.1000/new"),
                Some(&new_metadata),
            )
            .await
            .unwrap();
        let new_saved = output_dir.path().join("New_2026_Item.pdf");
        std::fs::write(&new_saved, b"pdf").unwrap();
        queue
            .mark_completed_with_path(new_id, Some(&new_saved))
            .await
            .unwrap();

        append_project_index(&queue, output_dir.path(), &completed_before)
            .await
            .unwrap();

        let index = std::fs::read_to_string(output_dir.path().join("index.md")).unwrap();
        assert!(index.contains("## Session unix-"));
        assert!(index.contains("New_2026_Item.pdf"));
        assert!(!index.contains("Old_2023_Item.pdf"));
    }

    #[test]
    fn test_render_project_download_log_section_includes_required_fields() {
        let attempts = vec![DownloadAttempt {
            id: 7,
            url: "https://example.com/paper.pdf".to_string(),
            status_str: "failed".to_string(),
            file_path: None,
            title: Some("Paper Title".to_string()),
            authors: Some("Doe, Jane".to_string()),
            doi: Some("10.1000/example".to_string()),
            parse_confidence: Some("low".to_string()),
            parse_confidence_factors: Some(
                r#"{"has_authors":false,"has_year":true,"has_title":false,"author_count":0}"#
                    .to_string(),
            ),
            project: Some("/tmp/project".to_string()),
            started_at: "2026-02-17 12:00:00".to_string(),
            error_message: Some(
                "HTTP 404 downloading https://example.com/paper.pdf\n  Suggestion: Verify source"
                    .to_string(),
            ),
            error_type: Some("not_found".to_string()),
            retry_count: 0,
            last_retry_at: None,
            original_input: Some("10.1000/example".to_string()),
            http_status: Some(404),
            duration_ms: Some(250),
        }];

        let section = render_project_download_log_section("unix-1", &attempts);
        assert!(section.contains("## Session unix-1"));
        assert!(section.contains("(1 attempts)"));
        assert!(section.contains("FAILED"));
        assert!(section.contains("file=Paper Title"));
        assert!(section.contains("source=10.1000/example"));
        assert!(section.contains("reason=HTTP 404 downloading https://example.com/paper.pdf"));
        assert!(section.contains("ref=history#7"));
    }

    #[tokio::test]
    async fn test_append_project_download_log_writes_only_new_history_rows() {
        let db = Database::new_in_memory().await.unwrap();
        let queue = Queue::new(db);
        let output_dir = TempDir::new().unwrap();
        let project_key = project::project_history_key(output_dir.path());

        let first = NewDownloadAttempt {
            url: "https://example.com/first.pdf",
            final_url: Some("https://example.com/first.pdf"),
            status: DownloadAttemptStatus::Success,
            file_path: Some("/tmp/first.pdf"),
            file_size: Some(100),
            content_type: Some("application/pdf"),
            error_message: None,
            error_type: None,
            retry_count: 0,
            project: Some(&project_key),
            original_input: Some("https://example.com/first.pdf"),
            http_status: Some(200),
            duration_ms: Some(10),
            title: Some("First"),
            authors: Some("Author A"),
            doi: None,
            topics: None,
            parse_confidence: None,
            parse_confidence_factors: None,
        };
        let first_id = queue.log_download_attempt(&first).await.unwrap();

        append_project_download_log(&queue, output_dir.path(), None)
            .await
            .unwrap();
        let log_path = output_dir.path().join("download.log");
        let first_log = std::fs::read_to_string(&log_path).unwrap();
        assert!(first_log.contains(&format!("ref=history#{first_id}")));

        let second = NewDownloadAttempt {
            url: "https://example.com/second.pdf",
            final_url: None,
            status: DownloadAttemptStatus::Failed,
            file_path: None,
            file_size: None,
            content_type: None,
            error_message: Some("HTTP 404 downloading https://example.com/second.pdf"),
            error_type: Some(DownloadErrorType::NotFound),
            retry_count: 1,
            project: Some(&project_key),
            original_input: Some("10.2000/second"),
            http_status: Some(404),
            duration_ms: Some(22),
            title: Some("Second"),
            authors: Some("Author B"),
            doi: Some("10.2000/second"),
            topics: None,
            parse_confidence: None,
            parse_confidence_factors: None,
        };
        let second_id = queue.log_download_attempt(&second).await.unwrap();

        append_project_download_log(&queue, output_dir.path(), Some(first_id))
            .await
            .unwrap();
        let second_log = std::fs::read_to_string(&log_path).unwrap();
        assert!(second_log.contains(&format!("ref=history#{first_id}")));
        assert!(second_log.contains(&format!("ref=history#{second_id}")));
        assert_eq!(
            second_log
                .matches(&format!("ref=history#{first_id}"))
                .count(),
            1,
            "first history row should not be duplicated"
        );
    }

    #[tokio::test]
    async fn test_generate_sidecars_for_completed_returns_zero_when_only_historical_items_exist() {
        let db = Database::new_in_memory().await.unwrap();
        let queue = Queue::new(db);
        let output_dir = TempDir::new().unwrap();

        let id = queue
            .enqueue("https://example.com/historical.pdf", "direct_url", None)
            .await
            .unwrap();
        let saved_path = output_dir.path().join("historical.pdf");
        std::fs::write(&saved_path, b"historical").unwrap();
        queue
            .mark_completed_with_path(id, Some(&saved_path))
            .await
            .unwrap();

        let mut completed_before = HashSet::new();
        completed_before.insert(id);

        let created = generate_sidecars_for_completed(&queue, &completed_before).await;
        assert_eq!(created, 0, "historical-only items should be skipped");
        assert!(
            !output_dir.path().join("historical.json").exists(),
            "no sidecar should be written for historical-only items"
        );
    }

    #[tokio::test]
    async fn test_generate_sidecars_for_completed_continues_after_write_error() {
        let db = Database::new_in_memory().await.unwrap();
        let queue = Queue::new(db);
        let output_dir = TempDir::new().unwrap();

        let bad_id = queue
            .enqueue("https://example.com/bad.pdf", "direct_url", None)
            .await
            .unwrap();
        let bad_saved_path = output_dir.path().join("missing-dir").join("bad.pdf");
        queue
            .mark_completed_with_path(bad_id, Some(&bad_saved_path))
            .await
            .unwrap();

        let good_id = queue
            .enqueue("https://example.com/good.pdf", "direct_url", None)
            .await
            .unwrap();
        let good_saved_path = output_dir.path().join("good.pdf");
        std::fs::write(&good_saved_path, b"good").unwrap();
        queue
            .mark_completed_with_path(good_id, Some(&good_saved_path))
            .await
            .unwrap();

        let created = generate_sidecars_for_completed(&queue, &HashSet::new()).await;
        assert_eq!(created, 1, "one valid sidecar should still be created");
        assert!(
            output_dir.path().join("good.json").exists(),
            "valid sidecar should be written despite earlier failure"
        );
    }

    #[tokio::test]
    async fn test_generate_sidecars_for_completed_creates_only_new_completed_items() {
        // GIVEN: one historical completed item and one newly completed item
        let db = Database::new_in_memory().await.unwrap();
        let queue = Queue::new(db);
        let output_dir = TempDir::new().unwrap();

        let historical_id = queue
            .enqueue("https://example.com/historical.pdf", "direct_url", None)
            .await
            .unwrap();
        let historical_saved = output_dir.path().join("historical.pdf");
        std::fs::write(&historical_saved, b"historical").unwrap();
        queue
            .mark_completed_with_path(historical_id, Some(&historical_saved))
            .await
            .unwrap();

        let new_id = queue
            .enqueue("https://example.com/new.pdf", "direct_url", None)
            .await
            .unwrap();
        let new_saved = output_dir.path().join("new.pdf");
        std::fs::write(&new_saved, b"new").unwrap();
        queue
            .mark_completed_with_path(new_id, Some(&new_saved))
            .await
            .unwrap();

        let mut completed_before = HashSet::new();
        completed_before.insert(historical_id);

        // WHEN: generating sidecars only for items completed in this run
        let created = generate_sidecars_for_completed(&queue, &completed_before).await;

        // THEN: only the new item gets a sidecar
        assert_eq!(created, 1);
        assert!(
            !output_dir.path().join("historical.json").exists(),
            "historical item should be excluded from sidecar generation"
        );
        assert!(
            output_dir.path().join("new.json").exists(),
            "newly completed item should receive sidecar"
        );
    }

    #[test]
    fn test_map_history_status_variants() {
        assert_eq!(
            map_history_status(crate::cli::HistoryStatusArg::Success),
            DownloadAttemptStatus::Success
        );
        assert_eq!(
            map_history_status(crate::cli::HistoryStatusArg::Failed),
            DownloadAttemptStatus::Failed
        );
        assert_eq!(
            map_history_status(crate::cli::HistoryStatusArg::Skipped),
            DownloadAttemptStatus::Skipped
        );
    }

    fn make_search_candidate(
        id: i64,
        started_at: &str,
        title: Option<&str>,
        authors: Option<&str>,
        doi: Option<&str>,
        file_path: Option<&str>,
    ) -> DownloadSearchCandidate {
        DownloadSearchCandidate {
            id,
            url: format!("https://example.com/{id}.pdf"),
            status_str: "success".to_string(),
            file_path: file_path.map(std::string::ToString::to_string),
            title: title.map(std::string::ToString::to_string),
            authors: authors.map(std::string::ToString::to_string),
            doi: doi.map(std::string::ToString::to_string),
            started_at: started_at.to_string(),
        }
    }

    #[test]
    fn test_rank_search_candidates_prioritizes_exact_substring_then_fuzzy() {
        let candidates = vec![
            make_search_candidate(
                1,
                "2026-02-01 00:00:00",
                Some("attention"),
                Some("Doe, Jane"),
                None,
                Some("/tmp/one.pdf"),
            ),
            make_search_candidate(
                2,
                "2026-02-02 00:00:00",
                Some("Attention Is All You Need"),
                Some("Doe, John"),
                None,
                Some("/tmp/two.pdf"),
            ),
            make_search_candidate(
                3,
                "2026-02-03 00:00:00",
                Some("Attenton Is All You Need"),
                Some("Doe, Alex"),
                None,
                Some("/tmp/three.pdf"),
            ),
        ];

        let ranked = search::rank_search_candidates("attention", candidates);
        assert_eq!(ranked.len(), 3);
        assert_eq!(ranked[0].candidate.id, 1, "exact match should rank first");
        assert_eq!(
            ranked[1].candidate.id, 2,
            "substring match should rank second"
        );
        assert_eq!(ranked[2].candidate.id, 3, "fuzzy match should rank last");
    }

    #[test]
    fn test_rank_search_candidates_uses_recency_tie_breaker_for_same_score() {
        let older = make_search_candidate(
            10,
            "2026-02-01 00:00:00",
            Some("Attention Is All You Need"),
            None,
            None,
            Some("/tmp/older.pdf"),
        );
        let newer = make_search_candidate(
            11,
            "2026-02-02 00:00:00",
            Some("Attention Is All You Need"),
            None,
            None,
            Some("/tmp/newer.pdf"),
        );
        let ranked = search::rank_search_candidates("attention", vec![older, newer]);
        assert_eq!(ranked.len(), 2);
        assert_eq!(ranked[0].candidate.id, 11);
        assert_eq!(ranked[1].candidate.id, 10);
    }

    #[test]
    fn test_classify_search_match_typos_hit_fuzzy_threshold() {
        let query = search::normalize_search_text("attenton");
        let classified = search::classify_search_match(&query, Some("Attention Is All You Need"));
        assert!(classified.is_some(), "typo should still fuzzy-match");
        let (kind, _similarity) = classified.unwrap();
        assert_eq!(kind, search::SearchMatchKind::Fuzzy);
    }

    #[test]
    fn test_compare_search_results_is_deterministic_with_id_tie_breaker() {
        let left = search::RankedSearchResult {
            candidate: make_search_candidate(
                20,
                "2026-02-02 00:00:00",
                Some("Attention"),
                None,
                None,
                Some("/tmp/left.pdf"),
            ),
            match_kind: search::SearchMatchKind::Exact,
            similarity: 1.0,
            matched_field: "title",
        };
        let right = search::RankedSearchResult {
            candidate: make_search_candidate(
                21,
                "2026-02-02 00:00:00",
                Some("Attention"),
                None,
                None,
                Some("/tmp/right.pdf"),
            ),
            match_kind: search::SearchMatchKind::Exact,
            similarity: 1.0,
            matched_field: "title",
        };
        assert_eq!(
            search::compare_search_results(&left, &right),
            std::cmp::Ordering::Greater
        );
        assert_eq!(
            search::compare_search_results(&right, &left),
            std::cmp::Ordering::Less
        );
    }

    #[test]
    fn test_render_search_cli_row_includes_date_title_path_and_match_field() {
        let result = search::RankedSearchResult {
            candidate: make_search_candidate(
                30,
                "2026-02-03 10:00:00",
                Some("Attention Is All You Need"),
                Some("Vaswani, Ashish"),
                None,
                Some("/tmp/attention.pdf"),
            ),
            match_kind: search::SearchMatchKind::Substring,
            similarity: 0.8,
            matched_field: "title",
        };

        let rendered = render_search_cli_row(&result, 200);
        assert!(rendered.contains("2026-02-03 10:00:00"));
        assert!(rendered.contains("Attention Is All You Need"));
        assert!(rendered.contains("/tmp/attention.pdf"));
        assert!(rendered.contains("match=title"));
    }

    #[test]
    fn test_search_result_title_or_file_falls_back_to_filename() {
        let result = search::RankedSearchResult {
            candidate: make_search_candidate(
                31,
                "2026-02-03 10:00:00",
                None,
                Some("Doe, Jane"),
                None,
                Some("/tmp/fallback-file.pdf"),
            ),
            match_kind: search::SearchMatchKind::Substring,
            similarity: 0.7,
            matched_field: "authors",
        };

        assert_eq!(search_result_title_or_file(&result), "fallback-file.pdf");
    }

    #[test]
    fn test_validate_search_date_range_accepts_inclusive_bounds() {
        assert!(
            validate_search_date_range(Some("2026-01-01 00:00:00"), Some("2026-01-01 00:00:00"),)
                .is_ok()
        );
        assert!(
            validate_search_date_range(Some("2026-01-01 00:00:00"), Some("2026-01-02 00:00:00"),)
                .is_ok()
        );
    }

    #[test]
    fn test_validate_search_date_range_rejects_inverted_bounds() {
        let err =
            validate_search_date_range(Some("2026-01-03 00:00:00"), Some("2026-01-02 00:00:00"))
                .expect_err("inverted date bounds should fail");
        let message = err.to_string();
        assert!(message.contains("What: Invalid search date range"));
        assert!(message.contains("--since"));
        assert!(message.contains("--until"));
    }

    #[test]
    fn test_resolve_search_candidate_file_path_joins_relative_path_against_history_root() {
        let mut candidate = make_search_candidate(
            32,
            "2026-02-03 10:00:00",
            Some("Relative"),
            Some("Doe, Jane"),
            None,
            Some("papers/relative.pdf"),
        );

        resolve_search_candidate_file_path(
            &mut candidate,
            Path::new("/tmp/workspace/.downloader/queue.db"),
        );

        assert_eq!(
            candidate.file_path.as_deref(),
            Some("/tmp/workspace/papers/relative.pdf")
        );
    }

    #[test]
    fn test_resolve_search_candidate_file_path_keeps_absolute_path() {
        let mut candidate = make_search_candidate(
            33,
            "2026-02-03 10:00:00",
            Some("Absolute"),
            Some("Doe, Jane"),
            None,
            Some("/tmp/already-absolute.pdf"),
        );

        resolve_search_candidate_file_path(
            &mut candidate,
            Path::new("/tmp/workspace/.downloader/queue.db"),
        );

        assert_eq!(
            candidate.file_path.as_deref(),
            Some("/tmp/already-absolute.pdf")
        );
    }

    #[test]
    fn test_build_open_command_invocation_uses_arg_list() {
        let invocation = build_open_command_invocation(Path::new("/tmp/a b.pdf"));
        if cfg!(target_os = "macos") {
            assert_eq!(invocation.program, "open");
            assert_eq!(invocation.args, vec!["/tmp/a b.pdf".to_string()]);
        } else if cfg!(target_os = "windows") {
            assert_eq!(invocation.program, "explorer");
            assert_eq!(invocation.args, vec!["/tmp/a b.pdf".to_string()]);
        } else {
            assert_eq!(invocation.program, "xdg-open");
            assert_eq!(invocation.args, vec!["/tmp/a b.pdf".to_string()]);
        }
    }

    #[test]
    fn test_open_path_with_runner_missing_path_returns_actionable_error_without_invoking_runner() {
        let mut called = false;
        let err =
            open_path_with_runner(Path::new("/tmp/does-not-exist-12345.pdf"), |_invocation| {
                called = true;
                unreachable!("runner should not be called for missing path");
            })
            .expect_err("missing path should fail before command invocation");

        let message = err.to_string();
        assert!(message.contains("What: Cannot open search result file"));
        assert!(message.contains("Why: File does not exist"));
        assert!(message.contains("Fix: Re-run without --open or redownload the item."));
        assert!(!called, "runner must not be invoked when path is missing");
    }

    #[test]
    fn test_render_history_cli_row_failed_includes_fix_line() {
        let attempt = DownloadAttempt {
            id: 11,
            url: "https://example.com/failure.pdf".to_string(),
            status_str: "failed".to_string(),
            file_path: None,
            title: Some("Failure Title".to_string()),
            authors: None,
            doi: None,
            parse_confidence: Some("low".to_string()),
            parse_confidence_factors: Some(
                r#"{"has_authors":false,"has_year":true,"has_title":false,"author_count":0}"#
                    .to_string(),
            ),
            project: Some("/tmp/project".to_string()),
            started_at: "2026-02-17 16:00:00".to_string(),
            error_message: Some(
                "HTTP 404 downloading https://example.com/failure.pdf\n  Suggestion: Verify source"
                    .to_string(),
            ),
            error_type: Some("not_found".to_string()),
            retry_count: 0,
            last_retry_at: None,
            original_input: Some("10.1234/failure".to_string()),
            http_status: Some(404),
            duration_ms: Some(50),
        };

        let row = render_history_cli_row(&attempt, true, 120);
        assert!(row.contains("FAILED"));
        assert!(row.contains("confidence=low"));
        assert!(row.contains("❌ What: Source not found"));
        assert!(row.contains("Why:"));
        assert!(row.contains("Fix: Verify the source URL/DOI/reference"));
    }

    #[test]
    fn test_render_history_cli_row_uses_fallback_fix_for_legacy_rows() {
        let attempt = DownloadAttempt {
            id: 12,
            url: "https://proxy.example.com/failure.pdf".to_string(),
            status_str: "failed".to_string(),
            file_path: None,
            title: Some("Proxy Failure".to_string()),
            authors: None,
            doi: None,
            parse_confidence: None,
            parse_confidence_factors: None,
            project: Some("/tmp/project".to_string()),
            started_at: "2026-02-17 16:05:00".to_string(),
            error_message: Some(
                "HTTP 407 downloading https://proxy.example.com/failure.pdf".to_string(),
            ),
            error_type: Some("auth".to_string()),
            retry_count: 0,
            last_retry_at: None,
            original_input: Some("https://proxy.example.com/failure.pdf".to_string()),
            http_status: Some(407),
            duration_ms: Some(50),
        };

        let row = render_history_cli_row(&attempt, true, 120);
        assert!(row.contains("🔐 What:"));
        assert!(row.contains("Fix: Configure your HTTP proxy settings"));
    }

    #[test]
    fn test_escape_markdown_cell_escapes_backticks_pipes_and_newlines() {
        let escaped = output::escape_markdown_cell("A|B\nline`one\rline2");
        assert_eq!(escaped, "A\\|B line\\`one line2");
    }
}
