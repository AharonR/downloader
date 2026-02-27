//! Reference string detection and metadata extraction.

use std::collections::HashMap;
use std::fmt;
use std::sync::LazyLock;
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};

use regex::Regex;
use serde::{Deserialize, Serialize};
use tracing::debug;

use super::error::ParseError;
use super::input::ParsedItem;

/// Regex for parenthesized years like `(2024)`.
#[allow(clippy::expect_used)]
static YEAR_PAREN_PATTERN: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"\((\d{4})\)").expect("reference year (paren) regex is valid") // Static pattern, safe to panic
});

/// Regex for bare years like `2024`.
#[allow(clippy::expect_used)]
static YEAR_BARE_PATTERN: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"\b((?:18|19|20)\d{2})\b").expect("reference year (bare) regex is valid") // Static pattern, safe to panic
});

/// Regex for author patterns like `Smith, J.` or `Smith, John`.
#[allow(clippy::expect_used)]
static AUTHOR_PATTERN: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"([\p{Lu}][\p{L}'`\-]+,\s*(?:[\p{Lu}]\.|[\p{Lu}][\p{L}]+(?:\s+[\p{Lu}][\p{L}]+)*))")
        .expect("reference author regex is valid") // Static pattern, safe to panic
});

/// Regex for `et al.` references.
#[allow(clippy::expect_used)]
static ET_AL_PATTERN: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"([\p{Lu}][\p{L}'`\-]+\s+et al\.)").expect("reference et-al regex is valid") // Static pattern, safe to panic
});
static REFERENCE_METADATA_CACHE: LazyLock<Mutex<HashMap<String, ReferenceMetadata>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));
const REFERENCE_METADATA_CACHE_MAX_ENTRIES: usize = 2_048;

/// Result type for reference extraction operations.
pub type ReferenceExtractionResult = Result<ParsedItem, ParseError>;

/// Confidence level for extracted reference metadata.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Confidence {
    /// Author, year, and title found.
    High,
    /// At least two of author/year/title found.
    Medium,
    /// Only one field found (or default when empty).
    Low,
}

impl fmt::Display for Confidence {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::High => write!(f, "high"),
            Self::Medium => write!(f, "medium"),
            Self::Low => write!(f, "low"),
        }
    }
}

/// Deterministic factors used to compute reference confidence.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct ConfidenceFactors {
    /// Whether at least one author token was extracted.
    pub has_authors: bool,
    /// Whether a publication year was extracted.
    pub has_year: bool,
    /// Whether a title candidate was extracted.
    pub has_title: bool,
    /// Number of extracted authors.
    pub author_count: usize,
}

impl ConfidenceFactors {
    /// Returns the derived confidence level for these factors.
    #[must_use]
    pub fn level(self) -> Confidence {
        let present = usize::from(self.has_authors)
            + usize::from(self.has_year)
            + usize::from(self.has_title);
        match present {
            3 => Confidence::High,
            2 => Confidence::Medium,
            _ => Confidence::Low,
        }
    }
}

/// Stable confidence payload for downstream persistence.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ReferenceConfidence {
    /// Computed confidence level.
    pub level: Confidence,
    /// Deterministic confidence factors.
    pub factors: ConfidenceFactors,
}

/// Structured metadata extracted from a single reference string.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReferenceMetadata {
    /// Extracted author names.
    pub authors: Vec<String>,
    /// Extracted publication year.
    pub year: Option<u16>,
    /// Extracted title text.
    pub title: Option<String>,
    /// Confidence level for extraction quality.
    pub confidence: Confidence,
    /// Deterministic factors behind confidence classification.
    pub confidence_factors: ConfidenceFactors,
}

impl ReferenceMetadata {
    /// Creates empty metadata with low confidence.
    #[must_use]
    pub fn new() -> Self {
        Self {
            authors: Vec::new(),
            year: None,
            title: None,
            confidence: Confidence::Low,
            confidence_factors: ConfidenceFactors::default(),
        }
    }

    /// Recomputes confidence based on available extracted fields.
    pub fn compute_confidence(&mut self) {
        self.confidence_factors = ConfidenceFactors {
            has_authors: !self.authors.is_empty(),
            has_year: self.year.is_some(),
            has_title: self.title.is_some(),
            author_count: self.authors.len(),
        };
        self.confidence = self.confidence_factors.level();
    }

    /// Returns level+factors snapshot for downstream consumers.
    #[must_use]
    pub fn confidence_details(&self) -> ReferenceConfidence {
        ReferenceConfidence {
            level: self.confidence,
            factors: self.confidence_factors,
        }
    }
}

impl Default for ReferenceMetadata {
    fn default() -> Self {
        Self::new()
    }
}

/// Parses a reference string into structured metadata.
#[tracing::instrument(skip(text), fields(input_len = text.len()))]
#[must_use]
pub fn parse_reference_metadata(text: &str) -> ReferenceMetadata {
    if let Some(cached) = {
        let cache = REFERENCE_METADATA_CACHE
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        cache.get(text).cloned()
    } {
        debug!(
            confidence = %cached.confidence,
            has_authors = cached.confidence_factors.has_authors,
            has_year = cached.confidence_factors.has_year,
            has_title = cached.confidence_factors.has_title,
            author_count = cached.confidence_factors.author_count,
            cache_hit = true,
            "Parsed reference confidence factors"
        );
        return cached;
    }

    let mut metadata = ReferenceMetadata::new();
    let (year_pos, year_end_pos) = find_year_with_position(text)
        .map_or((None, None), |(_, start, end)| (Some(start), Some(end)));

    metadata.year = extract_year(text);
    metadata.authors = extract_authors(text, year_pos);
    metadata.title = extract_title(text, year_end_pos);
    metadata.compute_confidence();
    debug!(
        confidence = %metadata.confidence,
        has_authors = metadata.confidence_factors.has_authors,
        has_year = metadata.confidence_factors.has_year,
        has_title = metadata.confidence_factors.has_title,
        author_count = metadata.confidence_factors.author_count,
        "Parsed reference confidence factors"
    );

    {
        let mut cache = REFERENCE_METADATA_CACHE
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        // Keep cache bounded for long-running sessions.
        if cache.len() >= REFERENCE_METADATA_CACHE_MAX_ENTRIES {
            cache.clear();
        }
        cache.insert(text.to_string(), metadata.clone());
    }

    metadata
}

/// Returns confidence level and factors for a reference text.
///
/// This helper relies on the parser cache, allowing downstream code to consume
/// confidence details without reparsing reference text in normal workflows.
#[must_use]
pub fn extract_reference_confidence(text: &str) -> ReferenceConfidence {
    parse_reference_metadata(text).confidence_details()
}

/// Extracts reference-like lines from input text.
#[tracing::instrument(skip(text), fields(input_len = text.len()))]
#[must_use]
pub fn extract_references(text: &str) -> Vec<ReferenceExtractionResult> {
    let mut results = Vec::new();

    for line in text.lines() {
        let candidate = line.trim();
        if candidate.is_empty() || !looks_like_reference(candidate) {
            continue;
        }

        let metadata = parse_reference_metadata(candidate);
        if metadata.confidence != Confidence::Low || !metadata.authors.is_empty() {
            debug!(
                reference = %candidate,
                confidence = %metadata.confidence,
                has_authors = metadata.confidence_factors.has_authors,
                has_year = metadata.confidence_factors.has_year,
                has_title = metadata.confidence_factors.has_title,
                author_count = metadata.confidence_factors.author_count,
                "Reference extracted"
            );
            results.push(Ok(ParsedItem::reference(candidate, candidate)));
        } else {
            debug!(reference = %candidate, "Reference-like line was unparseable");
            results.push(Err(ParseError::unparseable_reference(candidate)));
        }
    }

    results
}

/// Extracts a 4-digit publication year from a reference.
#[must_use]
fn extract_year(text: &str) -> Option<u16> {
    find_year_with_position(text).map(|(year, _, _)| year)
}

fn find_year_with_position(text: &str) -> Option<(u16, usize, usize)> {
    find_year_in_range(text, current_year_utc().saturating_add(1))
}

/// Searches for a year in text within the range `1800..=max_year`.
///
/// Separated from [`find_year_with_position`] for testability — avoids
/// coupling tests to `SystemTime::now()` (per project-context.md).
fn find_year_in_range(text: &str, max_year: u16) -> Option<(u16, usize, usize)> {
    for cap in YEAR_PAREN_PATTERN.captures_iter(text) {
        if let Some(m) = cap.get(1)
            && let Ok(year) = m.as_str().parse::<u16>()
            && (1800..=max_year).contains(&year)
        {
            return Some((year, m.start(), m.end()));
        }
    }

    for cap in YEAR_BARE_PATTERN.captures_iter(text) {
        if let Some(m) = cap.get(1)
            && let Ok(year) = m.as_str().parse::<u16>()
            && (1800..=max_year).contains(&year)
        {
            return Some((year, m.start(), m.end()));
        }
    }

    None
}

/// Extracts author list from the start of a reference up to year position.
#[must_use]
fn extract_authors(text: &str, year_pos: Option<usize>) -> Vec<String> {
    let prefix = year_pos
        .and_then(|pos| text.get(..pos))
        .unwrap_or(text)
        .trim()
        .trim_start_matches(|c: char| c.is_ascii_digit() || c == '.' || c == '[' || c == ']')
        .trim();

    if let Some(cap) = ET_AL_PATTERN.captures(prefix)
        && let Some(m) = cap.get(1)
    {
        return vec![m.as_str().trim().to_string()];
    }

    AUTHOR_PATTERN
        .captures_iter(prefix)
        .filter_map(|cap| {
            cap.get(1)
                .map(|m| m.as_str().trim().trim_end_matches(',').to_string())
        })
        .collect()
}

/// Extracts title from a reference using year-anchored and heuristic strategies.
#[must_use]
fn extract_title(text: &str, year_end_pos: Option<usize>) -> Option<String> {
    if let Some(pos) = year_end_pos
        && let Some(after_year) = text.get(pos..)
    {
        let normalized = after_year
            .trim_start_matches(|c: char| {
                c.is_whitespace() || matches!(c, ')' | '.' | ',' | ':' | ';' | '-')
            })
            .trim();

        if let Some(segment) = normalized.split(". ").next()
            && let Some(title) = clean_title(segment)
        {
            return Some(title);
        }
    }

    text.split(['.', ';', '!', '?'])
        .map(str::trim)
        .filter(|segment| segment.len() > 10)
        .filter(|segment| segment.chars().next().is_some_and(char::is_uppercase))
        .max_by_key(|segment| segment.len())
        .and_then(clean_title)
}

fn clean_title(raw: &str) -> Option<String> {
    let cleaned = raw
        .trim()
        .trim_matches('"')
        .trim_matches('\'')
        .trim_end_matches('.');

    if cleaned.len() < 3 {
        return None;
    }

    Some(cleaned.to_string())
}

/// Conservative heuristic for deciding if a line looks like a citation reference.
#[must_use]
fn looks_like_reference(line: &str) -> bool {
    let trimmed = line.trim();
    if trimmed.len() <= 20 {
        return false;
    }

    let has_year = YEAR_BARE_PATTERN.is_match(trimmed);
    let comma_count = trimmed.matches(',').count();
    let lower = trimmed.to_ascii_lowercase();
    let has_keyword = ["journal", "vol.", "pp.", "et al."]
        .iter()
        .any(|keyword| lower.contains(keyword));

    has_year || comma_count >= 3 || has_keyword
}

fn current_year_utc() -> u16 {
    // Fallback chain:
    // 1) Try system time since UNIX_EPOCH
    // 2) If conversion fails/overflows, use i64::MAX seconds as sentinel
    // 3) Convert to civil year
    // 4) Clamp to u16, final fallback 2100 for out-of-range years
    let seconds = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .ok()
        .and_then(|duration| i64::try_from(duration.as_secs()).ok())
        .unwrap_or(i64::MAX);
    let days = seconds.div_euclid(86_400);
    let year = civil_year_from_days(days);
    u16::try_from(year).unwrap_or(2100)
}

// Convert days since unix epoch to civil year in UTC.
fn civil_year_from_days(days_since_unix_epoch: i64) -> i64 {
    let z = days_since_unix_epoch + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let month = mp + if mp < 10 { 3 } else { -9 };
    y + i64::from(month <= 2)
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use std::sync::{Arc, Mutex};
    use std::time::{SystemTime, UNIX_EPOCH};
    use tracing::field::{Field, Visit};
    use tracing::{Event, Subscriber};
    use tracing_subscriber::layer::{Context, Layer};
    use tracing_subscriber::prelude::*;
    use tracing_subscriber::registry::LookupSpan;

    #[derive(Debug, Default)]
    struct CapturedEvent {
        fields: HashMap<String, String>,
    }

    #[derive(Default)]
    struct EventFieldVisitor {
        fields: HashMap<String, String>,
    }

    impl EventFieldVisitor {
        fn into_event(self) -> CapturedEvent {
            CapturedEvent {
                fields: self.fields,
            }
        }
    }

    impl Visit for EventFieldVisitor {
        fn record_bool(&mut self, field: &Field, value: bool) {
            self.fields
                .insert(field.name().to_string(), value.to_string());
        }

        fn record_i64(&mut self, field: &Field, value: i64) {
            self.fields
                .insert(field.name().to_string(), value.to_string());
        }

        fn record_u64(&mut self, field: &Field, value: u64) {
            self.fields
                .insert(field.name().to_string(), value.to_string());
        }

        fn record_str(&mut self, field: &Field, value: &str) {
            self.fields
                .insert(field.name().to_string(), value.to_string());
        }

        fn record_debug(&mut self, field: &Field, value: &dyn std::fmt::Debug) {
            self.fields
                .insert(field.name().to_string(), format!("{value:?}"));
        }
    }

    #[derive(Clone)]
    struct EventCaptureLayer {
        events: Arc<Mutex<Vec<CapturedEvent>>>,
    }

    impl<S> Layer<S> for EventCaptureLayer
    where
        S: Subscriber + for<'lookup> LookupSpan<'lookup>,
    {
        fn on_event(&self, event: &Event<'_>, _ctx: Context<'_, S>) {
            let mut visitor = EventFieldVisitor::default();
            event.record(&mut visitor);
            let mut events = self
                .events
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            events.push(visitor.into_event());
        }
    }

    // ==================== Year Extraction ====================

    #[test]
    fn test_extract_year_parenthesized() {
        assert_eq!(extract_year("Smith, J. (2024). Title."), Some(2024));
    }

    #[test]
    fn test_extract_year_bare() {
        assert_eq!(extract_year("Published in 2024 in Journal"), Some(2024));
    }

    #[test]
    fn test_extract_year_out_of_range() {
        assert_eq!(extract_year("Smith, J. (1799). Title."), None);
    }

    #[test]
    fn test_extract_year_no_year() {
        assert_eq!(extract_year("Smith, J. Title. Journal."), None);
    }

    #[test]
    fn test_extract_year_multiple_prefers_parenthesized() {
        assert_eq!(extract_year("Published 2020 by Smith (2024)."), Some(2024));
    }

    // ==================== Author Extraction ====================

    #[test]
    fn test_extract_authors_single() {
        let line = "Smith, J. (2024). Paper Title. Journal.";
        let year_pos = line.find("2024");
        let authors = extract_authors(line, year_pos);
        assert_eq!(authors, vec!["Smith, J."]);
    }

    #[test]
    fn test_extract_authors_last_first_name() {
        let line = "Smith, John (2024). Paper Title. Journal.";
        let year_pos = line.find("2024");
        let authors = extract_authors(line, year_pos);
        assert_eq!(authors, vec!["Smith, John"]);
    }

    #[test]
    fn test_extract_authors_multiple_ampersand() {
        let line = "Smith, J., & Jones, K. (2024). Title.";
        let year_pos = line.find("2024");
        let authors = extract_authors(line, year_pos);
        assert_eq!(authors.len(), 2);
        assert_eq!(authors[0], "Smith, J.");
        assert_eq!(authors[1], "Jones, K.");
    }

    #[test]
    fn test_extract_authors_et_al() {
        let line = "Smith et al. (2024). Title.";
        let year_pos = line.find("2024");
        let authors = extract_authors(line, year_pos);
        assert_eq!(authors, vec!["Smith et al."]);
    }

    #[test]
    fn test_extract_authors_no_match() {
        let authors = extract_authors("this is random text", None);
        assert!(authors.is_empty());
    }

    #[test]
    fn test_extract_authors_oxford_comma() {
        let line = "Smith, J., Jones, K., & Brown, L. (2024). Title.";
        let year_pos = line.find("2024");
        let authors = extract_authors(line, year_pos);
        assert_eq!(authors.len(), 3);
        assert_eq!(authors[0], "Smith, J.");
        assert_eq!(authors[1], "Jones, K.");
        assert_eq!(authors[2], "Brown, L.");
    }

    #[test]
    fn test_extract_authors_unicode_name() {
        let line = "García, J. (2024). Título. Revista.";
        let year_pos = line.find("2024");
        let authors = extract_authors(line, year_pos);
        assert_eq!(authors, vec!["García, J."]);
    }

    // ==================== Title Extraction ====================

    #[test]
    fn test_extract_title_apa_style() {
        let line = "Smith, J. (2024). Paper Title Here. Journal Name, 1(2), 3-4.";
        let year_end = line.find("2024").map(|pos| pos + 4);
        let title = extract_title(line, year_end);
        assert_eq!(title, Some("Paper Title Here".to_string()));
    }

    #[test]
    fn test_extract_title_fallback_heuristic() {
        let line = "In this work we discuss methods; Another Candidate Title Segment";
        let title = extract_title(line, None);
        assert_eq!(title, Some("Another Candidate Title Segment".to_string()));
    }

    #[test]
    fn test_extract_title_no_title() {
        let title = extract_title("a b c", None);
        assert_eq!(title, None);
    }

    #[test]
    fn test_extract_title_strips_trailing_period() {
        let line = "Smith, J. (2024). Paper Title. Journal.";
        let year_end = line.find("2024").map(|pos| pos + 4);
        let title = extract_title(line, year_end);
        assert_eq!(title, Some("Paper Title".to_string()));
    }

    // ==================== Confidence ====================

    #[test]
    fn test_confidence_high() {
        let mut metadata = ReferenceMetadata {
            authors: vec!["Smith, J.".to_string()],
            year: Some(2024),
            title: Some("Title".to_string()),
            confidence: Confidence::Low,
            confidence_factors: ConfidenceFactors::default(),
        };
        metadata.compute_confidence();
        assert_eq!(metadata.confidence, Confidence::High);
        assert_eq!(
            metadata.confidence_factors,
            ConfidenceFactors {
                has_authors: true,
                has_year: true,
                has_title: true,
                author_count: 1
            }
        );
    }

    #[test]
    fn test_confidence_medium() {
        let mut metadata = ReferenceMetadata {
            authors: vec!["Smith, J.".to_string()],
            year: Some(2024),
            title: None,
            confidence: Confidence::Low,
            confidence_factors: ConfidenceFactors::default(),
        };
        metadata.compute_confidence();
        assert_eq!(metadata.confidence, Confidence::Medium);
    }

    #[test]
    fn test_confidence_low() {
        let mut metadata = ReferenceMetadata {
            authors: Vec::new(),
            year: Some(2024),
            title: None,
            confidence: Confidence::High,
            confidence_factors: ConfidenceFactors::default(),
        };
        metadata.compute_confidence();
        assert_eq!(metadata.confidence, Confidence::Low);
    }

    #[test]
    fn test_extract_reference_confidence_returns_level_and_factors() {
        let details = extract_reference_confidence("Smith, J. (2024). Paper Title. Journal.");
        assert_eq!(details.level, Confidence::High);
        assert_eq!(
            details.factors,
            ConfidenceFactors {
                has_authors: true,
                has_year: true,
                has_title: true,
                author_count: 1
            }
        );
    }

    #[test]
    fn test_parse_reference_metadata_logs_structured_confidence_fields() {
        let captured = Arc::new(Mutex::new(Vec::new()));
        let subscriber = tracing_subscriber::registry()
            .with(tracing_subscriber::filter::LevelFilter::DEBUG)
            .with(EventCaptureLayer {
                events: Arc::clone(&captured),
            });
        let suffix = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let reference = format!("Smith, J. (2024). Structured Title {suffix}. Journal Name.");

        tracing::subscriber::with_default(subscriber, || {
            // Warm up the callsite under our subscriber; a parallel test running
            // with the noop dispatcher may have cached Interest::Never atomically.
            // Rebuilding the cache ensures our subscriber's Interest::Always wins.
            let _ = parse_reference_metadata(&reference);
            tracing::callsite::rebuild_interest_cache();
            let _ = parse_reference_metadata(&reference);
        });

        let events = captured
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let confidence_event = events
            .iter()
            .find(|event| {
                event.fields.get("message").map(String::as_str)
                    == Some("Parsed reference confidence factors")
            })
            .expect("expected confidence debug event");

        assert_eq!(
            confidence_event
                .fields
                .get("confidence")
                .map(String::as_str),
            Some("high")
        );
        assert_eq!(
            confidence_event
                .fields
                .get("has_authors")
                .map(String::as_str),
            Some("true")
        );
        assert_eq!(
            confidence_event.fields.get("has_year").map(String::as_str),
            Some("true")
        );
        assert_eq!(
            confidence_event.fields.get("has_title").map(String::as_str),
            Some("true")
        );
        assert_eq!(
            confidence_event
                .fields
                .get("author_count")
                .map(String::as_str),
            Some("1")
        );
    }

    // ==================== Pipeline ====================

    #[test]
    fn test_extract_references_apa_full() {
        let input = "Smith, J. (2024). Paper Title. Journal Name, 1(2), 3-4.";
        let results = extract_references(input);
        assert_eq!(results.len(), 1);
        assert!(results[0].is_ok());
    }

    #[test]
    fn test_extract_references_partial_match() {
        let input = "2024 publication details, unknown format, additional context";
        let results = extract_references(input);
        assert_eq!(results.len(), 1);
        assert!(results[0].is_ok());
    }

    #[test]
    fn test_extract_references_looks_like_ref_but_fails() {
        let input = "foo, bar, baz, qux, quux, corge";
        let results = extract_references(input);
        assert_eq!(results.len(), 1);
        assert!(matches!(
            &results[0],
            Err(ParseError::UnparseableReference { .. })
        ));
    }

    #[test]
    fn test_extract_references_plain_text_ignored() {
        let results = extract_references("hello world");
        assert!(results.is_empty());
    }

    #[test]
    fn test_looks_like_reference_with_year() {
        let line = "Smith, J. (2024). Title. Journal Name, 1(2), 3-4.";
        assert!(looks_like_reference(line));
    }

    #[test]
    fn test_looks_like_reference_short_text() {
        assert!(!looks_like_reference("too short"));
    }

    // ==================== Keyword Branch (M4 fix) ====================

    #[test]
    fn test_looks_like_reference_keyword_no_year() {
        // Has keyword "Journal" but no year and < 3 commas — keyword branch
        assert!(looks_like_reference(
            "Full paper in Journal of Applied Sciences"
        ));
    }

    #[test]
    fn test_looks_like_reference_keyword_et_al() {
        assert!(looks_like_reference("Smith et al. wrote about the topic"));
    }

    // ==================== Year Upper Bound (M5 fix) ====================

    #[test]
    fn test_extract_year_future_out_of_range() {
        // Year 2099 with max_year=2027 should not match
        assert_eq!(find_year_in_range("Smith, J. (2099). Title.", 2027), None);
    }

    #[test]
    fn test_extract_year_at_upper_bound() {
        // Year exactly at max_year should match
        assert!(find_year_in_range("Smith, J. (2027). Title.", 2027).is_some());
    }

    #[test]
    fn test_extract_year_above_upper_bound() {
        // Year one above max_year should not match
        assert_eq!(find_year_in_range("Smith, J. (2028). Title.", 2027), None);
    }

    // ==================== Calendar Algorithm (M3 fix) ====================

    #[test]
    fn test_civil_year_from_days_unix_epoch() {
        // Day 0 = 1970-01-01
        assert_eq!(civil_year_from_days(0), 1970);
    }

    #[test]
    fn test_civil_year_from_days_y2k() {
        // 2000-01-01 is 10957 days after epoch
        assert_eq!(civil_year_from_days(10_957), 2000);
    }

    #[test]
    fn test_civil_year_from_days_2024() {
        // 2024-01-01 is 19723 days after epoch
        assert_eq!(civil_year_from_days(19_723), 2024);
    }

    #[test]
    fn test_civil_year_from_days_pre_epoch() {
        // 1969-12-31 is day -1
        assert_eq!(civil_year_from_days(-1), 1969);
    }
}
