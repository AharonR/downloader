//! CLI argument definitions using clap derive macros.

use std::path::PathBuf;

use clap::{Args as ClapArgs, Parser, Subcommand, ValueEnum};

use downloader_core::{DEFAULT_CONCURRENCY, DEFAULT_MAX_RETRIES};

/// Batch download and organize reference documents.
///
/// Downloader transforms curated lists of sources (URLs, DOIs, bibliographies)
/// into organized, searchable, LLM-ready knowledge.
#[derive(Parser, Debug)]
#[command(name = "downloader")]
#[command(author, version, about)]
#[command(
    after_help = "Exit codes:\n  0 = all items succeeded\n  1 = partial success (some failed)\n  2 = complete failure or fatal error"
)]
#[command(args_conflicts_with_subcommands = true)]
#[command(subcommand_precedence_over_arg = true)]
pub struct Cli {
    /// Optional top-level command namespace.
    #[command(subcommand)]
    pub command: Option<Command>,

    #[command(flatten)]
    pub download: DownloadArgs,
}

/// Top-level command namespaces.
#[derive(Subcommand, Debug, PartialEq, Eq)]
pub enum Command {
    /// Authentication and persisted-cookie management commands.
    Auth {
        #[command(subcommand)]
        command: AuthCommand,
    },
    /// Query persisted download history.
    Log(LogArgs),
    /// Search persisted download history metadata.
    Search(SearchArgs),
    /// Manage downloader configuration.
    Config {
        #[command(subcommand)]
        command: ConfigCommand,
    },
}

/// Auth command variants.
#[derive(Subcommand, Debug, PartialEq, Eq)]
pub enum AuthCommand {
    /// Capture browser cookies from stdin/path and validate them.
    Capture(AuthCaptureArgs),
    /// Clear persisted encrypted cookies.
    Clear,
}

/// Config command variants.
#[derive(Subcommand, Debug, PartialEq, Eq)]
pub enum ConfigCommand {
    /// Show effective configuration values.
    Show,
}

/// Arguments for `downloader auth capture`.
#[derive(ClapArgs, Debug, PartialEq, Eq)]
pub struct AuthCaptureArgs {
    /// Persist cookies securely (encrypted at rest) for future runs.
    #[arg(long)]
    pub save_cookies: bool,
}

/// Status filter values for `downloader log --status`.
#[derive(ValueEnum, Debug, Clone, Copy, PartialEq, Eq)]
pub enum HistoryStatusArg {
    Success,
    Failed,
    Skipped,
}

/// Arguments for `downloader log`.
#[derive(ClapArgs, Debug, Clone, PartialEq, Eq)]
pub struct LogArgs {
    /// Output directory root containing `.downloader/queue.db` (default: current directory).
    #[arg(short = 'o', long = "output-dir", value_name = "DIR")]
    pub output_dir: Option<PathBuf>,

    /// Project folder filter/name (uses same sanitization rules as download mode).
    #[arg(long, value_name = "NAME")]
    pub project: Option<String>,

    /// Filter by attempt status.
    #[arg(long, value_enum)]
    pub status: Option<HistoryStatusArg>,

    /// Filter rows started at/after a timestamp (SQLite datetime string).
    #[arg(long, value_name = "DATETIME")]
    pub since: Option<String>,

    /// Filter by source domain host.
    #[arg(long, value_name = "DOMAIN")]
    pub domain: Option<String>,

    /// Shortcut for failed attempts only.
    #[arg(long, conflicts_with = "status")]
    pub failed: bool,

    /// Show only low-confidence reference rows needing manual verification.
    #[arg(long, conflicts_with_all = ["status", "failed"])]
    pub uncertain: bool,

    /// Maximum rows to show (default 50, max 10000).
    #[arg(long, default_value_t = 50, value_parser = clap::value_parser!(u16).range(1..=10000))]
    pub limit: u16,
}

/// Arguments for `downloader search`.
#[derive(ClapArgs, Debug, Clone, PartialEq, Eq)]
pub struct SearchArgs {
    /// Search query text (matched against title, authors, and DOI).
    #[arg(value_name = "QUERY")]
    pub query: String,

    /// Output directory root containing `.downloader/queue.db` (default: current directory).
    #[arg(short = 'o', long = "output-dir", value_name = "DIR")]
    pub output_dir: Option<PathBuf>,

    /// Project folder filter/name (uses same sanitization rules as download mode).
    #[arg(long, value_name = "NAME")]
    pub project: Option<String>,

    /// Filter rows started at/after a timestamp (SQLite datetime string).
    #[arg(long, value_name = "DATETIME")]
    pub since: Option<String>,

    /// Filter rows started at/before a timestamp (SQLite datetime string).
    #[arg(long, value_name = "DATETIME")]
    pub until: Option<String>,

    /// Maximum result rows to display (default 50, max 10000).
    #[arg(long, default_value_t = 50, value_parser = clap::value_parser!(u16).range(1..=10000))]
    pub limit: u16,

    /// Open the top-ranked result file in the system default app.
    #[arg(long = "open")]
    pub open: bool,
}

/// Download-mode arguments (default command when no subcommand is provided).
#[derive(ClapArgs, Debug, Clone)]
pub struct DownloadArgs {
    /// Increase output verbosity (-v for debug, -vv for trace)
    #[arg(short, long, action = clap::ArgAction::Count, conflicts_with_all = ["quiet", "debug"])]
    pub verbose: u8,

    /// Suppress non-error output
    #[arg(short, long, conflicts_with_all = ["verbose", "debug"])]
    pub quiet: bool,

    /// Enable full debug tracing output.
    #[arg(long, conflicts_with_all = ["verbose", "quiet"])]
    pub debug: bool,

    /// Disable ANSI color/styling in CLI output.
    #[arg(long = "no-color")]
    pub no_color: bool,

    /// Parse and resolve input without downloading files or writing queue records.
    #[arg(short = 'n', long = "dry-run")]
    pub dry_run: bool,

    /// Maximum concurrent downloads (1-100)
    #[arg(short = 'c', long, default_value_t = DEFAULT_CONCURRENCY as u8, value_parser = clap::value_parser!(u8).range(1..=100))]
    pub concurrency: u8,

    /// Maximum retry attempts for transient failures (0-10)
    #[arg(short = 'r', long, default_value_t = DEFAULT_MAX_RETRIES as u8, value_parser = clap::value_parser!(u8).range(0..=10))]
    pub max_retries: u8,

    /// Minimum delay between requests to same domain in milliseconds (0 to disable, max 60000)
    #[arg(short = 'l', long, default_value_t = 1000, value_parser = clap::value_parser!(u64).range(0..=60000))]
    pub rate_limit: u64,

    /// Use conservative settings for sensitive environments (overrides -c/-l/-r with concurrency=2, rate_limit=3000, max_retries=1)
    #[arg(long)]
    pub respectful: bool,

    /// Max random jitter in ms added to rate-limit delay (0 = disabled). When --respectful, defaults to 1000.
    #[arg(long, default_value_t = 0, value_parser = clap::value_parser!(u64).range(0..=60000))]
    pub rate_limit_jitter: u64,

    /// Check robots.txt before downloading; when --respectful, this is enabled.
    #[arg(long)]
    pub check_robots: bool,

    /// Output directory for downloaded files (default: current directory)
    #[arg(short = 'o', long = "output-dir", value_name = "DIR")]
    pub output_dir: Option<PathBuf>,

    /// Project folder name for organizing downloads (e.g., "Climate Research")
    #[arg(long, value_name = "NAME")]
    pub project: Option<String>,

    /// Cookie file in Netscape format (use `-` for stdin)
    #[arg(long, value_name = "FILE")]
    pub cookies: Option<String>,

    /// Persist cookies securely (encrypted at rest) for future runs
    #[arg(long)]
    pub save_cookies: bool,

    /// Enable topic auto-detection from downloaded paper titles and abstracts (Story 8.1)
    #[arg(long = "detect-topics")]
    pub detect_topics: bool,

    /// Path to custom topics file (one topic per line, enables topic matching priority)
    #[arg(long = "topics-file", value_name = "FILE", requires = "detect_topics")]
    pub topics_file: Option<PathBuf>,

    /// Write a JSON-LD sidecar file alongside each downloaded file (Story 8.2)
    #[arg(long = "sidecar")]
    pub sidecar: bool,

    /// URLs to download (reads from stdin if not provided).
    /// Flags may appear before or after URLs. Use `--` to pass a URL that starts with `-`.
    pub urls: Vec<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse_download(args: impl IntoIterator<Item = &'static str>) -> DownloadArgs {
        let cli = Cli::try_parse_from(args).unwrap();
        assert!(cli.command.is_none());
        cli.download
    }

    fn parse_log(args: impl IntoIterator<Item = &'static str>) -> LogArgs {
        let cli = Cli::try_parse_from(args).unwrap();
        match cli.command {
            Some(Command::Log(log)) => log,
            _ => panic!("expected log command"),
        }
    }

    fn parse_search(args: impl IntoIterator<Item = &'static str>) -> SearchArgs {
        let cli = Cli::try_parse_from(args).unwrap();
        match cli.command {
            Some(Command::Search(search)) => search,
            _ => panic!("expected search command"),
        }
    }

    #[test]
    fn test_cli_default_args_parses_successfully() {
        let args = parse_download(["downloader"]);
        assert_eq!(args.verbose, 0);
        assert!(!args.quiet);
        assert!(!args.debug);
        assert!(!args.no_color);
        assert!(!args.dry_run);
        assert_eq!(args.concurrency, 10); // DEFAULT_CONCURRENCY
        assert_eq!(args.max_retries, 3); // DEFAULT_MAX_RETRIES
    }

    #[test]
    fn test_cli_verbose_flag_increments_count() {
        let args = parse_download(["downloader", "-v"]);
        assert_eq!(args.verbose, 1);

        let args = parse_download(["downloader", "-vv"]);
        assert_eq!(args.verbose, 2);

        let args = parse_download(["downloader", "--verbose", "--verbose"]);
        assert_eq!(args.verbose, 2);
    }

    #[test]
    fn test_cli_quiet_flag_sets_quiet() {
        let args = parse_download(["downloader", "-q"]);
        assert!(args.quiet);
        assert!(!args.debug);

        let args = parse_download(["downloader", "--quiet"]);
        assert!(args.quiet);
        assert!(!args.debug);
    }

    #[test]
    fn test_cli_debug_flag_sets_debug() {
        let args = parse_download(["downloader", "--debug"]);
        assert!(args.debug);
        assert!(!args.quiet);
        assert_eq!(args.verbose, 0);
    }

    #[test]
    fn test_cli_debug_conflicts_with_verbose() {
        let result = Cli::try_parse_from(["downloader", "--debug", "-v"]);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert_eq!(err.kind(), clap::error::ErrorKind::ArgumentConflict);
    }

    #[test]
    fn test_cli_debug_conflicts_with_quiet() {
        let result = Cli::try_parse_from(["downloader", "--debug", "-q"]);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert_eq!(err.kind(), clap::error::ErrorKind::ArgumentConflict);
    }

    #[test]
    fn test_cli_no_color_flag_sets_no_color() {
        let args = parse_download(["downloader", "--no-color"]);
        assert!(args.no_color);
    }

    #[test]
    fn test_cli_detect_topics_flag_sets_detect_topics() {
        let args = parse_download(["downloader", "--detect-topics"]);
        assert!(args.detect_topics);
        assert_eq!(args.topics_file, None);
    }

    #[test]
    fn test_cli_topics_file_requires_detect_topics() {
        // topics-file without detect-topics should fail
        let result = Cli::try_parse_from(["downloader", "--topics-file", "topics.txt"]);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert_eq!(err.kind(), clap::error::ErrorKind::MissingRequiredArgument);
    }

    #[test]
    fn test_cli_topics_file_with_detect_topics_succeeds() {
        let args = parse_download([
            "downloader",
            "--detect-topics",
            "--topics-file",
            "custom.txt",
        ]);
        assert!(args.detect_topics);
        assert_eq!(args.topics_file, Some(PathBuf::from("custom.txt")));
    }

    #[test]
    fn test_cli_sidecar_flag_sets_sidecar() {
        let args = parse_download(["downloader", "--sidecar"]);
        assert!(args.sidecar);
    }

    #[test]
    fn test_cli_sidecar_flag_defaults_to_false() {
        let args = parse_download(["downloader"]);
        assert!(!args.sidecar);
    }

    #[test]
    fn test_cli_sidecar_flag_with_url() {
        let args = parse_download(["downloader", "--sidecar", "https://example.com/paper.pdf"]);
        assert!(args.sidecar);
        assert_eq!(args.urls, vec!["https://example.com/paper.pdf"]);
    }

    #[test]
    fn test_cli_dry_run_long_flag_sets_true() {
        let args = parse_download(["downloader", "--dry-run"]);
        assert!(args.dry_run);
    }

    #[test]
    fn test_cli_dry_run_short_flag_sets_true() {
        let args = parse_download(["downloader", "-n"]);
        assert!(args.dry_run);
    }

    #[test]
    fn test_cli_help_flag_shows_usage() {
        // --help causes early exit, so we check it returns an error with Help kind
        let result = Cli::try_parse_from(["downloader", "--help"]);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert_eq!(err.kind(), clap::error::ErrorKind::DisplayHelp);
    }

    #[test]
    fn test_cli_help_includes_exit_code_documentation() {
        let err = Cli::try_parse_from(["downloader", "--help"]).unwrap_err();
        let rendered = err.to_string();
        assert!(rendered.contains("Exit codes:"));
        assert!(rendered.contains("0 = all items succeeded"));
        assert!(rendered.contains("1 = partial success"));
        assert!(rendered.contains("2 = complete failure or fatal error"));
    }

    #[test]
    fn test_cli_version_flag_shows_version() {
        // --version causes early exit, so we check it returns an error with Version kind
        let result = Cli::try_parse_from(["downloader", "--version"]);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert_eq!(err.kind(), clap::error::ErrorKind::DisplayVersion);
    }

    #[test]
    fn test_cli_invalid_flag_returns_error() {
        let result = Cli::try_parse_from(["downloader", "--invalid-flag"]);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert_eq!(err.kind(), clap::error::ErrorKind::UnknownArgument);
    }

    #[test]
    fn test_cli_concurrency_short_flag() {
        let args = parse_download(["downloader", "-c", "5"]);
        assert_eq!(args.concurrency, 5);
    }

    #[test]
    fn test_cli_concurrency_long_flag() {
        let args = parse_download(["downloader", "--concurrency", "20"]);
        assert_eq!(args.concurrency, 20);
    }

    #[test]
    fn test_cli_concurrency_min_value() {
        let args = parse_download(["downloader", "-c", "1"]);
        assert_eq!(args.concurrency, 1);
    }

    #[test]
    fn test_cli_concurrency_max_value() {
        let args = parse_download(["downloader", "-c", "100"]);
        assert_eq!(args.concurrency, 100);
    }

    #[test]
    fn test_cli_concurrency_zero_rejected() {
        let result = Cli::try_parse_from(["downloader", "-c", "0"]);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert_eq!(err.kind(), clap::error::ErrorKind::ValueValidation);
    }

    #[test]
    fn test_cli_concurrency_over_max_rejected() {
        let result = Cli::try_parse_from(["downloader", "-c", "101"]);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert_eq!(err.kind(), clap::error::ErrorKind::ValueValidation);
    }

    // ==================== Max Retries Tests ====================

    #[test]
    fn test_cli_max_retries_short_flag() {
        let args = parse_download(["downloader", "-r", "5"]);
        assert_eq!(args.max_retries, 5);
    }

    #[test]
    fn test_cli_max_retries_long_flag() {
        let args = parse_download(["downloader", "--max-retries", "7"]);
        assert_eq!(args.max_retries, 7);
    }

    #[test]
    fn test_cli_max_retries_zero_allowed() {
        // 0 retries means no retry, just single attempt
        let args = parse_download(["downloader", "-r", "0"]);
        assert_eq!(args.max_retries, 0);
    }

    #[test]
    fn test_cli_max_retries_max_value() {
        let args = parse_download(["downloader", "-r", "10"]);
        assert_eq!(args.max_retries, 10);
    }

    #[test]
    fn test_cli_max_retries_over_max_rejected() {
        let result = Cli::try_parse_from(["downloader", "-r", "11"]);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert_eq!(err.kind(), clap::error::ErrorKind::ValueValidation);
    }

    #[test]
    fn test_cli_combined_concurrency_and_retries() {
        let args = parse_download(["downloader", "-c", "5", "-r", "2"]);
        assert_eq!(args.concurrency, 5);
        assert_eq!(args.max_retries, 2);
    }

    // ==================== Rate Limit Tests ====================

    #[test]
    fn test_cli_rate_limit_default() {
        let args = parse_download(["downloader"]);
        assert_eq!(args.rate_limit, 1000); // Default 1000ms
    }

    #[test]
    fn test_cli_rate_limit_short_flag() {
        let args = parse_download(["downloader", "-l", "2000"]);
        assert_eq!(args.rate_limit, 2000);
    }

    #[test]
    fn test_cli_rate_limit_long_flag() {
        let args = parse_download(["downloader", "--rate-limit", "500"]);
        assert_eq!(args.rate_limit, 500);
    }

    #[test]
    fn test_cli_rate_limit_zero_disables() {
        let args = parse_download(["downloader", "-l", "0"]);
        assert_eq!(args.rate_limit, 0);
    }

    #[test]
    fn test_cli_rate_limit_max_value() {
        let args = parse_download(["downloader", "-l", "60000"]);
        assert_eq!(args.rate_limit, 60000);
    }

    #[test]
    fn test_cli_rate_limit_over_max_rejected() {
        let result = Cli::try_parse_from(["downloader", "-l", "60001"]);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert_eq!(err.kind(), clap::error::ErrorKind::ValueValidation);
    }

    #[test]
    fn test_cli_combined_all_flags() {
        let args = parse_download(["downloader", "-c", "20", "-r", "5", "-l", "2000"]);
        assert_eq!(args.concurrency, 20);
        assert_eq!(args.max_retries, 5);
        assert_eq!(args.rate_limit, 2000);
    }

    #[test]
    fn test_cli_respectful_flag() {
        let args = parse_download(["downloader", "--respectful"]);
        assert!(args.respectful);
    }

    #[test]
    fn test_cli_respectful_default_off() {
        let args = parse_download(["downloader"]);
        assert!(!args.respectful);
    }

    // ==================== Positional URL Parsing ====================

    #[test]
    fn test_cli_flag_after_url_is_parsed_as_flag() {
        let args = parse_download(["downloader", "https://a.com/file.pdf", "-q"]);
        assert!(args.quiet);
        assert_eq!(args.urls, vec!["https://a.com/file.pdf"]);
    }

    #[test]
    fn test_cli_flag_before_url_is_parsed_as_flag() {
        let args = parse_download(["downloader", "-q", "https://a.com/file.pdf"]);
        assert!(args.quiet);
        assert_eq!(args.urls, vec!["https://a.com/file.pdf"]);
    }

    #[test]
    fn test_cli_flag_between_urls_is_parsed_as_flag() {
        let args = parse_download([
            "downloader",
            "https://a.com/file.pdf",
            "-r",
            "5",
            "https://b.com/file.pdf",
        ]);
        assert_eq!(args.max_retries, 5);
        assert_eq!(
            args.urls,
            vec!["https://a.com/file.pdf", "https://b.com/file.pdf"]
        );
    }

    #[test]
    fn test_cli_long_flag_after_url_is_parsed_as_flag() {
        let args = parse_download([
            "downloader",
            "https://a.com/file.pdf",
            "--concurrency",
            "20",
        ]);
        assert_eq!(args.concurrency, 20);
        assert_eq!(args.urls, vec!["https://a.com/file.pdf"]);
    }

    #[test]
    fn test_cli_invalid_flag_after_url_returns_error() {
        let result =
            Cli::try_parse_from(["downloader", "https://a.com/file.pdf", "--invalid-flag"]);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert_eq!(err.kind(), clap::error::ErrorKind::UnknownArgument);
    }

    // ==================== Cookies Flag Tests ====================

    #[test]
    fn test_cli_cookies_long_flag_with_file() {
        let args = parse_download([
            "downloader",
            "--cookies",
            "cookies.txt",
            "https://a.com/f.pdf",
        ]);
        assert_eq!(args.cookies, Some("cookies.txt".to_string()));
    }

    #[test]
    fn test_cli_cookies_stdin_dash() {
        let args = parse_download(["downloader", "--cookies", "-", "https://a.com/f.pdf"]);
        assert_eq!(args.cookies, Some("-".to_string()));
    }

    #[test]
    fn test_cli_cookies_not_set_by_default() {
        let args = parse_download(["downloader"]);
        assert!(args.project.is_none());
        assert!(args.cookies.is_none());
        assert!(!args.save_cookies);
    }

    #[test]
    fn test_cli_cookies_with_other_flags() {
        let args = parse_download([
            "downloader",
            "--cookies",
            "file.txt",
            "--save-cookies",
            "-c",
            "5",
            "https://a.com/f.pdf",
        ]);
        assert_eq!(args.cookies, Some("file.txt".to_string()));
        assert!(args.save_cookies);
        assert_eq!(args.concurrency, 5);
    }

    #[test]
    fn test_cli_save_cookies_flag_enabled() {
        let args = parse_download(["downloader", "--save-cookies", "https://a.com/f.pdf"]);
        assert!(args.save_cookies);
    }

    #[test]
    fn test_cli_separator_allows_dash_prefixed_url() {
        let args = parse_download(["downloader", "--", "-q"]);
        assert!(!args.quiet);
        assert_eq!(args.urls, vec!["-q"]);
    }

    #[test]
    fn test_cli_project_flag_parses_value() {
        let args = parse_download([
            "downloader",
            "--project",
            "Climate Research",
            "https://a.com/f.pdf",
        ]);
        assert_eq!(args.project, Some("Climate Research".to_string()));
    }

    #[test]
    fn test_cli_project_flag_with_output_dir() {
        let args = parse_download([
            "downloader",
            "--output-dir",
            "downloads",
            "--project",
            "Lab",
            "https://a.com/f.pdf",
        ]);
        assert_eq!(args.output_dir, Some(PathBuf::from("downloads")));
        assert_eq!(args.project, Some("Lab".to_string()));
    }

    // ==================== Auth Subcommand Tests ====================

    #[test]
    fn test_cli_auth_capture_save_cookies_parses() {
        let cli = Cli::try_parse_from(["downloader", "auth", "capture", "--save-cookies"])
            .expect("auth capture should parse");
        assert!(cli.download.urls.is_empty());
        assert!(matches!(
            cli.command,
            Some(Command::Auth {
                command: AuthCommand::Capture(AuthCaptureArgs { save_cookies: true })
            })
        ));
    }

    #[test]
    fn test_cli_auth_clear_parses() {
        let cli = Cli::try_parse_from(["downloader", "auth", "clear"]).expect("auth clear parses");
        assert!(matches!(
            cli.command,
            Some(Command::Auth {
                command: AuthCommand::Clear
            })
        ));
    }

    #[test]
    fn test_cli_auth_requires_subcommand() {
        let result = Cli::try_parse_from(["downloader", "auth"]);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert_eq!(
            err.kind(),
            clap::error::ErrorKind::DisplayHelpOnMissingArgumentOrSubcommand
        );
    }

    #[test]
    fn test_cli_auth_capture_unknown_flag_rejected() {
        let result = Cli::try_parse_from(["downloader", "auth", "capture", "--unknown"]);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert_eq!(err.kind(), clap::error::ErrorKind::UnknownArgument);
    }

    #[test]
    fn test_cli_auth_capture_rejects_download_cookie_flag() {
        let result =
            Cli::try_parse_from(["downloader", "auth", "capture", "--cookies", "cookies.txt"]);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert_eq!(err.kind(), clap::error::ErrorKind::UnknownArgument);
    }

    #[test]
    fn test_cli_auth_clear_rejects_save_cookies_flag() {
        let result = Cli::try_parse_from(["downloader", "auth", "clear", "--save-cookies"]);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert_eq!(err.kind(), clap::error::ErrorKind::UnknownArgument);
    }

    #[test]
    fn test_cli_download_flags_before_auth_parse_for_runtime_guard() {
        let cli = Cli::try_parse_from(["downloader", "-q", "auth", "clear"])
            .expect("parse should succeed for runtime guard coverage");
        assert!(cli.command.is_none());
        assert!(cli.download.quiet);
        assert_eq!(cli.download.urls, vec!["auth", "clear"]);
    }

    // ==================== History Log Command Tests ====================

    #[test]
    fn test_cli_log_command_parses_defaults() {
        let args = parse_log(["downloader", "log"]);
        assert_eq!(args.limit, 50);
        assert!(!args.failed);
        assert!(!args.uncertain);
        assert!(args.status.is_none());
        assert!(args.project.is_none());
        assert!(args.domain.is_none());
        assert!(args.since.is_none());
    }

    #[test]
    fn test_cli_log_command_parses_filters() {
        let args = parse_log([
            "downloader",
            "log",
            "--project",
            "Climate Research",
            "--status",
            "failed",
            "--since",
            "2026-02-01 00:00:00",
            "--domain",
            "example.com",
            "--limit",
            "125",
        ]);
        assert_eq!(args.project.as_deref(), Some("Climate Research"));
        assert_eq!(args.status, Some(HistoryStatusArg::Failed));
        assert_eq!(args.since.as_deref(), Some("2026-02-01 00:00:00"));
        assert_eq!(args.domain.as_deref(), Some("example.com"));
        assert_eq!(args.limit, 125);
        assert!(!args.uncertain);
    }

    #[test]
    fn test_cli_log_command_parses_uncertain_flag() {
        let args = parse_log(["downloader", "log", "--uncertain"]);
        assert!(args.uncertain);
        assert!(!args.failed);
        assert!(args.status.is_none());
    }

    #[test]
    fn test_cli_log_failed_conflicts_with_status() {
        let result = Cli::try_parse_from(["downloader", "log", "--failed", "--status", "success"]);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert_eq!(err.kind(), clap::error::ErrorKind::ArgumentConflict);
    }

    #[test]
    fn test_cli_log_uncertain_conflicts_with_status() {
        let result =
            Cli::try_parse_from(["downloader", "log", "--uncertain", "--status", "failed"]);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert_eq!(err.kind(), clap::error::ErrorKind::ArgumentConflict);
    }

    #[test]
    fn test_cli_log_uncertain_conflicts_with_failed() {
        let result = Cli::try_parse_from(["downloader", "log", "--uncertain", "--failed"]);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert_eq!(err.kind(), clap::error::ErrorKind::ArgumentConflict);
    }

    #[test]
    fn test_cli_log_limit_out_of_range_rejected() {
        let result = Cli::try_parse_from(["downloader", "log", "--limit", "10001"]);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert_eq!(err.kind(), clap::error::ErrorKind::ValueValidation);
    }

    // ==================== Search Command Tests ====================

    #[test]
    fn test_cli_search_command_parses_defaults() {
        let args = parse_search(["downloader", "search", "climate"]);
        assert_eq!(args.query, "climate");
        assert_eq!(args.limit, 50);
        assert!(!args.open);
        assert!(args.output_dir.is_none());
        assert!(args.project.is_none());
        assert!(args.since.is_none());
        assert!(args.until.is_none());
    }

    #[test]
    fn test_cli_search_command_parses_all_filters() {
        let args = parse_search([
            "downloader",
            "search",
            "attention is all you need",
            "--project",
            "Climate Research",
            "--since",
            "2026-02-01 00:00:00",
            "--until",
            "2026-02-10 00:00:00",
            "--output-dir",
            "/tmp/out",
            "--limit",
            "12",
            "--open",
        ]);
        assert_eq!(args.query, "attention is all you need");
        assert_eq!(args.project.as_deref(), Some("Climate Research"));
        assert_eq!(args.since.as_deref(), Some("2026-02-01 00:00:00"));
        assert_eq!(args.until.as_deref(), Some("2026-02-10 00:00:00"));
        assert_eq!(args.output_dir, Some(PathBuf::from("/tmp/out")));
        assert_eq!(args.limit, 12);
        assert!(args.open);
    }

    #[test]
    fn test_cli_search_command_requires_query() {
        let result = Cli::try_parse_from(["downloader", "search"]);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert_eq!(err.kind(), clap::error::ErrorKind::MissingRequiredArgument);
    }

    #[test]
    fn test_cli_search_limit_out_of_range_rejected() {
        let result = Cli::try_parse_from(["downloader", "search", "climate", "--limit", "0"]);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert_eq!(err.kind(), clap::error::ErrorKind::ValueValidation);
    }

    // ==================== Config Command Tests ====================

    #[test]
    fn test_cli_config_show_parses() {
        let cli = Cli::try_parse_from(["downloader", "config", "show"])
            .expect("config show should parse");
        assert!(matches!(
            cli.command,
            Some(Command::Config {
                command: ConfigCommand::Show
            })
        ));
        assert!(cli.download.urls.is_empty());
    }

    #[test]
    fn test_cli_config_requires_subcommand() {
        let result = Cli::try_parse_from(["downloader", "config"]);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert_eq!(
            err.kind(),
            clap::error::ErrorKind::DisplayHelpOnMissingArgumentOrSubcommand
        );
    }
}
