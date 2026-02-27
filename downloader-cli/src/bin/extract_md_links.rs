use std::collections::HashSet;
use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use clap::Parser;
use regex::Regex;

/// Extract HTTP/HTTPS links from Markdown files into urls.txt format.
#[derive(Parser, Debug)]
#[command(name = "extract-md-links")]
#[command(
    author,
    version,
    about = "Extract HTTP/HTTPS links from Markdown into urls.txt format"
)]
struct Args {
    /// Files or directories to scan for Markdown files
    #[arg(required = true)]
    inputs: Vec<PathBuf>,

    /// Output file path (defaults to stdout)
    #[arg(short, long)]
    output: Option<PathBuf>,

    /// Include duplicate URLs in output
    #[arg(long)]
    keep_duplicates: bool,
}

fn main() -> Result<()> {
    let args = Args::parse();

    let markdown_files = collect_markdown_files(&args.inputs)?;
    let urls = extract_urls_from_files(&markdown_files, args.keep_duplicates)?;

    write_output(&urls, args.output.as_deref())?;

    eprintln!(
        "Scanned {} markdown file(s), extracted {} URL(s)",
        markdown_files.len(),
        urls.len()
    );

    Ok(())
}

fn collect_markdown_files(inputs: &[PathBuf]) -> Result<Vec<PathBuf>> {
    let mut files = Vec::new();

    for input in inputs {
        if input.is_file() {
            if is_markdown_file(input) {
                files.push(input.clone());
            }
            continue;
        }

        if input.is_dir() {
            collect_markdown_files_recursive(input, &mut files)?;
            continue;
        }

        anyhow::bail!("Input path does not exist: {}", input.display());
    }

    files.sort();
    files.dedup();
    Ok(files)
}

fn collect_markdown_files_recursive(dir: &Path, files: &mut Vec<PathBuf>) -> Result<()> {
    let entries = fs::read_dir(dir)
        .with_context(|| format!("Failed to read directory: {}", dir.display()))?;

    for entry in entries {
        let entry = entry.with_context(|| format!("Failed to read entry in {}", dir.display()))?;
        let path = entry.path();

        if path.is_dir() {
            collect_markdown_files_recursive(&path, files)?;
        } else if path.is_file() && is_markdown_file(&path) {
            files.push(path);
        }
    }

    Ok(())
}

fn is_markdown_file(path: &Path) -> bool {
    let Some(ext) = path.extension().and_then(|e| e.to_str()) else {
        return false;
    };

    matches!(ext.to_ascii_lowercase().as_str(), "md" | "markdown" | "mdx")
}

fn extract_urls_from_files(files: &[PathBuf], keep_duplicates: bool) -> Result<Vec<String>> {
    let url_regex = Regex::new(r#"https?://[^\s<>\"']+"#).context("Failed to compile URL regex")?;

    let mut urls = Vec::new();
    let mut seen = HashSet::new();

    for file in files {
        let content = fs::read_to_string(file)
            .with_context(|| format!("Failed to read file: {}", file.display()))?;

        for mat in url_regex.find_iter(&content) {
            let url = trim_trailing_punctuation(mat.as_str());
            if url.is_empty() {
                continue;
            }

            if keep_duplicates || seen.insert(url.to_string()) {
                urls.push(url.to_string());
            }
        }
    }

    Ok(urls)
}

fn trim_trailing_punctuation(url: &str) -> &str {
    url.trim_end_matches(['.', ',', ';', ':', '!', '?', ')', ']', '}'])
}

fn write_output(urls: &[String], output: Option<&Path>) -> Result<()> {
    match output {
        Some(path) => {
            let mut content = urls.join("\n");
            if !content.is_empty() {
                content.push('\n');
            }
            fs::write(path, content)
                .with_context(|| format!("Failed to write output file: {}", path.display()))?;
        }
        None => {
            let mut stdout = io::stdout().lock();
            for url in urls {
                writeln!(stdout, "{url}")?;
            }
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::trim_trailing_punctuation;

    #[test]
    fn trims_common_markdown_trailing_punctuation() {
        assert_eq!(
            trim_trailing_punctuation("https://a.com/path),"),
            "https://a.com/path"
        );
        assert_eq!(
            trim_trailing_punctuation("https://a.com/path."),
            "https://a.com/path"
        );
    }

    #[test]
    fn leaves_normal_url_unchanged() {
        assert_eq!(
            trim_trailing_punctuation("https://a.com/path?x=1"),
            "https://a.com/path?x=1"
        );
    }
}
