//! Keyword extraction using RAKE (Rapid Automatic Keyword Extraction) algorithm.

use anyhow::{Context, Result};
use rake::{Rake, StopWords};
use stop_words::{LANGUAGE, get};
use tracing::instrument;

/// Topic extractor using RAKE algorithm for keyword extraction from text.
pub struct TopicExtractor {
    /// Cached RAKE instance with pre-built stop words.
    rake: Rake,
}

impl std::fmt::Debug for TopicExtractor {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TopicExtractor").finish()
    }
}

impl TopicExtractor {
    /// Creates a new topic extractor with English stop words.
    ///
    /// # Errors
    /// Returns error if stop words cannot be loaded.
    #[instrument]
    pub fn new() -> Result<Self> {
        let raw_stop_words = get(LANGUAGE::English);
        let mut sw = StopWords::new();
        for word in raw_stop_words {
            sw.insert(word);
        }
        Ok(Self {
            rake: Rake::new(sw),
        })
    }

    /// Extracts keywords from the given text using RAKE algorithm.
    ///
    /// Returns up to 10 keywords sorted by relevance score.
    ///
    /// # Arguments
    /// * `text` - Input text (title, abstract, or combined metadata)
    ///
    /// # Returns
    /// Vector of extracted keyword strings
    #[must_use]
    #[instrument(skip(self))]
    pub fn extract(&self, text: &str) -> Vec<String> {
        if text.trim().is_empty() {
            return Vec::new();
        }
        let keywords = self.rake.run(text);

        // Take top 10 keywords and extract just the keyword strings
        keywords
            .into_iter()
            .take(10)
            .map(|keyword_score| keyword_score.keyword)
            .collect()
    }

    /// Extracts keywords from title and abstract combined.
    ///
    /// Weights title keywords higher by including title text twice.
    ///
    /// # Arguments
    /// * `title` - Paper title
    /// * `abstract_text` - Paper abstract (optional)
    #[must_use]
    #[instrument(skip(self))]
    pub fn extract_from_metadata(&self, title: &str, abstract_text: Option<&str>) -> Vec<String> {
        let combined = match abstract_text {
            Some(abstract_str) if !abstract_str.trim().is_empty() => {
                format!("{title}. {title}. {abstract_str}")
            }
            _ => title.to_string(),
        };

        self.extract(&combined)
    }
}

/// Convenience function to extract keywords from text.
///
/// Creates a temporary extractor and extracts keywords.
/// For repeated extractions, create a `TopicExtractor` instance instead.
///
/// # Errors
/// Returns error if stop words cannot be loaded.
#[instrument]
pub fn extract_keywords(text: &str) -> Result<Vec<String>> {
    let extractor = TopicExtractor::new().context("Failed to initialize topic extractor")?;
    Ok(extractor.extract(text))
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn test_topic_extractor_creates_successfully() {
        let result = TopicExtractor::new();
        assert!(result.is_ok(), "TopicExtractor should initialize");
    }

    #[test]
    fn test_extract_keywords_from_academic_title() {
        let extractor = TopicExtractor::new().unwrap();
        let title = "Machine Learning Approaches to Climate Change Prediction";
        let keywords = extractor.extract(title);

        assert!(!keywords.is_empty(), "Should extract keywords from title");
    }

    #[test]
    fn test_extract_keywords_empty_input_returns_empty() {
        let extractor = TopicExtractor::new().unwrap();
        let keywords = extractor.extract("");
        assert!(keywords.is_empty(), "Empty input should return no keywords");

        let keywords = extractor.extract("   ");
        assert!(
            keywords.is_empty(),
            "Whitespace-only input should return no keywords"
        );
    }

    #[test]
    fn test_extract_keywords_limits_to_ten() {
        let extractor = TopicExtractor::new().unwrap();
        let long_text = "climate change global warming temperature increase carbon dioxide emissions greenhouse gases renewable energy solar power wind energy fossil fuels sustainability environmental impact biodiversity conservation ecosystem degradation pollution air quality water resources ocean acidification sea level rise extreme weather natural disasters mitigation adaptation resilience policy regulations";
        let keywords = extractor.extract(long_text);

        assert!(
            keywords.len() <= 10,
            "Should limit to 10 keywords, got {}",
            keywords.len()
        );
    }

    #[test]
    fn test_extract_from_metadata_title_only() {
        let extractor = TopicExtractor::new().unwrap();
        let title = "Quantum Computing Applications";

        let keywords = extractor.extract_from_metadata(title, None);
        assert!(!keywords.is_empty(), "Should extract from title alone");

        let keywords_empty = extractor.extract_from_metadata(title, Some(""));
        assert_eq!(
            keywords, keywords_empty,
            "Empty abstract should behave like None"
        );
    }

    #[test]
    fn test_extract_from_metadata_with_abstract() {
        let extractor = TopicExtractor::new().unwrap();
        let title = "Deep Learning for Natural Language Processing";
        let abstract_text =
            "This paper presents novel approaches to NLP using neural networks and transformers";

        let keywords = extractor.extract_from_metadata(title, Some(abstract_text));
        assert!(
            !keywords.is_empty(),
            "Should extract from combined metadata"
        );
    }

    #[test]
    fn test_extract_keywords_convenience_function() {
        let result = extract_keywords("Artificial Intelligence in Healthcare");
        assert!(result.is_ok(), "Convenience function should succeed");
        assert!(!result.unwrap().is_empty(), "Should extract keywords");
    }

    #[test]
    fn test_extract_keywords_empty_returns_ok_empty() {
        let result = extract_keywords("");
        assert!(result.is_ok(), "Should handle empty input gracefully");
        assert!(
            result.unwrap().is_empty(),
            "Empty input should yield no keywords"
        );
    }

    // --- Regression tests for Story 8.1 code-review bug fixes ---

    /// Regression: TopicExtractor previously rebuilt StopWords and Rake on every extract() call.
    /// Bug: the Rake instance was not cached in the struct; each call paid the full initialization
    /// cost and any state within Rake could not be reused.
    /// This test verifies that reusing a single TopicExtractor instance produces consistent
    /// keyword sets across multiple calls (RAKE ordering may vary, but the set of keywords must
    /// be the same — a state-mutation bug would cause keyword set divergence).
    #[test]
    fn test_topic_extractor_reuse_produces_consistent_results() {
        let extractor = TopicExtractor::new().unwrap();
        let text = "Machine Learning Approaches to Natural Language Processing";

        let mut first = extractor.extract(text);
        let mut second = extractor.extract(text);
        let mut third = extractor.extract(text);

        // Sort before comparing: RAKE's internal HashMap ordering is non-deterministic,
        // but the keyword SET (what RAKE identifies as relevant) must be stable.
        first.sort();
        second.sort();
        third.sort();

        assert_eq!(
            first, second,
            "Reusing TopicExtractor must yield the same keyword set on identical input (1st vs 2nd)"
        );
        assert_eq!(
            second, third,
            "Reusing TopicExtractor must yield the same keyword set on identical input (2nd vs 3rd)"
        );
        assert!(
            !first.is_empty(),
            "Extractor should return keywords for non-trivial input"
        );
    }

    /// Regression: cached Rake instance must handle alternating inputs without cross-contamination.
    #[test]
    fn test_topic_extractor_cached_rake_no_state_bleed_between_calls() {
        let extractor = TopicExtractor::new().unwrap();

        let mut ml_keywords = extractor.extract("Machine Learning Neural Networks Deep Learning");
        let climate_keywords = extractor.extract("Climate Change Global Warming Carbon Emissions");
        let mut ml_keywords_again =
            extractor.extract("Machine Learning Neural Networks Deep Learning");

        ml_keywords.sort();
        ml_keywords_again.sort();

        // Results for different inputs must differ (as sets)
        assert_ne!(
            ml_keywords, climate_keywords,
            "Different inputs must yield different keyword sets (no state bleed from caching)"
        );

        // Calling back with first input must yield the same keyword set as the original call
        assert_eq!(
            ml_keywords, ml_keywords_again,
            "Cached Rake must be stateless between calls — same input must yield same keyword set"
        );
    }
}
