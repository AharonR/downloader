//! Types representing parsed input items and results.

use std::fmt;

/// Type of input detected.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InputType {
    /// Direct HTTP/HTTPS URL
    Url,
    /// DOI identifier (future - Epic 2)
    Doi,
    /// Reference string (future - Epic 2)
    Reference,
    /// BibTeX entry (future - Epic 2)
    BibTex,
    /// Could not determine type
    Unknown,
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
    fn test_parse_result_display() {
        let mut result = ParseResult::new();
        result.add_item(ParsedItem::url("http://a.com", "http://a.com/"));
        result.add_skipped("text");

        assert_eq!(result.to_string(), "Parsed 1 items (1 skipped)");
    }
}
