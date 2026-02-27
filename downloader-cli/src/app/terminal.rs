use crate::cli::DownloadArgs;

pub(crate) fn no_color_env_requested() -> bool {
    std::env::var_os("NO_COLOR").is_some_and(|value| !value.is_empty())
}

pub(crate) fn is_dumb_terminal() -> bool {
    std::env::var("TERM")
        .map(|value| value.eq_ignore_ascii_case("dumb"))
        .unwrap_or(false)
}

pub(crate) fn should_disable_color(
    no_color_flag: bool,
    no_color_env: bool,
    dumb_terminal: bool,
) -> bool {
    no_color_flag || no_color_env || dumb_terminal
}

pub(crate) fn is_no_color_requested(args: &DownloadArgs) -> bool {
    should_disable_color(args.no_color, no_color_env_requested(), is_dumb_terminal())
}

pub(crate) fn should_use_spinner(
    stderr_is_terminal: bool,
    quiet: bool,
    dumb_terminal: bool,
) -> bool {
    stderr_is_terminal && !quiet && !dumb_terminal
}

pub(crate) fn init_tracing(default_level: &str, force_cli_level: bool, no_color: bool) {
    let filter = if force_cli_level {
        tracing_subscriber::EnvFilter::new(default_level)
    } else {
        tracing_subscriber::EnvFilter::try_from_default_env()
            .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new(default_level))
    };
    let _ = tracing_subscriber::fmt()
        .with_writer(std::io::stderr)
        .with_ansi(!no_color)
        .with_env_filter(filter)
        .try_init();
}
