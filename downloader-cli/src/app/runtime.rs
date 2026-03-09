use std::collections::HashSet;
use std::fs;
use std::io::{self, BufRead as _, IsTerminal, Write as _};
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use anyhow::{Result, bail};
use downloader_core::QueueStatus;
use tracing::{debug, info, warn};

use crate::app::{
    command_dispatcher, config_manager, config_runtime, context, download_orchestrator,
    exit_handler, input_processor, progress_manager, queue_manager, resolution_orchestrator,
    terminal,
};
use crate::app_config::{load_default_file_config, write_tos_acknowledged};
use crate::{ProcessExit, commands, output, project};

/// Checks whether the user has acknowledged their Terms of Service responsibilities.
///
/// - If the config already records `tos_acknowledged = true`, returns immediately.
/// - If stdin is not a TTY (non-interactive) and `quiet` is false, prints a one-line warning
///   and continues.
/// - If stdin is a TTY, shows a brief acknowledgment prompt.  A `y`/`Y` response records the
///   acknowledgment in the config file and continues.  Any other response (including Ctrl-C or
///   empty) exits with a helpful message.
fn check_tos_acknowledgment(quiet: bool) -> Result<ProcessExit> {
    let loaded = load_default_file_config()?;
    let already_acked = loaded
        .config
        .as_ref()
        .and_then(|c| c.tos_acknowledged)
        .unwrap_or(false);

    if already_acked {
        return Ok(ProcessExit::Success);
    }

    let is_tty = io::stdin().is_terminal();

    if !is_tty {
        if !quiet {
            // Non-interactive: warn once but do not block.
            eprintln!(
                "WARNING: Downloader automates HTTP requests on your behalf. You are \
                 responsible for complying with each publisher's Terms of Service. Run \
                 interactively once to acknowledge this and suppress this warning."
            );
        }
        return Ok(ProcessExit::Success);
    }

    // Interactive: show the acknowledgment prompt.
    eprintln!();
    eprintln!("==========================================================================");
    eprintln!("  Downloader — Terms of Service Responsibility Notice");
    eprintln!("==========================================================================");
    eprintln!();
    eprintln!("  This tool automates HTTP downloads on your behalf. By using it you");
    eprintln!("  acknowledge that:");
    eprintln!();
    eprintln!("  1. You are responsible for complying with the Terms of Service of every");
    eprintln!("     publisher or repository you access.");
    eprintln!();
    eprintln!("  2. This tool does not bypass paywalls, authentication, or DRM. It can");
    eprintln!("     only download content you are already entitled to access.");
    eprintln!();
    eprintln!("  3. If you access content through an institutional license, verify that");
    eprintln!("     automated downloading is permitted under your license agreement.");
    eprintln!();
    eprintln!("  4. The authors of this tool accept no liability for your use of it.");
    eprintln!();
    eprintln!("  See README.md § Responsible Use for rate-limit guidance and the full");
    eprintln!("  legal risk assessment.");
    eprintln!();
    eprint!("  Do you acknowledge? [y/N] ");
    io::stderr().flush().ok();

    let mut line = String::new();
    let read_result = io::stdin().lock().read_line(&mut line);

    match read_result {
        Ok(0) | Err(_) => {
            // EOF or read error — treat as no.
            eprintln!();
            eprintln!("No acknowledgment received. Exiting.");
            eprintln!(
                "Run the tool interactively and type 'y' to acknowledge, or set \
                 `tos_acknowledged = true` in your config file."
            );
            return Ok(ProcessExit::Failure);
        }
        Ok(_) => {}
    }

    let trimmed = line.trim();
    if trimmed.eq_ignore_ascii_case("y") {
        // Persist the acknowledgment so we never prompt again.
        if let Err(err) = write_tos_acknowledged() {
            // Non-fatal: warn but proceed.
            warn!(?err, "Could not persist tos_acknowledged to config file");
        }
        eprintln!();
        return Ok(ProcessExit::Success);
    }

    eprintln!();
    eprintln!("Acknowledgment declined. Exiting.");
    eprintln!(
        "Run the tool again and type 'y' to acknowledge, or set \
         `tos_acknowledged = true` in your config file."
    );
    Ok(ProcessExit::Failure)
}

pub(crate) async fn run_downloader() -> Result<ProcessExit> {
    let (cli, cli_sources) = config_runtime::parse_cli_with_sources();

    if let Some(exit) = command_dispatcher::try_dispatch(&cli, &cli_sources).await? {
        return Ok(exit);
    }

    let tos_result = check_tos_acknowledgment(cli.download.quiet)?;
    if tos_result != ProcessExit::Success {
        return Ok(tos_result);
    }

    let resolved = config_manager::resolve_config(&cli, &cli_sources)?;

    let default_level = config_runtime::resolve_default_log_level(&resolved.args);
    let force_cli_log_level = config_runtime::should_force_cli_log_level(&cli_sources);
    let no_color = terminal::is_no_color_requested(&resolved.args);
    terminal::init_tracing(default_level, force_cli_log_level, no_color);

    debug!("CLI arguments parsed");
    info!("Downloader starting");

    let base_output_dir = resolved
        .args
        .output_dir
        .clone()
        .unwrap_or_else(|| PathBuf::from("."));
    let output_dir =
        project::resolve_project_output_dir(&base_output_dir, resolved.args.project.as_deref())?;
    if resolved.args.project.is_some() {
        info!(project_dir = %output_dir.display(), "Project folder ready");
    }

    let (cookie_jar, input_text, piped_stdin_was_empty, bibliography_items) =
        input_processor::process_input(&resolved.args)?;

    let ctx = context::RunContext {
        args: resolved.args,
        http_timeouts: resolved.http_timeouts,
        db_options: resolved.db_options,
        output_dir,
        cookie_jar,
        input_text,
        piped_stdin_was_empty,
        bibliography_items,
    };

    if ctx.args.dry_run {
        if let Some(input_text) = ctx.input_text.as_deref() {
            commands::run_dry_run_preview(input_text, ctx.cookie_jar.clone()).await?;
        } else if !ctx.bibliography_items.is_empty() {
            // Bibliography-only dry run: report item count as a preview.
            output::print_bibliography_dry_run_summary(ctx.bibliography_items.len());
        } else if ctx.piped_stdin_was_empty {
            output::print_quick_start_guidance(true);
        } else {
            output::print_quick_start_guidance(false);
        }
        return Ok(ProcessExit::Success);
    }

    let state_dir = ctx.output_dir.join(".downloader");
    let has_prior_state = state_dir.exists();

    if ctx.input_text.is_none() && ctx.bibliography_items.is_empty() && !has_prior_state {
        output::print_quick_start_guidance(ctx.piped_stdin_was_empty);
        return Ok(ProcessExit::Success);
    }

    if !ctx.output_dir.exists() {
        fs::create_dir_all(&ctx.output_dir)?;
        info!(dir = %ctx.output_dir.display(), "Created output directory");
    }

    let (queue, history_start_id) =
        queue_manager::create_queue(&ctx.output_dir, &ctx.db_options).await?;

    let resolution = resolution_orchestrator::run_resolution(&ctx, Arc::clone(&queue)).await?;

    if resolution.parsed_item_count > 0
        && resolution.resolution_failed_count == resolution.parsed_item_count
    {
        let first_error = resolution
            .first_resolution_error
            .as_deref()
            .unwrap_or("unknown resolver failure");
        bail!(
            "All parsed items failed URL resolution ({}/{}).\n  \
             First error: {first_error}",
            resolution.resolution_failed_count,
            resolution.parsed_item_count
        );
    }

    let pending_items = queue.list_by_status(QueueStatus::Pending).await?;
    let total_queued = pending_items.len();
    let uncertain_references_in_run = pending_items
        .iter()
        .filter(|item| item.parse_confidence.as_deref() == Some("low"))
        .count();

    if total_queued == 0 {
        info!("No queue items were enqueued for downloading");
        return Ok(ProcessExit::Success);
    }

    let completed_before: HashSet<i64> = queue
        .list_by_status(QueueStatus::Completed)
        .await?
        .into_iter()
        .map(|item| item.id)
        .collect();

    let interrupted = Arc::new(AtomicBool::new(false));
    let interrupted_signal = Arc::clone(&interrupted);
    tokio::spawn(async move {
        if tokio::signal::ctrl_c().await.is_ok() {
            interrupted_signal.store(true, Ordering::SeqCst);
        }
    });

    let use_spinner = terminal::should_use_spinner(
        io::stderr().is_terminal(),
        ctx.args.quiet,
        terminal::is_dumb_terminal(),
    );
    let (progress_handle, progress_stop) =
        progress_manager::spawn_progress_ui(use_spinner, Arc::clone(&queue), total_queued);

    let stats =
        download_orchestrator::run_download(&ctx, Arc::clone(&queue), Arc::clone(&interrupted))
            .await?;

    progress_stop.store(true, Ordering::SeqCst);
    if let Some(handle) = progress_handle {
        let _ = handle.await;
    }

    info!(
        completed = stats.completed(),
        failed = stats.failed(),
        retried = stats.retried(),
        total_queued,
        "Download complete"
    );

    output::print_completion_summary(
        queue.as_ref(),
        &ctx.output_dir,
        &stats,
        total_queued,
        ctx.args.project.as_ref().map(|_| ctx.output_dir.as_path()),
        uncertain_references_in_run,
    )
    .await?;

    if ctx.args.sidecar {
        let count =
            project::generate_sidecars_for_completed(queue.as_ref(), &completed_before).await;
        if count > 0 {
            info!(count, "Generated sidecar files");
        }
    }

    if stats.was_interrupted() || interrupted.load(Ordering::SeqCst) {
        warn!(
            completed = stats.completed(),
            total_queued, "Interrupted. Run again to resume."
        );
        return Ok(ProcessExit::Failure);
    }

    if ctx.args.project.is_some() {
        project::append_project_download_log(queue.as_ref(), &ctx.output_dir, history_start_id)
            .await?;
        project::append_project_index(queue.as_ref(), &ctx.output_dir, &completed_before).await?;
    }

    Ok(exit_handler::determine_exit_outcome(
        stats.completed(),
        stats.failed(),
    ))
}
