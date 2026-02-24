use anyhow::{Result, bail};
use clap::{ArgMatches, CommandFactory, FromArgMatches, parser::ValueSource};
use downloader_core::DatabaseOptions;

use crate::app_config::{FileConfig, VerbositySetting};
use crate::cli::{Cli, DownloadArgs};

const DEFAULT_DOWNLOAD_CONNECT_TIMEOUT_SECS: u64 = 30;
const DEFAULT_DOWNLOAD_READ_TIMEOUT_SECS: u64 = 300;
const DEFAULT_RESOLVER_CONNECT_TIMEOUT_SECS: u64 = 10;
const DEFAULT_RESOLVER_READ_TIMEOUT_SECS: u64 = 30;

/// Conservative values when --respectful is set (overrides -c/-l/-r).
pub(crate) const RESPECTFUL_CONCURRENCY: u8 = 2;
pub(crate) const RESPECTFUL_RATE_LIMIT_MS: u64 = 3000;
pub(crate) const RESPECTFUL_MAX_RETRIES: u8 = 1;

#[derive(Debug, Clone, Copy, Default)]
pub(crate) struct CliValueSources {
    pub(crate) output_dir: bool,
    pub(crate) concurrency: bool,
    pub(crate) rate_limit: bool,
    pub(crate) respectful: bool,
    pub(crate) check_robots: bool,
    pub(crate) verbose: bool,
    pub(crate) quiet: bool,
    pub(crate) debug: bool,
    pub(crate) detect_topics: bool,
    pub(crate) topics_file: bool,
    pub(crate) sidecar: bool,
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct HttpTimeoutSettings {
    pub(crate) download_connect_secs: u64,
    pub(crate) download_read_secs: u64,
    pub(crate) resolver_connect_secs: u64,
    pub(crate) resolver_read_secs: u64,
}

impl Default for HttpTimeoutSettings {
    fn default() -> Self {
        Self {
            download_connect_secs: DEFAULT_DOWNLOAD_CONNECT_TIMEOUT_SECS,
            download_read_secs: DEFAULT_DOWNLOAD_READ_TIMEOUT_SECS,
            resolver_connect_secs: DEFAULT_RESOLVER_CONNECT_TIMEOUT_SECS,
            resolver_read_secs: DEFAULT_RESOLVER_READ_TIMEOUT_SECS,
        }
    }
}

pub(crate) fn parse_cli_with_sources() -> (Cli, CliValueSources) {
    let command = Cli::command();
    let matches = command.get_matches();
    let cli = Cli::from_arg_matches(&matches).unwrap_or_else(|err| err.exit());

    let sources = CliValueSources {
        output_dir: is_commandline_value(&matches, "output_dir"),
        concurrency: is_commandline_value(&matches, "concurrency"),
        rate_limit: is_commandline_value(&matches, "rate_limit"),
        respectful: is_commandline_value(&matches, "respectful"),
        check_robots: is_commandline_value(&matches, "check_robots"),
        verbose: is_commandline_value(&matches, "verbose"),
        quiet: is_commandline_value(&matches, "quiet"),
        debug: is_commandline_value(&matches, "debug"),
        detect_topics: is_commandline_value(&matches, "detect_topics"),
        topics_file: is_commandline_value(&matches, "topics_file"),
        sidecar: is_commandline_value(&matches, "sidecar"),
    };
    (cli, sources)
}

fn is_commandline_value(matches: &ArgMatches, id: &str) -> bool {
    matches.value_source(id) == Some(ValueSource::CommandLine)
}

pub(crate) fn apply_config_defaults(
    mut args: DownloadArgs,
    cli_sources: &CliValueSources,
    file_config: Option<&FileConfig>,
) -> Result<DownloadArgs> {
    if let Some(file_config) = file_config {
    if !cli_sources.output_dir
        && args.output_dir.is_none()
        && let Some(output_dir) = &file_config.output_dir
    {
        args.output_dir = Some(output_dir.clone());
    }

    if !cli_sources.concurrency
        && let Some(concurrency) = file_config.concurrency
    {
        args.concurrency = concurrency;
    }

    if !cli_sources.rate_limit
        && let Some(rate_limit) = file_config.rate_limit
    {
        args.rate_limit = rate_limit;
    }

    if !cli_sources.respectful
        && let Some(respectful) = file_config.respectful
    {
        args.respectful = respectful;
    }

    if !cli_sources.check_robots
        && let Some(check_robots) = file_config.check_robots
    {
        args.check_robots = check_robots;
    }

    if !cli_sources.verbose
        && !cli_sources.quiet
        && !cli_sources.debug
        && let Some(verbosity) = file_config.verbosity
    {
        apply_config_verbosity(&mut args, verbosity);
    }

    if !cli_sources.detect_topics
        && !args.detect_topics
        && let Some(detect_topics) = file_config.detect_topics
    {
        args.detect_topics = detect_topics;
    }

    if !cli_sources.topics_file
        && args.topics_file.is_none()
        && let Some(topics_file) = &file_config.topics_file
    {
        args.topics_file = Some(topics_file.clone());
    }

    if !cli_sources.sidecar
        && !args.sidecar
        && let Some(sidecar) = file_config.sidecar
    {
        args.sidecar = sidecar;
    }
    }

    // When --respectful is set, override concurrency, rate_limit, max_retries (plan: respectful wins).
    if args.respectful {
        args.concurrency = RESPECTFUL_CONCURRENCY;
        args.rate_limit = RESPECTFUL_RATE_LIMIT_MS;
        args.max_retries = RESPECTFUL_MAX_RETRIES;
        if args.rate_limit_jitter == 0 {
            args.rate_limit_jitter = 1000;
        }
        args.check_robots = true;
    }

    if !(1..=100).contains(&args.concurrency) {
        bail!(
            "Invalid effective concurrency value: {}. Expected range: 1..=100",
            args.concurrency
        );
    }
    if args.rate_limit > 60_000 {
        bail!(
            "Invalid effective rate_limit value: {}. Expected range: 0..=60000",
            args.rate_limit
        );
    }

    Ok(args)
}

fn apply_config_verbosity(args: &mut DownloadArgs, verbosity: VerbositySetting) {
    match verbosity {
        VerbositySetting::Default => {
            args.quiet = false;
            args.debug = false;
            args.verbose = 0;
        }
        VerbositySetting::Verbose => {
            args.quiet = false;
            args.debug = false;
            args.verbose = 1;
        }
        VerbositySetting::Quiet => {
            args.quiet = true;
            args.debug = false;
            args.verbose = 0;
        }
        VerbositySetting::Debug => {
            args.quiet = false;
            args.debug = true;
            args.verbose = 0;
        }
    }
}

pub(crate) fn resolve_http_timeouts(file_config: Option<&FileConfig>) -> HttpTimeoutSettings {
    let mut settings = HttpTimeoutSettings::default();
    let Some(file_config) = file_config else {
        return settings;
    };

    if let Some(value) = file_config.download_connect_timeout_secs {
        settings.download_connect_secs = value;
    }
    if let Some(value) = file_config.download_read_timeout_secs {
        settings.download_read_secs = value;
    }
    if let Some(value) = file_config.resolver_connect_timeout_secs {
        settings.resolver_connect_secs = value;
    }
    if let Some(value) = file_config.resolver_read_timeout_secs {
        settings.resolver_read_secs = value;
    }
    settings
}

pub(crate) fn resolve_db_options(file_config: Option<&FileConfig>) -> DatabaseOptions {
    let mut options = DatabaseOptions::default();
    let Some(file_config) = file_config else {
        return options;
    };
    if let Some(n) = file_config.db_max_connections {
        options.max_connections = n;
    }
    if let Some(ms) = file_config.db_busy_timeout_ms {
        options.busy_timeout_ms = ms;
    }
    options
}

pub(crate) fn resolve_default_log_level(args: &DownloadArgs) -> &'static str {
    if args.quiet {
        "error"
    } else if args.debug {
        "trace"
    } else {
        match args.verbose {
            0 => "info",
            1 => "debug",
            _ => "trace",
        }
    }
}

pub(crate) fn should_force_cli_log_level(cli_sources: &CliValueSources) -> bool {
    cli_sources.verbose || cli_sources.quiet || cli_sources.debug
}

pub(crate) fn verbosity_label(verbose: u8, quiet: bool, debug: bool) -> &'static str {
    if debug {
        VerbositySetting::Debug.as_str()
    } else if quiet {
        VerbositySetting::Quiet.as_str()
    } else if verbose == 0 {
        VerbositySetting::Default.as_str()
    } else if verbose == 1 {
        VerbositySetting::Verbose.as_str()
    } else {
        VerbositySetting::Debug.as_str()
    }
}
