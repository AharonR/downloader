//! Types representing parsed input items and results.

use std::fmt;

/// Type of input detected.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InputType {
    /// Direct HTTP/HTTPS URL
    Url,
    /// DOI identifier (10.XXXX/suffix format)
    Doi,
    /// Reference string (Author, Year, Title format)
    Reference,
    /// BibTeX entry (@article, @book, @inproceedings)
    BibTex,
    /// Could not determine type
    Unknown,
}

impl InputType {
    /// Returns the queue source type label used by queue persistence.
    ///
    /// `BibTex` uses an explicit `bibtex` source label so downstream routing
    /// and logging can preserve original parser classification.
    #[must_use]
    pub fn queue_source_type(self) -> &'static str {
        match self {
            Self::Url => "direct_url",
            Self::Doi => "doi",
            Self::Reference | Self::Unknown => "reference",
            Self::BibTex => "bibtex",
        }
    }
}

/// Per-type counts for parsed input items.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ParseTypeCounts {
    /// Number of URL items.
    pub urls: usize,
    /// Number of DOI items.
    pub dois: usize,
    /// Number of reference items.
    pub references: usize,
    /// Number of BibTeX items.
    pub bibtex: usize,
    /// Number of unknown items.
    pub unknown: usize,
}

impl ParseTypeCounts {
    /// Returns the total number of parsed items across all types.
    #[must_use]
    pub fn total(self) -> usize {
        self.urls + self.dois + self.references + self.bibtex + self.unknown
    }
}

impl fmt::Display for InputType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Url => write!(f, "URL"),
            Self::Doi => write!(f, "DOI"),
            Self::Reference => write!(f, "Reference"),
            Self::BibTex => write!(f, "BibTeX"),
            Self::Unknown => write!(f, "Unknown"),
        }
    }
}

/// A single parsed item from input.
#[derive(Debug, Clone)]
pub struct ParsedItem {
    /// Original input text
    pub raw: String,
    /// Detected input type
    pub input_type: InputType,
    /// Extracted/normalized value (e.g., validated URL)
    pub value: String,
}

impl ParsedItem {
    /// Creates a new parsed item.
    #[must_use]
    pub fn new(raw: impl Into<String>, input_type: InputType, value: impl Into<String>) -> Self {
        Self {
            raw: raw.into(),
            input_type,
            value: value.into(),
        }
    }

    /// Creates a URL item.
    #[must_use]
    pub fn url(raw: impl Into<String>, normalized: impl Into<String>) -> Self {
        Self::new(raw, InputType::Url, normalized)
    }

    /// Creates a DOI item.
    #[must_use]
    pub fn doi(raw: impl Into<String>, normalized: impl Into<String>) -> Self {
        Self::new(raw, InputType::Doi, normalized)
    }

    /// Creates a reference item.
    #[must_use]
    pub fn reference(raw: impl Into<String>, value: impl Into<String>) -> Self {
        Self::new(raw, InputType::Reference, value)
    }

    /// Creates a BibTeX item.
    #[must_use]
    pub fn bibtex(raw: impl Into<String>, value: impl Into<String>) -> Self {
        Self::new(raw, InputType::BibTex, value)
    }
}

impl fmt::Display for ParsedItem {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "[{}] {}", self.input_type, self.value)
    }
}

/// Collection of parsed items from input.
#[derive(Debug, Default)]
pub struct ParseResult {
    /// Successfully parsed items
    pub items: Vec<ParsedItem>,
    /// Lines/items that could not be parsed (for logging)
    pub skipped: Vec<String>,
}

impl ParseResult {
    /// Creates a new empty result.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Adds a successfully parsed item.
    pub fn add_item(&mut self, item: ParsedItem) {
        self.items.push(item);
    }

    /// Adds a skipped line (non-parseable).
    pub fn add_skipped(&mut self, line: impl Into<String>) {
        self.skipped.push(line.into());
    }

    /// Returns true if no items were parsed.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    /// Returns count of parsed items.
    #[must_use]
    pub fn len(&self) -> usize {
        self.items.len()
    }

    /// Returns count of skipped items.
    #[must_use]
    pub fn skipped_count(&self) -> usize {
        self.skipped.len()
    }

    /// Returns an iterator over URL items only.
    pub fn urls(&self) -> impl Iterator<Item = &ParsedItem> {
        self.items
            .iter()
            .filter(|item| item.input_type == InputType::Url)
    }

    /// Returns an iterator over DOI items only.
    pub fn dois(&self) -> impl Iterator<Item = &ParsedItem> {
        self.items
            .iter()
            .filter(|item| item.input_type == InputType::Doi)
    }

    /// Returns an iterator over reference items only.
    pub fn references(&self) -> impl Iterator<Item = &ParsedItem> {
        self.items
            .iter()
            .filter(|item| item.input_type == InputType::Reference)
    }

    /// Returns an iterator over BibTeX items only.
    pub fn bibtex(&self) -> impl Iterator<Item = &ParsedItem> {
        self.items
            .iter()
            .filter(|item| item.input_type == InputType::BibTex)
    }

    /// Returns per-type counts for parsed items.
    #[must_use]
    pub fn type_counts(&self) -> ParseTypeCounts {
        let mut counts = ParseTypeCounts::default();
        for item in &self.items {
            match item.input_type {
                InputType::Url => counts.urls += 1,
                InputType::Doi => counts.dois += 1,
                InputType::Reference => counts.references += 1,
                InputType::BibTex => counts.bibtex += 1,
                InputType::Unknown => counts.unknown += 1,
            }
        }
        counts
    }

    /// Returns the number of parsed items of a specific type.
    #[must_use]
    pub fn count_by_type(&self, input_type: InputType) -> usize {
        self.items
            .iter()
            .filter(|item| item.input_type == input_type)
            .count()
    }
}

impl fmt::Display for ParseResult {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Parsed {} items ({} skipped)",
            self.items.len(),
            self.skipped.len()
        )
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn test_input_type_display() {
        assert_eq!(InputType::Url.to_string(), "URL");
        assert_eq!(InputType::Doi.to_string(), "DOI");
        assert_eq!(InputType::Reference.to_string(), "Reference");
        assert_eq!(InputType::BibTex.to_string(), "BibTeX");
        assert_eq!(InputType::Unknown.to_string(), "Unknown");
    }

    #[test]
    fn test_input_type_queue_source_type_mapping() {
        assert_eq!(InputType::Url.queue_source_type(), "direct_url");
        assert_eq!(InputType::Doi.queue_source_type(), "doi");
        assert_eq!(InputType::Reference.queue_source_type(), "reference");
        assert_eq!(InputType::BibTex.queue_source_type(), "bibtex");
        assert_eq!(InputType::Unknown.queue_source_type(), "reference");
    }

    #[test]
    fn test_parsed_item_url() {
        let item = ParsedItem::url("http://example.com", "http://example.com/");
        assert_eq!(item.raw, "http://example.com");
        assert_eq!(item.input_type, InputType::Url);
        assert_eq!(item.value, "http://example.com/");
    }

    #[test]
    fn test_parsed_item_display() {
        let item = ParsedItem::url("http://example.com", "http://example.com/");
        assert_eq!(item.to_string(), "[URL] http://example.com/");
    }

    #[test]
    fn test_parse_result_new() {
        let result = ParseResult::new();
        assert!(result.is_empty());
        assert_eq!(result.len(), 0);
        assert_eq!(result.skipped_count(), 0);
    }

    #[test]
    fn test_parse_result_add_item() {
        let mut result = ParseResult::new();
        result.add_item(ParsedItem::url("http://a.com", "http://a.com/"));
        result.add_item(ParsedItem::url("http://b.com", "http://b.com/"));

        assert!(!result.is_empty());
        assert_eq!(result.len(), 2);
    }

    #[test]
    fn test_parse_result_add_skipped() {
        let mut result = ParseResult::new();
        result.add_skipped("some text");
        result.add_skipped("more text");

        assert_eq!(result.skipped_count(), 2);
        assert!(result.skipped.contains(&"some text".to_string()));
    }

    #[test]
    fn test_parse_result_urls_iterator() {
        let mut result = ParseResult::new();
        result.add_item(ParsedItem::url("http://a.com", "http://a.com/"));
        result.add_item(ParsedItem::new("not-a-url", InputType::Unknown, ""));
        result.add_item(ParsedItem::url("http://b.com", "http://b.com/"));

        let urls: Vec<_> = result.urls().collect();
        assert_eq!(urls.len(), 2);
        assert_eq!(urls[0].value, "http://a.com/");
        assert_eq!(urls[1].value, "http://b.com/");
    }

    #[test]
    fn test_parsed_item_doi() {
        let item = ParsedItem::doi("DOI: 10.1234/test", "10.1234/test");
        assert_eq!(item.raw, "DOI: 10.1234/test");
        assert_eq!(item.input_type, InputType::Doi);
        assert_eq!(item.value, "10.1234/test");
    }

    #[test]
    fn test_parsed_item_doi_display() {
        let item = ParsedItem::doi("10.1234/test", "10.1234/test");
        assert_eq!(item.to_string(), "[DOI] 10.1234/test");
    }

    #[test]
    fn test_parsed_item_reference() {
        let item = ParsedItem::reference(
            "Smith, J. (2024). Paper Title. Journal.",
            "Smith, J. (2024). Paper Title. Journal.",
        );
        assert_eq!(item.input_type, InputType::Reference);
        assert_eq!(item.raw, "Smith, J. (2024). Paper Title. Journal.");
        assert_eq!(item.value, "Smith, J. (2024). Paper Title. Journal.");
    }

    #[test]
    fn test_parse_result_dois_iterator() {
        let mut result = ParseResult::new();
        result.add_item(ParsedItem::url("http://a.com", "http://a.com/"));
        result.add_item(ParsedItem::doi("10.1234/a", "10.1234/a"));
        result.add_item(ParsedItem::url("http://b.com", "http://b.com/"));
        result.add_item(ParsedItem::doi("10.5678/b", "10.5678/b"));

        let dois: Vec<_> = result.dois().collect();
        assert_eq!(dois.len(), 2);
        assert_eq!(dois[0].value, "10.1234/a");
        assert_eq!(dois[1].value, "10.5678/b");
    }

    #[test]
    fn test_parse_result_references_iterator() {
        let mut result = ParseResult::new();
        result.add_item(ParsedItem::url("http://a.com", "http://a.com/"));
        result.add_item(ParsedItem::doi("10.1234/a", "10.1234/a"));
        result.add_item(ParsedItem::reference(
            "Smith, J. (2024). Title. Journal.",
            "Smith, J. (2024). Title. Journal.",
        ));

        let references: Vec<_> = result.references().collect();
        assert_eq!(references.len(), 1);
        assert_eq!(references[0].input_type, InputType::Reference);
    }

    #[test]
    fn test_parse_result_display() {
        let mut result = ParseResult::new();
        result.add_item(ParsedItem::url("http://a.com", "http://a.com/"));
        result.add_skipped("text");

        assert_eq!(result.to_string(), "Parsed 1 items (1 skipped)");
    }

    #[test]
    fn test_parsed_item_bibtex() {
        let item = ParsedItem::bibtex("@article{k,...}", "Smith, J. (2024) Title.");
        assert_eq!(item.input_type, InputType::BibTex);
    }

    #[test]
    fn test_parse_result_type_counts() {
        let mut result = ParseResult::new();
        result.add_item(ParsedItem::url("http://a.com", "http://a.com/"));
        result.add_item(ParsedItem::doi("10.1234/a", "10.1234/a"));
        result.add_item(ParsedItem::reference("ref", "ref"));
        result.add_item(ParsedItem::bibtex("@article{k,...}", "k"));

        let counts = result.type_counts();
        assert_eq!(counts.urls, 1);
        assert_eq!(counts.dois, 1);
        assert_eq!(counts.references, 1);
        assert_eq!(counts.bibtex, 1);
        assert_eq!(counts.total(), 4);
        assert_eq!(result.count_by_type(InputType::Url), 1);
        assert_eq!(result.count_by_type(InputType::Doi), 1);
        assert_eq!(result.count_by_type(InputType::Reference), 1);
        assert_eq!(result.count_by_type(InputType::BibTex), 1);
    }
}
