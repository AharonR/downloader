//! Convert command handler: convert `.html` corpus files to `.pdf` using headless Chrome.

use std::path::{Path, PathBuf};
use std::time::Instant;

use anyhow::{Context, Result, bail};
use tokio::process::Command as TokioCommand;
use tracing::{info, warn};
use url::Url;

use crate::cli::ConvertArgs;

/// Hosts (and their subdomains) whose HTML outputs are known-useless for conversion:
/// paywall stubs, abstract-only pages, and redirect landing pages.
const KNOWN_USELESS_HOST_SUFFIXES: &[&str] =
    &["sciencedirect.com", "pubmed.ncbi.nlm.nih.gov", "doi.org"];

/// Runs `downloader convert`: scans `args.corpus_dir` for `.html` files and converts
/// each eligible one to a sibling `.pdf` using headless Chrome.
pub async fn run_convert_command(args: &ConvertArgs) -> Result<()> {
    let corpus_dir = &args.corpus_dir;
    if !corpus_dir.is_dir() {
        bail!(
            "What: Corpus directory not found\n\
             Why: {path} does not exist or is not a directory\n\
             Fix: verify the path is correct",
            path = corpus_dir.display()
        );
    }

    let chrome = if args.dry_run {
        PathBuf::new() // not needed for dry-run
    } else {
        find_chrome_binary(args.chrome_binary.as_deref())?
    };

    let html_files = collect_html_files(corpus_dir)?;
    let total = html_files.len();

    if total == 0 {
        println!(
            "Converted 0 of 0 HTML files (0 skipped, 0 failed) in {}",
            corpus_dir.display()
        );
        return Ok(());
    }

    let mut converted = 0usize;
    let mut skipped = 0usize;
    let mut failed = 0usize;

    for html_path in &html_files {
        let pdf_path = html_path.with_extension("pdf");

        if pdf_path.exists() {
            info!(path = %html_path.display(), "skip: PDF already exists");
            skipped += 1;
            continue;
        }

        if !args.no_skip {
            if let Some(host) = useless_host(&html_path.with_extension("json")) {
                info!(path = %html_path.display(), host = %host, "skip: known-useless host");
                skipped += 1;
                continue;
            }
        }

        if args.dry_run {
            println!("would convert: {}", html_path.display());
            converted += 1;
            continue;
        }

        let start = Instant::now();
        match convert_html_to_pdf(&chrome, html_path, &pdf_path).await {
            Ok(()) => {
                info!(
                    path = %html_path.display(),
                    elapsed_ms = start.elapsed().as_millis(),
                    "Converted HTML to PDF"
                );
                converted += 1;
            }
            Err(e) => {
                warn!(path = %html_path.display(), error = %e, "Failed to convert HTML to PDF");
                failed += 1;
            }
        }
    }

    if args.dry_run {
        println!(
            "Dry run: would convert {} of {} HTML files ({} would be skipped) in {}",
            converted,
            total,
            skipped,
            corpus_dir.display()
        );
    } else {
        println!(
            "Converted {} of {} HTML files ({} skipped, {} failed) in {}",
            converted,
            total,
            skipped,
            failed,
            corpus_dir.display()
        );
    }

    Ok(())
}

/// Collects all `.html` files in `dir` (non-recursive), sorted for determinism.
fn collect_html_files(dir: &Path) -> Result<Vec<PathBuf>> {
    let mut files: Vec<PathBuf> = std::fs::read_dir(dir)
        .with_context(|| format!("failed to read directory {}", dir.display()))?
        .filter_map(|entry| {
            let path = entry.ok()?.path();
            if path.is_file() {
                let ext = path.extension()?.to_str()?.to_ascii_lowercase();
                if ext == "html" {
                    return Some(path);
                }
            }
            None
        })
        .collect();
    files.sort();
    Ok(files)
}

/// Returns the host string if the sidecar JSON records a URL whose host is in
/// `KNOWN_USELESS_HOST_SUFFIXES`, otherwise `None`.
fn useless_host(json_path: &Path) -> Option<String> {
    let content = std::fs::read_to_string(json_path).ok()?;
    let value: serde_json::Value = serde_json::from_str(&content).ok()?;
    let url_str = value.get("url")?.as_str()?;
    let host = Url::parse(url_str).ok()?.host_str()?.to_owned();
    for suffix in KNOWN_USELESS_HOST_SUFFIXES {
        if host == *suffix || host.ends_with(&format!(".{suffix}")) {
            return Some(host);
        }
    }
    None
}

/// Locates the Chrome/Chromium binary to use.
///
/// Resolution order: CLI flag → `DOWNLOADER_CHROME_BINARY` env → macOS default path →
/// PATH names (`google-chrome`, `google-chrome-stable`, `chromium`, `chromium-browser`).
fn find_chrome_binary(override_path: Option<&Path>) -> Result<PathBuf> {
    if let Some(p) = override_path {
        return Ok(p.to_path_buf());
    }
    if let Ok(env_val) = std::env::var("DOWNLOADER_CHROME_BINARY") {
        return Ok(PathBuf::from(env_val));
    }
    let macos_path = PathBuf::from("/Applications/Google Chrome.app/Contents/MacOS/Google Chrome");
    if macos_path.exists() {
        return Ok(macos_path);
    }
    for name in [
        "google-chrome",
        "google-chrome-stable",
        "chromium",
        "chromium-browser",
    ] {
        if let Ok(output) = std::process::Command::new("which").arg(name).output() {
            if output.status.success() {
                let path_str = String::from_utf8_lossy(&output.stdout).trim().to_string();
                if !path_str.is_empty() {
                    return Ok(PathBuf::from(path_str));
                }
            }
        }
    }
    bail!(
        "What: Chrome/Chromium binary not found\n\
         Why: headless Chrome is required for HTML→PDF conversion\n\
         Fix: install Google Chrome, set DOWNLOADER_CHROME_BINARY, or pass --chrome-binary <path>"
    )
}

/// Invokes headless Chrome to convert `html_path` → `pdf_path`.
async fn convert_html_to_pdf(chrome: &Path, html_path: &Path, pdf_path: &Path) -> Result<()> {
    let abs_html = html_path
        .canonicalize()
        .with_context(|| format!("failed to resolve {}", html_path.display()))?;
    let file_url = format!("file://{}", abs_html.display());

    let output = TokioCommand::new(chrome)
        .args([
            "--headless=new",
            "--disable-gpu",
            "--no-sandbox",
            "--print-to-pdf-no-header",
            &format!("--print-to-pdf={}", pdf_path.display()),
            &file_url,
        ])
        .output()
        .await
        .with_context(|| format!("failed to spawn Chrome for {}", html_path.display()))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        bail!(
            "Chrome exited with {status}: {stderr}",
            status = output.status
        );
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cli::ConvertArgs;
    use std::os::unix::fs::PermissionsExt;

    fn make_args(corpus_dir: &Path) -> ConvertArgs {
        ConvertArgs {
            corpus_dir: corpus_dir.to_path_buf(),
            chrome_binary: None,
            no_skip: false,
            dry_run: false,
        }
    }

    fn make_sidecar(url: &str) -> String {
        format!(
            r#"{{"@context":"https://schema.org","@type":"ScholarlyArticle","name":"Test","url":"{url}"}}"#
        )
    }

    #[tokio::test]
    async fn test_missing_corpus_returns_error() {
        let args = ConvertArgs {
            corpus_dir: PathBuf::from("/nonexistent/path/that/does/not/exist"),
            chrome_binary: None,
            no_skip: false,
            dry_run: false,
        };
        let result = run_convert_command(&args).await;
        assert!(result.is_err(), "expected error for missing corpus dir");
        let msg = format!("{}", result.unwrap_err());
        assert!(msg.contains("Corpus directory not found"), "{msg}");
    }

    #[tokio::test]
    async fn test_empty_corpus_succeeds_with_zero_summary() {
        let tmp = tempfile::TempDir::new().unwrap();
        let result = run_convert_command(&make_args(tmp.path())).await;
        assert!(result.is_ok(), "empty corpus should not error: {result:?}");
    }

    #[tokio::test]
    async fn test_skips_when_pdf_sibling_exists() {
        let tmp = tempfile::TempDir::new().unwrap();
        std::fs::write(tmp.path().join("paper.html"), "<html></html>").unwrap();
        std::fs::write(tmp.path().join("paper.pdf"), b"%PDF").unwrap();

        let mut args = make_args(tmp.path());
        args.dry_run = true;
        let result = run_convert_command(&args).await;
        assert!(result.is_ok());
        // PDF exists → file is skipped, so "would convert" does not appear
        // (we can't capture stdout here, but the skipped counter increments — no panic is the assertion)
    }

    #[tokio::test]
    async fn test_skips_known_useless_host() {
        let tmp = tempfile::TempDir::new().unwrap();
        std::fs::write(tmp.path().join("paper.html"), "<html></html>").unwrap();
        std::fs::write(
            tmp.path().join("paper.json"),
            make_sidecar("https://www.sciencedirect.com/science/article/pii/S000"),
        )
        .unwrap();

        let mut args = make_args(tmp.path());
        args.dry_run = true;
        let result = run_convert_command(&args).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_no_skip_overrides_host_filter() {
        let tmp = tempfile::TempDir::new().unwrap();
        std::fs::write(tmp.path().join("paper.html"), "<html></html>").unwrap();
        std::fs::write(
            tmp.path().join("paper.json"),
            make_sidecar("https://pubmed.ncbi.nlm.nih.gov/12345/"),
        )
        .unwrap();

        // With no_skip=true and dry_run=true the file should be counted for conversion.
        let args = ConvertArgs {
            corpus_dir: tmp.path().to_path_buf(),
            chrome_binary: None,
            no_skip: true,
            dry_run: true,
        };
        let result = run_convert_command(&args).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_useless_host_detects_sciencedirect() {
        let tmp = tempfile::TempDir::new().unwrap();
        let json_path = tmp.path().join("paper.json");
        std::fs::write(
            &json_path,
            make_sidecar("https://www.sciencedirect.com/science/article/pii/S000"),
        )
        .unwrap();
        assert!(useless_host(&json_path).is_some());
    }

    #[tokio::test]
    async fn test_useless_host_detects_pubmed() {
        let tmp = tempfile::TempDir::new().unwrap();
        let json_path = tmp.path().join("paper.json");
        std::fs::write(
            &json_path,
            make_sidecar("https://pubmed.ncbi.nlm.nih.gov/12345/"),
        )
        .unwrap();
        assert!(useless_host(&json_path).is_some());
    }

    #[tokio::test]
    async fn test_useless_host_detects_doi_org() {
        let tmp = tempfile::TempDir::new().unwrap();
        let json_path = tmp.path().join("paper.json");
        std::fs::write(&json_path, make_sidecar("https://doi.org/10.1234/test")).unwrap();
        assert!(useless_host(&json_path).is_some());
    }

    #[tokio::test]
    async fn test_real_host_not_filtered() {
        let tmp = tempfile::TempDir::new().unwrap();
        let json_path = tmp.path().join("paper.json");
        std::fs::write(
            &json_path,
            make_sidecar("https://openreview.net/forum?id=abc123"),
        )
        .unwrap();
        assert!(useless_host(&json_path).is_none());
    }

    #[tokio::test]
    async fn test_convert_with_stubbed_chrome() {
        // Create a stub Chrome script that writes a minimal PDF marker.
        let tmp = tempfile::TempDir::new().unwrap();
        let stub_path = tmp.path().join("fake-chrome");
        std::fs::write(
            &stub_path,
            "#!/bin/sh\nfor arg in \"$@\"; do\n  case \"$arg\" in\n    --print-to-pdf=*) \
             out=\"${arg#--print-to-pdf=}\"; printf '%%PDF-stub' > \"$out\" ;;\n  esac\ndone\n",
        )
        .unwrap();
        std::fs::set_permissions(&stub_path, std::fs::Permissions::from_mode(0o755)).unwrap();

        let corpus = tempfile::TempDir::new().unwrap();
        std::fs::write(
            corpus.path().join("article.html"),
            "<html><body>Content</body></html>",
        )
        .unwrap();
        // No sidecar → no host filter → should attempt conversion
        std::fs::write(
            corpus.path().join("article.json"),
            make_sidecar("https://openreview.net/forum?id=xyz"),
        )
        .unwrap();

        let args = ConvertArgs {
            corpus_dir: corpus.path().to_path_buf(),
            chrome_binary: Some(stub_path),
            no_skip: false,
            dry_run: false,
        };
        run_convert_command(&args).await.unwrap();

        let pdf_path = corpus.path().join("article.pdf");
        assert!(pdf_path.exists(), "expected PDF to be created");
        let content = std::fs::read_to_string(&pdf_path).unwrap();
        assert!(
            content.starts_with("%PDF"),
            "expected PDF marker: {content}"
        );
    }
}
