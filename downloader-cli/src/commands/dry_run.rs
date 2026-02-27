//! Dry-run command flow for previewing parse+resolution behavior.

use std::sync::Arc;

use anyhow::Result;
use downloader_core::{ResolveContext, build_default_resolver_registry, parse_input};
use reqwest::cookie::Jar;
use tracing::{info, warn};

pub async fn run_dry_run_preview(input_text: &str, cookie_jar: Option<Arc<Jar>>) -> Result<()> {
    let parse_result = parse_input(input_text);
    let counts = parse_result.type_counts();
    info!(
        parsed_total = parse_result.len(),
        urls = counts.urls,
        dois = counts.dois,
        references = counts.references,
        bibtex = counts.bibtex,
        skipped = parse_result.skipped_count(),
        "Parsed input (dry run)"
    );
    for skipped in &parse_result.skipped {
        warn!(skipped = %skipped, "Skipped unrecognized input");
    }

    if parse_result.is_empty() {
        info!("No valid URLs found in input");
        println!("Dry run - no files downloaded");
        return Ok(());
    }

    crate::log_parse_feedback(&parse_result);

    let resolver_registry = build_default_resolver_registry(cookie_jar, "downloader@example.com");
    let resolve_context = ResolveContext::default();

    println!(
        "Dry run preview: {} parsed item(s), {} skipped.",
        parse_result.len(),
        parse_result.skipped_count()
    );

    let mut resolved_count = 0usize;
    let mut unresolved_count = 0usize;
    for item in &parse_result.items {
        let resolver_input = if item.input_type == downloader_core::InputType::BibTex {
            item.raw.as_str()
        } else {
            item.value.as_str()
        };

        match resolver_registry
            .resolve_to_url(resolver_input, item.input_type, &resolve_context)
            .await
        {
            Ok(resolved) => {
                resolved_count += 1;
                println!(
                    "- [resolved][{}] {} -> {}",
                    item.input_type, item.value, resolved.url
                );
            }
            Err(error) => {
                unresolved_count += 1;
                println!(
                    "- [unresolved][{}] {} -> {}",
                    item.input_type,
                    item.value,
                    preview_single_line(&error.to_string())
                );
            }
        }
    }

    println!(
        "Dry run summary: {} resolved, {} unresolved.",
        resolved_count, unresolved_count
    );
    println!("Dry run - no files downloaded");
    Ok(())
}

fn preview_single_line(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}
