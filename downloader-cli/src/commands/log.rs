//! Log command handler: query and display download history.

use std::path::PathBuf;

use anyhow::Result;
use downloader_core::{Database, DownloadAttemptQuery, DownloadAttemptStatus, Queue};

use crate::cli::LogArgs;
use crate::map_history_status;
use crate::output;
use crate::project;
use crate::render_history_cli_row;

const PROJECT_LOG_QUERY_PAGE_SIZE: usize = 10_000;

pub async fn run_log_command(args: &LogArgs) -> Result<()> {
    let base_output_dir = args
        .output_dir
        .clone()
        .unwrap_or_else(|| PathBuf::from("."));
    let (history_scope_label, db_paths, project_scope_key) = if let Some(project_name) =
        args.project.as_deref()
    {
        let output_dir = project::resolve_project_output_dir(&base_output_dir, Some(project_name))?;
        let db_path = output_dir.join(".downloader").join("queue.db");
        (
            format!("project {}", output_dir.display()),
            vec![db_path],
            Some(project::project_history_key(&output_dir)),
        )
    } else {
        let db_paths = project::discover_history_db_paths(&base_output_dir)?;
        (
            format!("global under {}", base_output_dir.display()),
            db_paths,
            None,
        )
    };

    if db_paths.is_empty() {
        println!("No download history found for {history_scope_label}.");
        return Ok(());
    }

    let mut query = DownloadAttemptQuery {
        since: args.since.clone(),
        limit: usize::from(args.limit)
            .saturating_add(1)
            .min(PROJECT_LOG_QUERY_PAGE_SIZE),
        domain: args.domain.clone(),
        uncertain_only: args.uncertain,
        ..DownloadAttemptQuery::default()
    };
    if let Some(project_key) = project_scope_key {
        query.project = Some(project_key);
    }
    if args.failed {
        query.status = Some(DownloadAttemptStatus::Failed);
    } else if let Some(status) = args.status {
        query.status = Some(map_history_status(status));
    }

    let mut attempts = Vec::new();
    let mut maybe_capped_by_hard_limit = false;
    for db_path in &db_paths {
        let db = Database::new(db_path).await?;
        let queue = Queue::new(db);
        let db_attempts = queue.query_download_attempts(&query).await?;
        if usize::from(args.limit) == PROJECT_LOG_QUERY_PAGE_SIZE
            && db_attempts.len() == PROJECT_LOG_QUERY_PAGE_SIZE
        {
            maybe_capped_by_hard_limit = true;
        }
        attempts.extend(db_attempts);
    }

    attempts.sort_by(|left, right| {
        right
            .started_at
            .cmp(&left.started_at)
            .then_with(|| right.id.cmp(&left.id))
    });

    let requested_limit = usize::from(args.limit);
    let truncated = attempts.len() > requested_limit;
    if truncated {
        attempts.truncate(requested_limit);
    }

    if attempts.is_empty() {
        println!("No history rows matched the current filters.");
        return Ok(());
    }

    let width = output::terminal_width();
    for attempt in &attempts {
        println!("{}", render_history_cli_row(attempt, args.failed, width));
    }
    if truncated {
        println!(
            "Showing first {requested_limit} rows for {history_scope_label}; rerun with a higher --limit to inspect more."
        );
    } else if maybe_capped_by_hard_limit {
        println!(
            "Showing up to {PROJECT_LOG_QUERY_PAGE_SIZE} rows for {history_scope_label}; additional rows may exist. Narrow filters or use --project for a smaller scope."
        );
    }

    Ok(())
}
