//! Parse input, resolve items to URLs, and enqueue with metadata.
//!
//! Do not log cookie_jar contents or URLs that could correlate with authenticated
//! sessions; limit debug logs to counts and non-sensitive metadata.

use std::sync::Arc;

use anyhow::{Result, bail};
use downloader_core::{
    InputType, Queue, QueueMetadata, ResolveContext,
    build_default_resolver_registry, build_preferred_filename, extract_reference_confidence,
    load_custom_topics, match_custom_topics, normalize_topics, parse_input,
    TopicExtractor,
};
use tracing::{debug, info, warn};

use crate::app::context::RunContext;
use crate::output;

/// Outcome of the resolution phase: counts and first error for runtime to decide bails.
pub(crate) struct ResolutionOutcome {
    pub(crate) parsed_item_count: usize,
    pub(crate) resolution_failed_count: usize,
    pub(crate) first_resolution_error: Option<String>,
}

/// Parses input text, resolves each item to a URL, enqueues with metadata.
/// When `ctx.input_text` is None, returns zeros (no input to parse).
pub(crate) async fn run_resolution(
    ctx: &RunContext,
    queue: Arc<Queue>,
) -> Result<ResolutionOutcome> {
    let Some(input_text) = &ctx.input_text else {
        return Ok(ResolutionOutcome {
            parsed_item_count: 0,
            resolution_failed_count: 0,
            first_resolution_error: None,
        });
    };

    let parse_result = parse_input(input_text);
    let parsed_item_count = parse_result.len();

    let counts = parse_result.type_counts();
    info!(
        parsed_total = parse_result.len(),
        urls = counts.urls,
        dois = counts.dois,
        references = counts.references,
        bibtex = counts.bibtex,
        skipped = parse_result.skipped_count(),
        "Parsed input"
    );
    for skipped in &parse_result.skipped {
        warn!(skipped = %skipped, "Skipped unrecognized input");
    }

    let mut resolution_failed_count = 0usize;
    let mut first_resolution_error: Option<String> = None;

    if parse_result.is_empty() {
        return Ok(ResolutionOutcome {
            parsed_item_count,
            resolution_failed_count: 0,
            first_resolution_error: None,
        });
    }

    output::log_parse_feedback(&parse_result);

    let resolver_registry =
        build_default_resolver_registry(ctx.cookie_jar.clone(), "downloader@example.com");
    let resolve_context = ResolveContext::default();

    let topic_extractor = if ctx.args.detect_topics {
        match TopicExtractor::new() {
            Ok(extractor) => {
                debug!("Topic extractor initialized");
                Some(extractor)
            }
            Err(error) => {
                debug!(error = %error, "Topic extraction unavailable (non-critical)");
                None
            }
        }
    } else {
        None
    };

    let custom_topics = if let Some(ref topics_path) = ctx.args.topics_file {
        match load_custom_topics(topics_path) {
            Ok(topics) => {
                info!(count = topics.len(), path = %topics_path.display(), "Loaded custom topics");
                topics
            }
            Err(error) => {
                bail!(
                    "Cannot read topics file '{}'\n  {error}\n  \
                    Check the path and ensure the file exists, or remove --topics-file flag.",
                    topics_path.display()
                );
            }
        }
    } else {
        Vec::new()
    };

    for item in &parse_result.items {
        let resolver_input = if item.input_type == InputType::BibTex {
            item.raw.as_str()
        } else {
            item.value.as_str()
        };

        let resolved_item = match resolver_registry
            .resolve_to_url(resolver_input, item.input_type, &resolve_context)
            .await
        {
            Ok(resolved) => {
                if !resolved.metadata.is_empty() {
                    debug!(
                        metadata_fields = resolved.metadata.len(),
                        "Resolver returned metadata"
                    );
                }
                Some(resolved)
            }
            Err(error) => {
                resolution_failed_count += 1;
                if first_resolution_error.is_none() {
                    first_resolution_error = Some(error.to_string());
                }
                // Do not log raw URLs (could correlate with authenticated sessions).
                let log_input = if item.input_type == InputType::Url {
                    "(url redacted)".to_string()
                } else if resolver_input.len() > 80 {
                    format!("{}...", &resolver_input[..80])
                } else {
                    resolver_input.to_string()
                };
                warn!(
                    input = %log_input,
                    input_type = %item.input_type,
                    error = %error,
                    "Skipped unresolved parsed item"
                );
                None
            }
        };

        let Some(resolved) = resolved_item else {
            continue;
        };
        let queue_value = resolved.url;

        if queue.has_active_url(&queue_value).await? {
            debug!("Skipping duplicate URL already in queue");
            continue;
        }

        let topics = topic_extractor.as_ref().and_then(|extractor| {
            let title = resolved.metadata.get("title")?;
            let raw_keywords = extractor.extract_from_metadata(title, None);
            if raw_keywords.is_empty() {
                return None;
            }
            let final_topics = if custom_topics.is_empty() {
                normalize_topics(raw_keywords)
            } else {
                match_custom_topics(raw_keywords, custom_topics.clone())
            };
            if final_topics.is_empty() {
                None
            } else {
                debug!(
                    count = final_topics.len(),
                    topics = ?final_topics,
                    "Extracted topics from metadata"
                );
                Some(final_topics)
            }
        });

        let reference_confidence = (item.input_type == InputType::Reference)
            .then(|| extract_reference_confidence(&item.raw));

        let queue_metadata = QueueMetadata {
            suggested_filename: Some(build_preferred_filename(
                &queue_value,
                &resolved.metadata,
            )),
            title: resolved.metadata.get("title").cloned(),
            authors: resolved.metadata.get("authors").cloned(),
            year: resolved.metadata.get("year").cloned(),
            doi: resolved.metadata.get("doi").cloned(),
            topics,
            parse_confidence: reference_confidence.map(|details| details.level.to_string()),
            parse_confidence_factors: reference_confidence
                .and_then(|details| serde_json::to_string(&details.factors).ok()),
        };

        queue
            .enqueue_with_metadata(
                &queue_value,
                item.input_type.queue_source_type(),
                Some(&item.raw),
                Some(&queue_metadata),
            )
            .await?;
        debug!(
            input_type = %item.input_type,
            source_type = item.input_type.queue_source_type(),
            "Enqueued parsed item"
        );
    }

    if resolution_failed_count > 0 {
        warn!(
            routing_skipped = resolution_failed_count,
            "Skipped parsed items that could not be resolved"
        );
    }

    Ok(ResolutionOutcome {
        parsed_item_count,
        resolution_failed_count,
        first_resolution_error,
    })
}

#[cfg(test)]
mod tests {
    use super::run_resolution;
    use crate::app::config_runtime::HttpTimeoutSettings;
    use crate::app::context::RunContext;
    use crate::cli::Cli;
    use clap::Parser;
    use downloader_core::{Database, DatabaseOptions, Queue};
    use std::path::PathBuf;
    use std::sync::Arc;

    #[tokio::test]
    async fn run_resolution_with_no_input_returns_zeros() {
        let db = Database::new_in_memory().await.unwrap();
        let queue = Arc::new(Queue::new(db));

        let cli = Cli::try_parse_from(["downloader"]).unwrap();
        let ctx = RunContext {
            args: cli.download,
            http_timeouts: HttpTimeoutSettings::default(),
            db_options: DatabaseOptions::default(),
            output_dir: PathBuf::from("."),
            cookie_jar: None,
            input_text: None,
            piped_stdin_was_empty: false,
        };

        let outcome = run_resolution(&ctx, queue).await.unwrap();

        assert_eq!(outcome.parsed_item_count, 0);
        assert_eq!(outcome.resolution_failed_count, 0);
        assert_eq!(outcome.first_resolution_error, None);
    }
}
