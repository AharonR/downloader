//! Topic normalization for deduplication and standardization.

use std::collections::HashSet;
use tracing::instrument;

/// Normalizes a list of topics by applying lowercase conversion and deduplication.
///
/// Normalization steps:
/// 1. Convert to lowercase for case-insensitive matching
/// 2. Trim whitespace
/// 3. Remove duplicates (case-insensitive)
/// 4. Sort alphabetically for consistent output
///
/// # Arguments
/// * `topics` - Raw topic strings from keyword extraction
///
/// # Returns
/// Normalized, deduplicated, and sorted topic list
#[must_use]
#[instrument(skip(topics), fields(count = topics.len()))]
pub fn normalize_topics(topics: Vec<String>) -> Vec<String> {
    let mut seen = HashSet::new();
    let mut normalized = Vec::new();

    for topic in topics {
        let normalized_topic = topic.trim().to_lowercase();

        // Skip empty topics after normalization
        if normalized_topic.is_empty() {
            continue;
        }

        // Deduplicate using HashSet (case-insensitive)
        if seen.insert(normalized_topic.clone()) {
            normalized.push(normalized_topic);
        }
    }

    // Sort alphabetically for consistent output
    normalized.sort();
    normalized
}

/// Matches custom topics against extracted keywords with priority.
///
/// Returns custom topics that match (case-insensitive substring match),
/// followed by extracted topics not in custom list.
///
/// # Arguments
/// * `extracted` - Topics extracted from paper metadata
/// * `custom` - User-provided custom topics for prioritized matching
#[must_use]
#[instrument(skip(extracted, custom), fields(extracted_count = extracted.len(), custom_count = custom.len()))]
pub fn match_custom_topics(extracted: Vec<String>, custom: Vec<String>) -> Vec<String> {
    let normalized_extracted = normalize_topics(extracted);
    let normalized_custom = normalize_topics(custom);

    let mut matched_custom = Vec::new();
    let mut unmatched_extracted = Vec::new();

    // Check each extracted topic against custom topics
    for extracted_topic in normalized_extracted {
        let mut found_match = false;

        for custom_topic in &normalized_custom {
            // Case-insensitive substring match (bi-directional)
            if extracted_topic.contains(custom_topic) || custom_topic.contains(&extracted_topic) {
                if !matched_custom.contains(custom_topic) {
                    matched_custom.push(custom_topic.clone());
                }
                found_match = true;
                break;
            }
        }

        if !found_match && !unmatched_extracted.contains(&extracted_topic) {
            unmatched_extracted.push(extracted_topic);
        }
    }

    // Return custom matches first, then unmatched extracted topics
    matched_custom.extend(unmatched_extracted);
    matched_custom
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_normalize_topics_lowercase_conversion() {
        let topics = vec![
            "Machine Learning".to_string(),
            "CLIMATE CHANGE".to_string(),
            "quantum Computing".to_string(),
        ];

        let normalized = normalize_topics(topics);

        assert_eq!(normalized[0], "climate change");
        assert_eq!(normalized[1], "machine learning");
        assert_eq!(normalized[2], "quantum computing");
    }

    #[test]
    fn test_normalize_topics_deduplication() {
        let topics = vec![
            "machine learning".to_string(),
            "Machine Learning".to_string(),
            "MACHINE LEARNING".to_string(),
            "climate change".to_string(),
        ];

        let normalized = normalize_topics(topics);

        assert_eq!(normalized.len(), 2, "Should deduplicate case-insensitive");
        assert!(normalized.contains(&"machine learning".to_string()));
        assert!(normalized.contains(&"climate change".to_string()));
    }

    #[test]
    fn test_normalize_topics_removes_empty() {
        let topics = vec![
            "valid topic".to_string(),
            "".to_string(),
            "  ".to_string(),
            "another topic".to_string(),
        ];

        let normalized = normalize_topics(topics);

        assert_eq!(normalized.len(), 2, "Should remove empty topics");
        assert!(normalized.contains(&"valid topic".to_string()));
        assert!(normalized.contains(&"another topic".to_string()));
    }

    #[test]
    fn test_normalize_topics_sorts_alphabetically() {
        let topics = vec![
            "zebra".to_string(),
            "apple".to_string(),
            "banana".to_string(),
        ];

        let normalized = normalize_topics(topics);

        assert_eq!(normalized, vec!["apple", "banana", "zebra"]);
    }

    #[test]
    fn test_normalize_topics_trims_whitespace() {
        let topics = vec![
            "  topic one  ".to_string(),
            "topic two\n".to_string(),
            "\ttopic three".to_string(),
        ];

        let normalized = normalize_topics(topics);

        assert_eq!(normalized[0], "topic one");
        assert_eq!(normalized[1], "topic three");
        assert_eq!(normalized[2], "topic two");
    }

    #[test]
    fn test_match_custom_topics_prioritizes_custom() {
        let extracted = vec![
            "machine learning".to_string(),
            "climate change".to_string(),
            "quantum computing".to_string(),
        ];
        let custom = vec!["climate".to_string(), "quantum".to_string()];

        let matched = match_custom_topics(extracted, custom);

        // Custom topics should come first
        assert_eq!(matched[0], "climate");
        assert_eq!(matched[1], "quantum");
        // Unmatched extracted topics follow
        assert_eq!(matched[2], "machine learning");
    }

    #[test]
    fn test_match_custom_topics_substring_matching() {
        let extracted = vec![
            "machine learning algorithms".to_string(),
            "deep learning networks".to_string(),
        ];
        let custom = vec!["learning".to_string()];

        let matched = match_custom_topics(extracted, custom);

        // "learning" matches both extracted topics
        assert_eq!(matched.len(), 1, "Should match custom topic once");
        assert_eq!(matched[0], "learning");
    }

    #[test]
    fn test_match_custom_topics_no_custom_returns_extracted() {
        let extracted = vec!["topic one".to_string(), "topic two".to_string()];
        let custom = vec![];

        let matched = match_custom_topics(extracted, custom);

        assert_eq!(matched.len(), 2);
        assert!(matched.contains(&"topic one".to_string()));
        assert!(matched.contains(&"topic two".to_string()));
    }

    #[test]
    fn test_match_custom_topics_no_matches_returns_all() {
        let extracted = vec!["machine learning".to_string()];
        let custom = vec!["climate change".to_string()];

        let matched = match_custom_topics(extracted, custom);

        assert_eq!(matched.len(), 1);
        assert_eq!(matched[0], "machine learning");
    }

    #[test]
    fn test_match_custom_topics_handles_case_insensitivity() {
        let extracted = vec!["Machine Learning".to_string()];
        let custom = vec!["MACHINE".to_string()];

        let matched = match_custom_topics(extracted, custom);

        assert_eq!(matched[0], "machine", "Should match case-insensitively");
    }

    // --- Regression tests for Story 8.1 code-review bug fixes ---

    /// Regression: enqueue flow previously called normalize_topics(match_custom_topics(...)),
    /// causing double normalization. match_custom_topics normalizes internally, so adding
    /// normalize_topics on top was redundant. This verifies match_custom_topics output is
    /// already in normalized form and calling normalize_topics again produces identical results.
    #[test]
    fn test_match_custom_topics_output_is_already_normalized_no_double_normalization() {
        let extracted = vec![
            "Machine Learning".to_string(),
            "Climate Change".to_string(),
            "NEURAL NETWORKS".to_string(),
        ];
        let custom = vec!["climate".to_string()];

        let once = match_custom_topics(extracted.clone(), custom.clone());
        // Applying normalize_topics again must produce the same result
        let twice = normalize_topics(once.clone());

        assert_eq!(
            once, twice,
            "match_custom_topics output is already normalized; \
             normalize_topics must be idempotent on it (regression: was called twice)"
        );
    }

    /// Regression: same check for the no-custom-topics path — normalize_topics output
    /// is stable under a second application of normalize_topics.
    #[test]
    fn test_normalize_topics_is_idempotent() {
        let raw = vec![
            "  Machine Learning  ".to_string(),
            "machine learning".to_string(), // duplicate
            "Climate Change".to_string(),
        ];

        let once = normalize_topics(raw);
        let twice = normalize_topics(once.clone());

        assert_eq!(
            once, twice,
            "normalize_topics must be idempotent (calling it twice gives same result)"
        );
    }
}
