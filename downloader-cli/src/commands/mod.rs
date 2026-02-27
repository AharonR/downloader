//! CLI command handlers.

mod auth;
mod config;
mod dry_run;
mod log;
mod search;

pub use auth::{run_auth_capture_command, run_auth_clear_command};
pub use config::run_config_show_command;
pub use dry_run::run_dry_run_preview;
pub use log::run_log_command;
pub use search::run_search_command;
