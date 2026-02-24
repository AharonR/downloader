//! Topic auto-detection module for extracting keywords from paper metadata.
//!
//! This module provides keyword extraction from titles and abstracts using
//! the RAKE (Rapid Automatic Keyword Extraction) algorithm. Topics are
//! normalized and can be matched against custom topic lists.

mod extractor;
mod normalizer;

pub use extractor::{TopicExtractor, extract_keywords};
pub use normalizer::{match_custom_topics, normalize_topics};

use anyhow::{Context, Result};
use std::path::Path;
use tracing::instrument;

/// Loads custom topic list from a file (one topic per line).
///
/// Blank lines and lines starting with `#` are skipped.
///
/// # Errors
/// Returns error if the file cannot be read.
#[instrument]
pub fn load_custom_topics(path: &Path) -> Result<Vec<String>> {
    let content = std::fs::read_to_string(path)
        .with_context(|| format!("Cannot read topics file '{}'", path.display()))?;

    let topics = content
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
        .map(String::from)
        .collect();

    Ok(topics)
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn test_module_exports_are_accessible() {
        // Smoke test that public API is accessible
        let extractor = TopicExtractor::new();
        assert!(extractor.is_ok());
    }

    #[test]
    fn test_load_custom_topics_reads_file() {
        let mut file = NamedTempFile::new().unwrap();
        writeln!(file, "machine learning").unwrap();
        writeln!(file, "climate change").unwrap();
        writeln!(file, "quantum computing").unwrap();

        let topics = load_custom_topics(file.path()).unwrap();
        assert_eq!(topics.len(), 3);
        assert_eq!(topics[0], "machine learning");
        assert_eq!(topics[1], "climate change");
        assert_eq!(topics[2], "quantum computing");
    }

    #[test]
    fn test_load_custom_topics_skips_comments_and_blanks() {
        let mut file = NamedTempFile::new().unwrap();
        writeln!(file, "# This is a comment").unwrap();
        writeln!(file, "").unwrap();
        writeln!(file, "machine learning").unwrap();
        writeln!(file, "  ").unwrap();
        writeln!(file, "# Another comment").unwrap();
        writeln!(file, "climate change").unwrap();

        let topics = load_custom_topics(file.path()).unwrap();
        assert_eq!(topics.len(), 2);
        assert_eq!(topics[0], "machine learning");
        assert_eq!(topics[1], "climate change");
    }

    #[test]
    fn test_load_custom_topics_nonexistent_file_errors() {
        let result = load_custom_topics(Path::new("/nonexistent/topics.txt"));
        assert!(result.is_err());
    }

    #[test]
    fn test_load_custom_topics_empty_file_returns_empty() {
        let file = NamedTempFile::new().unwrap();
        let topics = load_custom_topics(file.path()).unwrap();
        assert!(topics.is_empty());
    }
}
