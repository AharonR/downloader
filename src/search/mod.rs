//! Search ranking and match classification for download history.
//!
//! Ranks candidates by exact, substring, and fuzzy match quality.

use std::cmp::Ordering;

use downloader_core::DownloadSearchCandidate;

/// Fuzzy match threshold (0.0–1.0); matches at or above this are included.
pub const SEARCH_FUZZY_THRESHOLD: f64 = 0.86;

/// How a query matched a candidate field.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum SearchMatchKind {
    Fuzzy,
    Substring,
    Exact,
}

/// A search candidate with match metadata for ranking and display.
#[derive(Debug, Clone)]
pub struct RankedSearchResult {
    pub candidate: DownloadSearchCandidate,
    pub match_kind: SearchMatchKind,
    pub similarity: f64,
    pub matched_field: &'static str,
}

/// Ranks candidates by best match across title, authors, and DOI.
pub fn rank_search_candidates(
    query: &str,
    candidates: Vec<DownloadSearchCandidate>,
) -> Vec<RankedSearchResult> {
    let query_norm = normalize_search_text(query);
    if query_norm.is_empty() {
        return Vec::new();
    }

    let mut ranked: Vec<RankedSearchResult> = candidates
        .into_iter()
        .filter_map(|candidate| {
            let mut best: Option<(SearchMatchKind, f64, &'static str)> = None;
            for (field_name, field_value) in [
                ("title", candidate.title.as_deref()),
                ("authors", candidate.authors.as_deref()),
                ("doi", candidate.doi.as_deref()),
            ] {
                let Some((kind, similarity)) = classify_search_match(&query_norm, field_value)
                else {
                    continue;
                };
                let is_better = best
                    .as_ref()
                    .map(|(best_kind, best_similarity, _)| {
                        (kind, similarity) > (*best_kind, *best_similarity)
                    })
                    .unwrap_or(true);
                if is_better {
                    best = Some((kind, similarity, field_name));
                }
            }

            best.map(
                |(match_kind, similarity, matched_field)| RankedSearchResult {
                    candidate,
                    match_kind,
                    similarity,
                    matched_field,
                },
            )
        })
        .collect();

    ranked.sort_by(compare_search_results);
    ranked
}

/// Compares two ranked results for sort order (best first).
pub fn compare_search_results(left: &RankedSearchResult, right: &RankedSearchResult) -> Ordering {
    right
        .match_kind
        .cmp(&left.match_kind)
        .then_with(|| {
            right
                .similarity
                .partial_cmp(&left.similarity)
                .unwrap_or(Ordering::Equal)
        })
        .then_with(|| right.candidate.started_at.cmp(&left.candidate.started_at))
        .then_with(|| right.candidate.id.cmp(&left.candidate.id))
}

/// Classifies how the normalized query matches the field value, if at all.
pub fn classify_search_match(
    query_norm: &str,
    field_value: Option<&str>,
) -> Option<(SearchMatchKind, f64)> {
    let value = field_value?;
    let normalized = normalize_search_text(value);
    if normalized.is_empty() {
        return None;
    }

    if normalized == query_norm {
        return Some((SearchMatchKind::Exact, 1.0));
    }

    if normalized.contains(query_norm) {
        let similarity =
            (query_norm.chars().count() as f64 / normalized.chars().count() as f64).clamp(0.0, 1.0);
        return Some((SearchMatchKind::Substring, similarity));
    }

    let similarity = fuzzy_similarity(query_norm, &normalized);
    if similarity >= SEARCH_FUZZY_THRESHOLD {
        return Some((SearchMatchKind::Fuzzy, similarity));
    }

    None
}

/// Returns best fuzzy similarity between query and value (or any token in value).
pub fn fuzzy_similarity(query_norm: &str, normalized_value: &str) -> f64 {
    let mut best = strsim::normalized_levenshtein(query_norm, normalized_value);

    for token in normalized_value.split(|ch: char| !ch.is_alphanumeric()) {
        if token.chars().count() < 3 {
            continue;
        }
        best = best.max(strsim::normalized_levenshtein(query_norm, token));
    }

    best
}

/// Normalizes text for search: collapse whitespace and lowercase.
pub fn normalize_search_text(value: &str) -> String {
    value
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_lowercase()
}

#[cfg(test)]
mod tests {
    use super::*;
    use downloader_core::DownloadSearchCandidate;

    #[test]
    fn test_normalize_search_text() {
        assert_eq!(normalize_search_text("  Foo   Bar  "), "foo bar");
        assert_eq!(normalize_search_text("UPPER"), "upper");
        assert!(normalize_search_text("  ").is_empty());
    }

    #[test]
    fn test_classify_search_match_exact() {
        let (kind, sim) = classify_search_match("hello", Some("hello")).unwrap();
        assert_eq!(kind, SearchMatchKind::Exact);
        assert!((sim - 1.0).abs() < 1e-9);
    }

    #[test]
    fn test_classify_search_match_substring() {
        let (kind, _sim) = classify_search_match("att", Some("Attention Is All You Need")).unwrap();
        assert_eq!(kind, SearchMatchKind::Substring);
    }

    #[test]
    fn test_classify_search_match_none_for_empty_value() {
        assert!(classify_search_match("query", Some("")).is_none());
    }

    #[test]
    fn test_compare_search_results_orders_exact_before_fuzzy() {
        let base = DownloadSearchCandidate {
            id: 1,
            url: String::new(),
            status_str: String::new(),
            title: None,
            authors: None,
            doi: None,
            started_at: String::new(),
            file_path: None,
        };
        let exact = RankedSearchResult {
            candidate: base.clone(),
            match_kind: SearchMatchKind::Exact,
            similarity: 1.0,
            matched_field: "title",
        };
        let fuzzy = RankedSearchResult {
            candidate: base,
            match_kind: SearchMatchKind::Fuzzy,
            similarity: 0.9,
            matched_field: "title",
        };
        assert_eq!(
            compare_search_results(&exact, &fuzzy),
            Ordering::Less,
            "exact should sort before fuzzy (less = higher priority)"
        );
        assert_eq!(compare_search_results(&fuzzy, &exact), Ordering::Greater);
    }
}
