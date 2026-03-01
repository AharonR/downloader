//! CLI output formatting and display helpers.

use std::path::Path;

use anyhow::Result;
use downloader_core::{
    DownloadAttempt, DownloadAttemptStatus, DownloadStats, Queue, QueueStatus,
    extract_reference_confidence,
};
use tracing::info;

use crate::cli::HistoryStatusArg;
use crate::failure;

/// Message when no input was provided at all.
pub const NO_INPUT_GUIDANCE: &str = "No input provided. Pipe URLs via stdin or pass as arguments.";

/// Message when stdin was piped but empty.
pub const EMPTY_STDIN_GUIDANCE: &str =
    "Received empty stdin input. Pipe URLs, DOIs, or references, or pass them as arguments.";

/// Example for piping input.
pub const INPUT_PIPE_EXAMPLE: &str = "Example: echo 'https://example.com/file.pdf' | downloader";

/// Example for passing URLs as arguments.
pub const INPUT_ARG_EXAMPLE: &str = "Example: downloader https://example.com/file.pdf";

/// Returns terminal width from COLUMNS, or 80 if unset/invalid.
pub fn terminal_width() -> usize {
    std::env::var("COLUMNS")
        .ok()
        .and_then(|value| value.parse::<usize>().ok())
        .filter(|width| *width >= 20)
        .unwrap_or(80)
}

/// Truncates text to at most `width` chars, appending ellipsis if truncated.
pub fn truncate_to_width(text: &str, width: usize) -> String {
    let text_len = text.chars().count();
    if text_len <= width {
        return text.to_string();
    }
    if width == 0 {
        return String::new();
    }
    if width == 1 {
        return "…".to_string();
    }

    let mut output: String = text.chars().take(width - 1).collect();
    output.push('…');
    output
}

/// Returns lines for quick-start guidance (headline + examples), truncated to width.
pub fn quick_start_guidance_lines(empty_stdin: bool, width: usize) -> Vec<String> {
    let headline = if empty_stdin {
        EMPTY_STDIN_GUIDANCE
    } else {
        NO_INPUT_GUIDANCE
    };

    vec![
        truncate_to_width(headline, width),
        truncate_to_width(INPUT_PIPE_EXAMPLE, width),
        truncate_to_width(INPUT_ARG_EXAMPLE, width),
    ]
}

/// Prints quick-start guidance to stdout (no input or empty stdin).
pub fn print_quick_start_guidance(empty_stdin: bool) {
    let width = terminal_width().min(80);
    for line in quick_start_guidance_lines(empty_stdin, width) {
        println!("{line}");
    }
}

pub(crate) fn log_parse_feedback(parse_result: &downloader_core::ParseResult) {
    let summary = build_parse_feedback_summary(parse_result);
    let width = terminal_width();
    info!("{}", truncate_to_width(&summary, width));
}

pub(crate) fn build_parse_feedback_summary(parse_result: &downloader_core::ParseResult) -> String {
    let counts = parse_result.type_counts();
    let confidence_distribution = reference_confidence_distribution(parse_result);
    let mut summary = format!(
        "Parsed {} items: {} URLs, {} DOIs, {} references, {} BibTeX",
        parse_result.len(),
        counts.urls,
        counts.dois,
        counts.references,
        counts.bibtex
    );

    if counts.references > 0 {
        summary.push_str(&format!(
            " [reference confidence: high={}, medium={}, low={}]",
            confidence_distribution.high,
            confidence_distribution.medium,
            confidence_distribution.low
        ));
    }

    if confidence_distribution.low > 0 {
        summary.push_str(&format!(
            " ({} references need verification)",
            confidence_distribution.low
        ));
    }

    summary
}

#[derive(Debug, Clone, Copy, Default)]
struct ReferenceConfidenceDistribution {
    high: usize,
    medium: usize,
    low: usize,
}

fn reference_confidence_distribution(
    parse_result: &downloader_core::ParseResult,
) -> ReferenceConfidenceDistribution {
    let mut distribution = ReferenceConfidenceDistribution::default();
    for item in parse_result.references() {
        match extract_reference_confidence(&item.raw).level {
            downloader_core::Confidence::High => distribution.high += 1,
            downloader_core::Confidence::Medium => distribution.medium += 1,
            downloader_core::Confidence::Low => distribution.low += 1,
        }
    }
    distribution
}

pub(crate) fn map_history_status(status: HistoryStatusArg) -> DownloadAttemptStatus {
    match status {
        HistoryStatusArg::Success => DownloadAttemptStatus::Success,
        HistoryStatusArg::Failed => DownloadAttemptStatus::Failed,
        HistoryStatusArg::Skipped => DownloadAttemptStatus::Skipped,
    }
}

pub(crate) fn render_history_cli_row(
    attempt: &DownloadAttempt,
    failed_only: bool,
    width: usize,
) -> String {
    let date = &attempt.started_at;
    let status = attempt.status().as_str().to_ascii_uppercase();
    let title_or_file = download_log_filename(attempt);
    let source = download_log_source(attempt);
    let confidence_suffix = attempt
        .parse_confidence
        .as_deref()
        .map(|value| format!(" | confidence={value}"))
        .unwrap_or_default();
    let base_line = format!("{date} | {status} | {title_or_file}{confidence_suffix} | {source}");

    if failed_only && attempt.status() == DownloadAttemptStatus::Failed {
        let descriptor = failure::history_failure_descriptor(attempt);
        let suggestion = failure::history_failure_suggestion(attempt);
        let what_line = format!("  {} What: {}", descriptor.category.icon(), descriptor.what);
        let why_line = format!("  Why: {}", descriptor.why);
        let suggestion_line = format!("  Fix: {suggestion}");
        return format!(
            "{}\n{}\n{}\n{}",
            truncate_to_width(&base_line, width),
            truncate_to_width(&what_line, width),
            truncate_to_width(&why_line, width),
            truncate_to_width(&suggestion_line, width)
        );
    }

    truncate_to_width(&base_line, width)
}

pub(crate) async fn print_completion_summary(
    queue: &Queue,
    output_dir: &Path,
    stats: &DownloadStats,
    total_queued: usize,
    project_output_dir: Option<&Path>,
    uncertain_references_in_run: usize,
) -> Result<()> {
    let failed_items = queue.list_by_status(QueueStatus::Failed).await?;
    let succeeded = stats.completed();

    info!(
        succeeded,
        total_queued,
        output_dir = %output_dir.display(),
        "Download Summary"
    );
    if let Some(project_dir) = project_output_dir {
        info!(project_dir = %project_dir.display(), "Project folder");
    }
    if let Some(summary_line) = uncertain_reference_summary_line(uncertain_references_in_run) {
        info!(
            uncertain_references = uncertain_references_in_run,
            "{summary_line}"
        );
    }

    if !failed_items.is_empty() {
        let reasons: Vec<&str> = failed_items
            .iter()
            .map(|item| item.last_error.as_deref().unwrap_or("unknown"))
            .collect();
        for line in render_failure_summary_lines(&reasons, terminal_width()) {
            println!("{line}");
        }
    }

    Ok(())
}

pub(crate) fn uncertain_reference_summary_line(
    uncertain_references_in_run: usize,
) -> Option<String> {
    if uncertain_references_in_run == 0 {
        return None;
    }
    Some(format!(
        "{uncertain_references_in_run} low-confidence references need manual verification"
    ))
}

pub(crate) fn render_failure_summary_lines(failed_reasons: &[&str], width: usize) -> Vec<String> {
    use std::collections::BTreeMap;

    if failed_reasons.is_empty() {
        return Vec::new();
    }

    let mut grouped: BTreeMap<failure::FailureCategory, usize> = BTreeMap::new();
    let mut auth_domains: BTreeMap<String, usize> = BTreeMap::new();

    for reason in failed_reasons {
        let descriptor = failure::classify_failure(reason);
        grouped
            .entry(descriptor.category)
            .and_modify(|count| *count += 1)
            .or_insert(1);

        if descriptor.category == failure::FailureCategory::Auth
            && let Some(domain) = failure::extract_auth_domain(reason)
        {
            *auth_domains.entry(domain).or_insert(0) += 1;
        }
    }

    let mut lines = vec![truncate_to_width("Failure summary by category:", width)];
    for (category, count) in &grouped {
        let descriptor = failure::category_failure_descriptor(*category);
        let header = format!("- {} {}: {}", category.icon(), category.label(), count);
        lines.push(truncate_to_width(&header, width));
        lines.push(truncate_to_width(
            &format!("  What: {}", descriptor.what),
            width,
        ));

        let why_detail = if *category == failure::FailureCategory::Auth && !auth_domains.is_empty()
        {
            let domains: Vec<String> = auth_domains
                .iter()
                .map(|(domain, count)| format!("{domain} ({count})"))
                .collect();
            format!(
                "{} Affected domains: {}",
                descriptor.why,
                domains.join(", ")
            )
        } else {
            descriptor.why.to_string()
        };
        lines.push(truncate_to_width(&format!("  Why: {why_detail}"), width));
        lines.push(truncate_to_width(
            &format!("  Fix: {}", descriptor.fix),
            width,
        ));
    }

    lines
}

pub(crate) fn download_log_filename(attempt: &DownloadAttempt) -> String {
    attempt
        .file_path
        .as_deref()
        .and_then(|path| {
            std::path::Path::new(path)
                .file_name()
                .and_then(|name| name.to_str())
        })
        .map(std::string::ToString::to_string)
        .or_else(|| attempt.title.clone())
        .unwrap_or_else(|| "n/a".to_string())
}

pub(crate) fn download_log_source(attempt: &DownloadAttempt) -> &str {
    attempt
        .original_input
        .as_deref()
        .unwrap_or(attempt.url.as_str())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_terminal_width_returns_sensible_value() {
        let w = terminal_width();
        assert!(w >= 20, "terminal_width should be at least 20, got {}", w);
        assert!(
            w <= 2000,
            "terminal_width should be at most 2000, got {}",
            w
        );
    }
}
