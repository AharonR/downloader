//! Search command handler: query history and open top result.

use std::path::PathBuf;

use anyhow::Result;
use downloader_core::{Database, DownloadSearchQuery, Queue};

use crate::cli::SearchArgs;
use crate::open_path_in_default_app;
use crate::output;
use crate::project;
use crate::render_search_cli_row;
use crate::resolve_search_candidate_file_path;
use crate::search;
use crate::validate_search_date_range;

const SEARCH_CANDIDATE_LIMIT_PER_DB: usize = 10_000;

pub async fn run_search_command(args: &SearchArgs) -> Result<()> {
    validate_search_date_range(args.since.as_deref(), args.until.as_deref())?;

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

    let mut query = DownloadSearchQuery {
        since: args.since.clone(),
        until: args.until.clone(),
        openable_only: true,
        limit: SEARCH_CANDIDATE_LIMIT_PER_DB,
        ..DownloadSearchQuery::default()
    };
    if let Some(project_key) = project_scope_key {
        query.project = Some(project_key);
    }

    let mut candidates = Vec::new();
    let mut maybe_capped_by_hard_limit = false;
    for db_path in &db_paths {
        let db = Database::new(db_path).await?;
        let queue = Queue::new(db);
        let mut db_candidates = queue.query_download_search_candidates(&query).await?;
        for candidate in &mut db_candidates {
            resolve_search_candidate_file_path(candidate, db_path);
        }
        if db_candidates.len() == SEARCH_CANDIDATE_LIMIT_PER_DB {
            maybe_capped_by_hard_limit = true;
        }
        candidates.extend(db_candidates);
    }

    if candidates.is_empty() {
        println!("No search candidates found for {history_scope_label}.");
        return Ok(());
    }

    let mut ranked = search::rank_search_candidates(&args.query, candidates);
    if ranked.is_empty() {
        println!("No search results matched the current query and filters.");
        return Ok(());
    }

    let requested_limit = usize::from(args.limit);
    let truncated = ranked.len() > requested_limit;
    if truncated {
        ranked.truncate(requested_limit);
    }

    let width = output::terminal_width();
    for result in &ranked {
        println!("{}", render_search_cli_row(result, width));
    }

    if truncated {
        println!(
            "Showing first {requested_limit} search results for {history_scope_label}; rerun with a higher --limit to inspect more."
        );
    } else if maybe_capped_by_hard_limit {
        println!(
            "Search candidates were capped at {SEARCH_CANDIDATE_LIMIT_PER_DB} rows per history database; older matches may exist."
        );
    }

    if args.open
        && let Some(top) = ranked.first()
    {
        let top_path = top
            .candidate
            .file_path
            .as_deref()
            .map(PathBuf::from)
            .unwrap_or_default();
        if top_path.as_os_str().is_empty() {
            println!(
                "What: Cannot open top search result\nWhy: Result has no file path metadata\nFix: Re-run without --open or redownload the item."
            );
        } else {
            match open_path_in_default_app(&top_path) {
                Ok(()) => println!("Opened top result: {}", top_path.display()),
                Err(error) => println!("{error}"),
            }
        }
    }

    Ok(())
}
