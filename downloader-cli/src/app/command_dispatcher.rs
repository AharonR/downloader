//! CLI command routing: runs Auth, Log, Search, and Config subcommands.
//!
//! If the user invoked a top-level command (e.g. `downloader auth`, `downloader log`),
//! this module runs the corresponding handler and returns the exit outcome. Otherwise
//! returns `None` so the caller continues with the main download flow.

use anyhow::Result;

use crate::app::{config_runtime, terminal};
use crate::cli::{AuthCommand, Cli, Command, ConfigCommand};
use crate::{ProcessExit, commands};

/// If `cli` has a top-level command, run it and return `Some(exit)`; otherwise return `None`.
pub(crate) async fn try_dispatch(
    cli: &Cli,
    cli_sources: &config_runtime::CliValueSources,
) -> Result<Option<ProcessExit>> {
    let Some(command) = &cli.command else {
        return Ok(None);
    };

    let no_color = terminal::should_disable_color(
        false,
        terminal::no_color_env_requested(),
        terminal::is_dumb_terminal(),
    );
    terminal::init_tracing("info", false, no_color);

    match command {
        Command::Auth { command } => match command {
            AuthCommand::Capture(capture_args) => {
                commands::run_auth_capture_command(capture_args.save_cookies)?;
            }
            AuthCommand::Clear => {
                commands::run_auth_clear_command()?;
            }
        },
        Command::Log(log_args) => {
            commands::run_log_command(log_args).await?;
        }
        Command::Search(search_args) => {
            commands::run_search_command(search_args).await?;
        }
        Command::Config { command } => match command {
            ConfigCommand::Show => {
                commands::run_config_show_command(&cli.download, cli_sources)?;
            }
        },
    }

    Ok(Some(ProcessExit::Success))
}

#[cfg(test)]
mod tests {
    use super::try_dispatch;
    use crate::app::config_runtime;
    use crate::cli::Cli;
    use clap::Parser;

    /// When no top-level command is present, try_dispatch returns None so runtime continues to download flow.
    #[tokio::test]
    async fn test_try_dispatch_returns_none_when_no_command() {
        let cli = Cli::parse_from(["downloader"]);
        assert!(cli.command.is_none());
        let sources = config_runtime::CliValueSources::default();
        let result = try_dispatch(&cli, &sources).await.unwrap();
        assert_eq!(result, None);
    }
}
