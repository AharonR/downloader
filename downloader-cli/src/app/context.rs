//! Shared runtime context built after CLI/config and input handling.

use std::path::PathBuf;
use std::sync::Arc;

use reqwest::cookie::Jar;

use crate::app::config_runtime::HttpTimeoutSettings;
use crate::cli::DownloadArgs;
use downloader_core::{DatabaseOptions, ParsedItem};

/// Holds shared state built during startup so the rest of `run_downloader`
/// can use `ctx.args`, `ctx.output_dir`, etc., instead of passing many arguments.
pub(crate) struct RunContext {
    pub(crate) args: DownloadArgs,
    pub(crate) http_timeouts: HttpTimeoutSettings,
    pub(crate) db_options: DatabaseOptions,
    pub(crate) output_dir: PathBuf,
    pub(crate) cookie_jar: Option<Arc<Jar>>,
    pub(crate) input_text: Option<String>,
    pub(crate) piped_stdin_was_empty: bool,
    /// Pre-parsed items from bibliography files (`--bibliography`).
    ///
    /// These are injected directly into the resolution pipeline alongside
    /// items produced by `parse_input(input_text)`, preserving full metadata
    /// (title, authors, year) extracted from `.bib` and `.ris` files.
    pub(crate) bibliography_items: Vec<ParsedItem>,
}
