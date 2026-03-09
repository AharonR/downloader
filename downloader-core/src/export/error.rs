//! Error types for the export module.

use thiserror::Error;

/// Errors that can occur during corpus export.
#[derive(Debug, Error)]
pub enum ExportError {
    /// An I/O error occurred while scanning the corpus directory or writing the output file.
    #[error("I/O error during export: {0}")]
    Io(#[from] std::io::Error),

    /// A sidecar JSON file could not be parsed. The file is skipped automatically during scanning,
    /// but this variant is used when an explicit parse failure must be surfaced.
    #[error("Failed to parse sidecar file at {path}: {source}")]
    SidecarParse {
        /// Path to the sidecar file that could not be parsed.
        path: String,
        /// Underlying JSON parse error.
        #[source]
        source: serde_json::Error,
    },

    /// The supplied corpus directory path does not exist or is not a directory.
    #[error(
        "Corpus directory not found: {path}\n\
         Why: the path does not exist or is not a directory.\n\
         Fix: verify the path with `ls` and pass the directory that contains the downloaded files."
    )]
    CorpusNotFound {
        /// The directory path that was not found.
        path: String,
    },
}
