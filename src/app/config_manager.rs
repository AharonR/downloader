//! Configuration lifecycle: load file config, merge CLI, resolve timeouts and DB options.

use anyhow::Result;

use crate::app::config_runtime::{self, CliValueSources, HttpTimeoutSettings};
use crate::app_config::load_default_file_config;
use crate::cli::{Cli, DownloadArgs};
use downloader_core::{configure_resolver_http_timeouts, DatabaseOptions};

/// Resolved configuration bundle used to build RunContext.
/// Its fields are copied into RunContext in runtime; ResolvedConfig is not stored in RunContext.
pub(crate) struct ResolvedConfig {
    pub(crate) args: DownloadArgs,
    pub(crate) http_timeouts: HttpTimeoutSettings,
    pub(crate) db_options: DatabaseOptions,
}

/// Load file config, merge CLI overrides, resolve HTTP timeouts and DB options, apply resolver timeouts.
/// Single entry point that returns a resolved config bundle.
pub(crate) fn resolve_config(
    cli: &Cli,
    cli_sources: &CliValueSources,
) -> Result<ResolvedConfig> {
    let loaded_config = load_default_file_config()?;
    let args = config_runtime::apply_config_defaults(
        cli.download.clone(),
        cli_sources,
        loaded_config.config.as_ref(),
    )?;
    let http_timeouts = config_runtime::resolve_http_timeouts(loaded_config.config.as_ref());
    let db_options = config_runtime::resolve_db_options(loaded_config.config.as_ref());
    configure_resolver_http_timeouts(
        http_timeouts.resolver_connect_secs,
        http_timeouts.resolver_read_secs,
    );
    Ok(ResolvedConfig {
        args,
        http_timeouts,
        db_options,
    })
}

#[cfg(test)]
mod tests {
    use super::resolve_config;
    use crate::app::config_runtime::CliValueSources;
    use crate::cli::Cli;
    use clap::Parser;
    use downloader_core::DEFAULT_CONCURRENCY;
    use tempfile::TempDir;

    /// With no config file (XDG_CONFIG_HOME pointing at empty temp dir), resolve_config
    /// succeeds and returns default-like values for args and timeouts.
    #[test]
    fn test_resolve_config_no_config_file_returns_defaults() {
        let temp = TempDir::new().unwrap();
        let prev = std::env::var_os("XDG_CONFIG_HOME");
        // SAFETY: test isolates env change and restores on drop.
        unsafe { std::env::set_var("XDG_CONFIG_HOME", temp.path()); }
        let _restore = RestoreEnv::new("XDG_CONFIG_HOME", prev);

        let cli = Cli::try_parse_from(["downloader"]).unwrap();
        let sources = CliValueSources::default();
        let resolved = resolve_config(&cli, &sources);
        assert!(resolved.is_ok(), "resolve_config should succeed with no config file");
        let r = resolved.unwrap();

        assert_eq!(
            r.args.concurrency,
            DEFAULT_CONCURRENCY as u8,
            "concurrency should be default when no config"
        );
        assert_eq!(
            r.http_timeouts.download_connect_secs, 30,
            "download_connect_secs default"
        );
        assert_eq!(
            r.http_timeouts.download_read_secs, 300,
            "download_read_secs default"
        );
        assert!(
            r.args.output_dir.is_none(),
            "output_dir should be none when no config"
        );
    }

    /// Restores an env var to its previous value (or removes it) when dropped.
    struct RestoreEnv {
        key: &'static str,
        value: Option<std::ffi::OsString>,
    }
    impl RestoreEnv {
        fn new(key: &'static str, value: Option<std::ffi::OsString>) -> Self {
            Self { key, value }
        }
    }
    impl Drop for RestoreEnv {
        fn drop(&mut self) {
            // SAFETY: test restores env to prior state.
            match &self.value {
                Some(v) => unsafe { std::env::set_var(self.key, v) },
                None => unsafe { std::env::remove_var(self.key) },
            }
        }
    }
}
