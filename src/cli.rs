//! CLI argument definitions using clap derive macros.

use clap::Parser;

use downloader_core::{DEFAULT_CONCURRENCY, DEFAULT_MAX_RETRIES};

/// Batch download and organize reference documents.
///
/// Downloader transforms curated lists of sources (URLs, DOIs, bibliographies)
/// into organized, searchable, LLM-ready knowledge.
#[derive(Parser, Debug)]
#[command(name = "downloader")]
#[command(author, version, about)]
pub struct Args {
    /// Increase output verbosity (-v for debug, -vv for trace)
    #[arg(short, long, action = clap::ArgAction::Count)]
    pub verbose: u8,

    /// Suppress non-error output
    #[arg(short, long)]
    pub quiet: bool,

    /// Maximum concurrent downloads (1-100)
    #[arg(short = 'c', long, default_value_t = DEFAULT_CONCURRENCY as u8, value_parser = clap::value_parser!(u8).range(1..=100))]
    pub concurrency: u8,

    /// Maximum retry attempts for transient failures (0-10)
    #[arg(short = 'r', long, default_value_t = DEFAULT_MAX_RETRIES as u8, value_parser = clap::value_parser!(u8).range(0..=10))]
    pub max_retries: u8,

    /// Minimum delay between requests to same domain in milliseconds (0 to disable, max 60000)
    #[arg(short = 'l', long, default_value_t = 1000, value_parser = clap::value_parser!(u64).range(0..=60000))]
    pub rate_limit: u64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cli_default_args_parses_successfully() {
        let args = Args::try_parse_from(["downloader"]).unwrap();
        assert_eq!(args.verbose, 0);
        assert!(!args.quiet);
        assert_eq!(args.concurrency, 10); // DEFAULT_CONCURRENCY
        assert_eq!(args.max_retries, 3); // DEFAULT_MAX_RETRIES
    }

    #[test]
    fn test_cli_verbose_flag_increments_count() {
        let args = Args::try_parse_from(["downloader", "-v"]).unwrap();
        assert_eq!(args.verbose, 1);

        let args = Args::try_parse_from(["downloader", "-vv"]).unwrap();
        assert_eq!(args.verbose, 2);

        let args = Args::try_parse_from(["downloader", "--verbose", "--verbose"]).unwrap();
        assert_eq!(args.verbose, 2);
    }

    #[test]
    fn test_cli_quiet_flag_sets_quiet() {
        let args = Args::try_parse_from(["downloader", "-q"]).unwrap();
        assert!(args.quiet);

        let args = Args::try_parse_from(["downloader", "--quiet"]).unwrap();
        assert!(args.quiet);
    }

    #[test]
    fn test_cli_help_flag_shows_usage() {
        // --help causes early exit, so we check it returns an error with Help kind
        let result = Args::try_parse_from(["downloader", "--help"]);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert_eq!(err.kind(), clap::error::ErrorKind::DisplayHelp);
    }

    #[test]
    fn test_cli_version_flag_shows_version() {
        // --version causes early exit, so we check it returns an error with Version kind
        let result = Args::try_parse_from(["downloader", "--version"]);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert_eq!(err.kind(), clap::error::ErrorKind::DisplayVersion);
    }

    #[test]
    fn test_cli_invalid_flag_returns_error() {
        let result = Args::try_parse_from(["downloader", "--invalid-flag"]);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert_eq!(err.kind(), clap::error::ErrorKind::UnknownArgument);
    }

    #[test]
    fn test_cli_concurrency_short_flag() {
        let args = Args::try_parse_from(["downloader", "-c", "5"]).unwrap();
        assert_eq!(args.concurrency, 5);
    }

    #[test]
    fn test_cli_concurrency_long_flag() {
        let args = Args::try_parse_from(["downloader", "--concurrency", "20"]).unwrap();
        assert_eq!(args.concurrency, 20);
    }

    #[test]
    fn test_cli_concurrency_min_value() {
        let args = Args::try_parse_from(["downloader", "-c", "1"]).unwrap();
        assert_eq!(args.concurrency, 1);
    }

    #[test]
    fn test_cli_concurrency_max_value() {
        let args = Args::try_parse_from(["downloader", "-c", "100"]).unwrap();
        assert_eq!(args.concurrency, 100);
    }

    #[test]
    fn test_cli_concurrency_zero_rejected() {
        let result = Args::try_parse_from(["downloader", "-c", "0"]);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert_eq!(err.kind(), clap::error::ErrorKind::ValueValidation);
    }

    #[test]
    fn test_cli_concurrency_over_max_rejected() {
        let result = Args::try_parse_from(["downloader", "-c", "101"]);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert_eq!(err.kind(), clap::error::ErrorKind::ValueValidation);
    }

    // ==================== Max Retries Tests ====================

    #[test]
    fn test_cli_max_retries_short_flag() {
        let args = Args::try_parse_from(["downloader", "-r", "5"]).unwrap();
        assert_eq!(args.max_retries, 5);
    }

    #[test]
    fn test_cli_max_retries_long_flag() {
        let args = Args::try_parse_from(["downloader", "--max-retries", "7"]).unwrap();
        assert_eq!(args.max_retries, 7);
    }

    #[test]
    fn test_cli_max_retries_zero_allowed() {
        // 0 retries means no retry, just single attempt
        let args = Args::try_parse_from(["downloader", "-r", "0"]).unwrap();
        assert_eq!(args.max_retries, 0);
    }

    #[test]
    fn test_cli_max_retries_max_value() {
        let args = Args::try_parse_from(["downloader", "-r", "10"]).unwrap();
        assert_eq!(args.max_retries, 10);
    }

    #[test]
    fn test_cli_max_retries_over_max_rejected() {
        let result = Args::try_parse_from(["downloader", "-r", "11"]);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert_eq!(err.kind(), clap::error::ErrorKind::ValueValidation);
    }

    #[test]
    fn test_cli_combined_concurrency_and_retries() {
        let args = Args::try_parse_from(["downloader", "-c", "5", "-r", "2"]).unwrap();
        assert_eq!(args.concurrency, 5);
        assert_eq!(args.max_retries, 2);
    }

    // ==================== Rate Limit Tests ====================

    #[test]
    fn test_cli_rate_limit_default() {
        let args = Args::try_parse_from(["downloader"]).unwrap();
        assert_eq!(args.rate_limit, 1000); // Default 1000ms
    }

    #[test]
    fn test_cli_rate_limit_short_flag() {
        let args = Args::try_parse_from(["downloader", "-l", "2000"]).unwrap();
        assert_eq!(args.rate_limit, 2000);
    }

    #[test]
    fn test_cli_rate_limit_long_flag() {
        let args = Args::try_parse_from(["downloader", "--rate-limit", "500"]).unwrap();
        assert_eq!(args.rate_limit, 500);
    }

    #[test]
    fn test_cli_rate_limit_zero_disables() {
        let args = Args::try_parse_from(["downloader", "-l", "0"]).unwrap();
        assert_eq!(args.rate_limit, 0);
    }

    #[test]
    fn test_cli_rate_limit_max_value() {
        let args = Args::try_parse_from(["downloader", "-l", "60000"]).unwrap();
        assert_eq!(args.rate_limit, 60000);
    }

    #[test]
    fn test_cli_rate_limit_over_max_rejected() {
        let result = Args::try_parse_from(["downloader", "-l", "60001"]);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert_eq!(err.kind(), clap::error::ErrorKind::ValueValidation);
    }

    #[test]
    fn test_cli_combined_all_flags() {
        let args =
            Args::try_parse_from(["downloader", "-c", "20", "-r", "5", "-l", "2000"]).unwrap();
        assert_eq!(args.concurrency, 20);
        assert_eq!(args.max_retries, 5);
        assert_eq!(args.rate_limit, 2000);
    }
}
