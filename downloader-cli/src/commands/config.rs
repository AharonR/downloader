//! Config command handlers: show effective configuration.

use std::path::PathBuf;

use anyhow::Result;

use crate::CliValueSources;
use crate::app_config::load_default_file_config;
use crate::cli::DownloadArgs;

pub fn run_config_show_command(
    download_args: &DownloadArgs,
    cli_sources: &CliValueSources,
) -> Result<()> {
    let loaded_config = load_default_file_config()?;
    let effective = crate::apply_config_defaults(
        download_args.clone(),
        cli_sources,
        loaded_config.config.as_ref(),
    )?;
    let effective_output_dir = effective.output_dir.unwrap_or_else(|| PathBuf::from("."));

    let resolved_path = loaded_config.path.as_ref().map_or_else(
        || "<unresolved>".to_string(),
        |path| path.display().to_string(),
    );
    println!("config_path = {resolved_path}");
    println!(
        "config_file = {}",
        if loaded_config.loaded_from_file {
            "loaded"
        } else {
            "not found (using defaults)"
        }
    );
    println!("output_dir = {}", effective_output_dir.display());
    println!("concurrency = {}", effective.concurrency);
    println!("rate_limit = {}", effective.rate_limit);
    println!("rate_limit_jitter = {}", effective.rate_limit_jitter);
    println!("max_retries = {}", effective.max_retries);
    println!("respectful = {}", effective.respectful);
    println!("check_robots = {}", effective.check_robots);
    println!(
        "verbosity = {}",
        crate::verbosity_label(effective.verbose, effective.quiet, effective.debug)
    );

    Ok(())
}
