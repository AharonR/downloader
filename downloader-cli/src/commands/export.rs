//! Export command handler: generate BibTeX or RIS bibliography from a corpus directory.

use std::io::{self, Write};
use std::path::PathBuf;

use anyhow::{Context, Result};
use downloader_core::{ExportFormat, generate_bibtex, generate_ris, scan_corpus};
use tracing::info;

use crate::cli::ExportArgs;

/// Runs `downloader export`: scans `args.corpus_dir` for sidecar files and writes a
/// bibliography to `args.output` (or stdout when output is `"-"` or unset with `--stdout`).
///
/// The default output file name is derived from the format:
/// - BibTeX → `bibliography.bib`
/// - RIS → `bibliography.ris`
pub async fn run_export_command(args: &ExportArgs) -> Result<()> {
    let format = ExportFormat::from(args.format);

    let entries = scan_corpus(&args.corpus_dir).with_context(|| {
        format!(
            "What: Failed to scan corpus directory\n\
             Why: {path} could not be read\n\
             Fix: verify the path exists and is readable",
            path = args.corpus_dir.display()
        )
    })?;

    if entries.is_empty() {
        println!(
            "No sidecar metadata found in {}.\n\
             Why: the directory contains no valid ScholarlyArticle JSON-LD files.\n\
             Fix: run downloader with --sidecar to generate metadata, then re-export.",
            args.corpus_dir.display()
        );
        return Ok(());
    }

    let content = match format {
        ExportFormat::BibTex => generate_bibtex(&entries),
        ExportFormat::Ris => generate_ris(&entries),
    };

    let output_path = resolve_output_path(args, format);

    match output_path {
        Some(path) => {
            std::fs::write(&path, &content).with_context(|| {
                format!(
                    "What: Failed to write bibliography file\n\
                     Why: could not write to {path}\n\
                     Fix: check that the parent directory exists and is writable",
                    path = path.display()
                )
            })?;
            info!(
                path = %path.display(),
                entries = entries.len(),
                format = ?format,
                "Bibliography written"
            );
            println!("Exported {} entries to {}", entries.len(), path.display());
        }
        None => {
            // Write to stdout.
            let stdout = io::stdout();
            let mut handle = stdout.lock();
            handle.write_all(content.as_bytes()).with_context(|| {
                "What: Failed to write bibliography to stdout\n\
                 Why: stdout write error\n\
                 Fix: check that stdout is not closed or redirected to a full disk"
                    .to_string()
            })?;
        }
    }

    Ok(())
}

/// Determines the output file path from CLI args.
///
/// Returns `None` to indicate stdout output (when `--output -` is passed).
/// Returns `Some(path)` for file output; uses a default name if `--output` is absent.
fn resolve_output_path(args: &ExportArgs, format: ExportFormat) -> Option<PathBuf> {
    match &args.output {
        Some(path) if path.as_os_str() == "-" => None,
        Some(path) => Some(path.clone()),
        None => {
            let default_name = match format {
                ExportFormat::BibTex => "bibliography.bib",
                ExportFormat::Ris => "bibliography.ris",
            };
            Some(PathBuf::from(default_name))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cli::ExportFormatArg;

    fn make_args(corpus_dir: &str, format: ExportFormatArg, output: Option<&str>) -> ExportArgs {
        ExportArgs {
            corpus_dir: PathBuf::from(corpus_dir),
            format,
            output: output.map(PathBuf::from),
        }
    }

    #[test]
    fn test_resolve_output_path_default_bibtex() {
        let args = make_args("/corpus", ExportFormatArg::Bibtex, None);
        let path = resolve_output_path(&args, ExportFormat::BibTex).unwrap();
        assert_eq!(path, PathBuf::from("bibliography.bib"));
    }

    #[test]
    fn test_resolve_output_path_default_ris() {
        let args = make_args("/corpus", ExportFormatArg::Ris, None);
        let path = resolve_output_path(&args, ExportFormat::Ris).unwrap();
        assert_eq!(path, PathBuf::from("bibliography.ris"));
    }

    #[test]
    fn test_resolve_output_path_explicit_file() {
        let args = make_args("/corpus", ExportFormatArg::Bibtex, Some("refs.bib"));
        let path = resolve_output_path(&args, ExportFormat::BibTex).unwrap();
        assert_eq!(path, PathBuf::from("refs.bib"));
    }

    #[test]
    fn test_resolve_output_path_dash_returns_none() {
        let args = make_args("/corpus", ExportFormatArg::Bibtex, Some("-"));
        let path = resolve_output_path(&args, ExportFormat::BibTex);
        assert!(path.is_none(), "expected None for stdout output");
    }

    #[tokio::test]
    async fn test_run_export_command_missing_corpus_returns_error() {
        let args = make_args(
            "/nonexistent/path/that/should/not/exist",
            ExportFormatArg::Bibtex,
            None,
        );
        let result = run_export_command(&args).await;
        assert!(result.is_err(), "expected error for missing corpus dir");
    }

    #[tokio::test]
    async fn test_run_export_command_empty_corpus_prints_message() {
        let tmp = tempfile::TempDir::new().unwrap();
        let args = ExportArgs {
            corpus_dir: tmp.path().to_path_buf(),
            format: ExportFormatArg::Bibtex,
            output: Some(tmp.path().join("out.bib")),
        };
        // Should succeed (no entries = prints message, no output file).
        let result = run_export_command(&args).await;
        assert!(result.is_ok(), "empty corpus should not error: {result:?}");
    }

    #[tokio::test]
    async fn test_run_export_command_writes_bibtex_file() {
        let tmp = tempfile::TempDir::new().unwrap();

        // Write a valid sidecar
        let json = r#"{"@context":"https://schema.org","@type":"ScholarlyArticle","name":"Test Paper","author":[{"@type":"Person","name":"Alice Smith"}],"datePublished":"2024","url":"https://example.com/paper.pdf"}"#;
        std::fs::write(tmp.path().join("paper.json"), json).unwrap();

        let out_path = tmp.path().join("out.bib");
        let args = ExportArgs {
            corpus_dir: tmp.path().to_path_buf(),
            format: ExportFormatArg::Bibtex,
            output: Some(out_path.clone()),
        };

        run_export_command(&args).await.unwrap();
        assert!(out_path.exists(), "output file should have been created");
        let content = std::fs::read_to_string(&out_path).unwrap();
        assert!(
            content.contains("@article{"),
            "should contain BibTeX entry: {content}"
        );
        assert!(content.contains("Test Paper"), "{content}");
    }

    #[tokio::test]
    async fn test_run_export_command_writes_ris_file() {
        let tmp = tempfile::TempDir::new().unwrap();

        let json = r#"{"@context":"https://schema.org","@type":"ScholarlyArticle","name":"RIS Paper","datePublished":"2023","url":"https://example.com/ris.pdf"}"#;
        std::fs::write(tmp.path().join("ris.json"), json).unwrap();

        let out_path = tmp.path().join("out.ris");
        let args = ExportArgs {
            corpus_dir: tmp.path().to_path_buf(),
            format: ExportFormatArg::Ris,
            output: Some(out_path.clone()),
        };

        run_export_command(&args).await.unwrap();
        assert!(out_path.exists(), "output file should have been created");
        let content = std::fs::read_to_string(&out_path).unwrap();
        assert!(
            content.contains("TY  - JOUR"),
            "should contain RIS entry: {content}"
        );
        assert!(content.contains("RIS Paper"), "{content}");
    }
}
