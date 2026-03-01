//! Project folder and history path utilities.
//!
//! Sanitisation and output-directory resolution are now provided by
//! [`downloader_core::project`]. This module re-exports those items and adds
//! the CLI-specific `discover_history_db_paths` helper.

use std::fs;
use std::path::{Path, PathBuf};

use anyhow::Result;
use tracing::warn;

// ---------------------------------------------------------------------------
// Re-exports from core (production code uses these)
// ---------------------------------------------------------------------------

pub use downloader_core::project::{
    append_project_download_log, append_project_index, generate_sidecars_for_completed,
    project_history_key, resolve_project_output_dir,
};

// ---------------------------------------------------------------------------
// Re-exports from core (test code only)
// ---------------------------------------------------------------------------

#[cfg(test)]
pub use downloader_core::project::{
    MAX_PROJECT_FOLDER_CHARS, MAX_PROJECT_SEGMENTS, escape_markdown_cell, is_windows_reserved_name,
    render_project_download_log_section, sanitize_project_name,
};

// ---------------------------------------------------------------------------
// CLI-specific: history database discovery
// ---------------------------------------------------------------------------

/// Discovers all `.downloader/queue.db` paths under `base_output_dir` (recursive).
pub fn discover_history_db_paths(base_output_dir: &Path) -> Result<Vec<PathBuf>> {
    if !base_output_dir.exists() {
        return Ok(Vec::new());
    }

    let mut db_paths = Vec::new();
    let mut stack = vec![base_output_dir.to_path_buf()];

    while let Some(dir) = stack.pop() {
        let entries = match fs::read_dir(&dir) {
            Ok(entries) => entries,
            Err(error) => {
                warn!(
                    path = %dir.display(),
                    error = %error,
                    "Skipping unreadable directory while discovering history databases"
                );
                continue;
            }
        };

        for entry in entries {
            let entry = match entry {
                Ok(entry) => entry,
                Err(error) => {
                    warn!(error = %error, "Skipping unreadable directory entry");
                    continue;
                }
            };
            let file_type = match entry.file_type() {
                Ok(file_type) => file_type,
                Err(error) => {
                    warn!(
                        path = %entry.path().display(),
                        error = %error,
                        "Skipping entry with unreadable file type"
                    );
                    continue;
                }
            };
            if !file_type.is_dir() {
                continue;
            }

            let path = entry.path();
            if entry.file_name() == ".downloader" {
                let db_path = path.join("queue.db");
                if db_path.exists() {
                    db_paths.push(db_path);
                }
                continue;
            }

            stack.push(path);
        }
    }

    db_paths.sort();
    db_paths.dedup();
    Ok(db_paths)
}
