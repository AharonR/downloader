//! Minimal robots.txt fetching and parsing for polite crawling.
//!
//! Supports `User-agent: *` and `Disallow: /path` rules. Caches per origin with 24h TTL.

use std::time::{Duration, SystemTime};

use dashmap::DashMap;
use tracing::{debug, instrument};

use super::client::HttpClient;

const ROBOTS_TTL: Duration = Duration::from_secs(24 * 3600);

/// Result of checking a URL against robots.txt.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RobotsDecision {
    /// URL is allowed.
    Allowed,
    /// URL is disallowed by robots.txt.
    Disallowed,
}

/// Minimal robots.txt checker with per-origin cache.
#[derive(Debug, Default)]
pub struct RobotsCache {
    cache: DashMap<String, CachedRobots>,
}

#[derive(Debug)]
struct CachedRobots {
    disallowed_prefixes: Vec<String>,
    fetched_at: SystemTime,
}

impl RobotsCache {
    #[must_use]
    pub fn new() -> Self {
        Self {
            cache: DashMap::new(),
        }
    }

    /// Returns whether the URL is allowed by robots.txt for the given origin.
    /// Fetches and caches robots.txt per origin; uses cache if still valid (24h TTL).
    ///
    /// # Errors
    ///
    /// Returns `RobotsError` if the URL is invalid, fetch fails, or cache state is inconsistent.
    #[instrument(skip(self, client), fields(origin = %origin))]
    pub async fn check_allowed(
        &self,
        url: &str,
        origin: &str,
        client: &HttpClient,
    ) -> Result<RobotsDecision, RobotsError> {
        let path = path_from_url(url)?;
        let now = SystemTime::now();
        // map_or(true, ...) is the stable replacement for Option::is_none_or (not yet stabilized); allow avoids clippy suggesting is_none_or.
        #[allow(clippy::unnecessary_map_or)]
        let need_fetch = self.cache.get(origin).map_or(true, |c| {
            now.duration_since(c.fetched_at).unwrap_or(Duration::MAX) > ROBOTS_TTL
        });

        if need_fetch {
            let body = fetch_robots_txt(origin, client).await?;
            let disallowed = parse_disallow_rules(&body);
            self.cache.insert(
                origin.to_string(),
                CachedRobots {
                    disallowed_prefixes: disallowed,
                    fetched_at: now,
                },
            );
        }

        let entry = self.cache.get(origin).ok_or(RobotsError::CacheMissing)?;
        let allowed = !entry
            .disallowed_prefixes
            .iter()
            .any(|prefix| path.starts_with(prefix.as_str()));
        Ok(if allowed {
            RobotsDecision::Allowed
        } else {
            debug!(path = %path, origin = %origin, "robots.txt disallows path");
            RobotsDecision::Disallowed
        })
    }
}

fn path_from_url(url: &str) -> Result<String, RobotsError> {
    let parsed = url::Url::parse(url).map_err(|_| RobotsError::InvalidUrl)?;
    let path = parsed.path();
    if path.is_empty() {
        Ok("/".to_string())
    } else {
        Ok(path.to_string())
    }
}

async fn fetch_robots_txt(origin: &str, client: &HttpClient) -> Result<String, RobotsError> {
    let robots_url = format!(
        "{}robots.txt",
        origin.trim_end_matches('/').to_string() + "/"
    );
    let response = client
        .inner()
        .get(&robots_url)
        .send()
        .await
        .map_err(RobotsError::Fetch)?;
    let status = response.status();
    if !status.is_success() {
        if status.as_u16() == 404 {
            return Ok(String::new());
        }
        return Err(RobotsError::Status(robots_url, status.as_u16()));
    }
    let body = response.text().await.map_err(RobotsError::Body)?;
    Ok(body)
}

/// Parses robots.txt body for User-agent: * and Disallow rules.
fn parse_disallow_rules(body: &str) -> Vec<String> {
    let mut in_star = false;
    let mut disallowed = Vec::new();
    for line in body.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        if let Some(rest) = line.strip_prefix("User-agent:") {
            let rest = rest.trim();
            in_star = rest == "*" || rest.is_empty();
            continue;
        }
        if in_star {
            if let Some(suffix) = line.strip_prefix("Disallow:") {
                let path = suffix.trim();
                if path.is_empty() {
                    continue;
                }
                let prefix = normalize_disallow_path(path);
                if !prefix.is_empty() && !disallowed.contains(&prefix) {
                    disallowed.push(prefix);
                }
            }
        }
    }
    disallowed.sort_by_key(|b| std::cmp::Reverse(b.len()));
    disallowed
}

fn normalize_disallow_path(path: &str) -> String {
    let s = path.trim();
    if s.is_empty() {
        return String::new();
    }
    let mut s = s.to_string();
    if !s.starts_with('/') {
        s.insert(0, '/');
    }
    s
}

/// Errors from robots.txt checking.
#[derive(Debug, thiserror::Error)]
pub enum RobotsError {
    #[error("invalid URL")]
    InvalidUrl,
    #[error("failed to fetch robots.txt: {0}")]
    Fetch(#[source] reqwest::Error),
    #[error("robots.txt returned status {1} for {0}")]
    Status(String, u16),
    #[error("failed to read robots.txt body: {0}")]
    Body(#[source] reqwest::Error),
    #[error("cache entry missing after fetch (internal)")]
    CacheMissing,
}

/// Builds the origin string (scheme + host) from a URL for robots.txt lookup.
#[must_use]
pub fn origin_for_robots(url: &str) -> Option<String> {
    let parsed = url::Url::parse(url).ok()?;
    let scheme = parsed.scheme();
    let host = parsed.host_str()?;
    let port = parsed.port();
    let origin = if let Some(p) = port {
        format!("{scheme}://{host}:{p}")
    } else {
        format!("{scheme}://{host}")
    };
    Some(origin)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_disallow_empty() {
        let r = parse_disallow_rules("");
        assert!(r.is_empty());
    }

    #[test]
    fn test_parse_disallow_star() {
        let r = parse_disallow_rules("User-agent: *\nDisallow: /api/\nDisallow: /private/\n");
        assert!(r.contains(&"/api/".to_string()));
        assert!(r.contains(&"/private/".to_string()));
    }

    #[test]
    fn test_parse_disallow_no_star_ignored() {
        let r = parse_disallow_rules("User-agent: Googlebot\nDisallow: /nobot/\n");
        assert!(r.is_empty());
    }

    #[test]
    fn test_normalize_disallow_path() {
        assert_eq!(normalize_disallow_path("/foo"), "/foo");
        assert_eq!(normalize_disallow_path("foo"), "/foo");
    }

    #[test]
    fn test_origin_for_robots() {
        assert_eq!(
            origin_for_robots("https://example.com/path"),
            Some("https://example.com".to_string())
        );
        assert_eq!(
            origin_for_robots("http://localhost:8080/file"),
            Some("http://localhost:8080".to_string())
        );
    }

    // --- parse_disallow_rules edge cases ---

    #[test]
    fn test_parse_disallow_with_comments_ignored() {
        let body = "# This is a comment\nUser-agent: *\nDisallow: /secret/\n";
        let r = parse_disallow_rules(body);
        assert!(r.contains(&"/secret/".to_string()));
    }

    #[test]
    fn test_parse_disallow_empty_disallow_line_skipped() {
        // Empty Disallow: means "allow all" — must not be added to list
        let body = "User-agent: *\nDisallow: \n";
        let r = parse_disallow_rules(body);
        assert!(r.is_empty());
    }

    #[test]
    fn test_parse_disallow_deduplicates_paths() {
        let body = "User-agent: *\nDisallow: /api/\nDisallow: /api/\n";
        let r = parse_disallow_rules(body);
        assert_eq!(r.iter().filter(|p| *p == "/api/").count(), 1);
    }

    #[test]
    fn test_parse_disallow_sorted_longest_first() {
        let body = "User-agent: *\nDisallow: /a/\nDisallow: /longer/path/\nDisallow: /medium/\n";
        let r = parse_disallow_rules(body);
        assert_eq!(r.len(), 3);
        // Longest prefix first so most-specific rules win during matching
        assert!(r[0].len() >= r[1].len());
        assert!(r[1].len() >= r[2].len());
    }

    #[test]
    fn test_parse_disallow_non_star_agent_ignored() {
        // Rules for named agents must not affect wildcard matching
        let body = "User-agent: Googlebot\nDisallow: /nobot/\nUser-agent: *\nDisallow: /all/\n";
        let r = parse_disallow_rules(body);
        assert!(!r.contains(&"/nobot/".to_string()));
        assert!(r.contains(&"/all/".to_string()));
    }

    // --- normalize_disallow_path edge cases ---

    #[test]
    fn test_normalize_disallow_path_empty_returns_empty() {
        assert_eq!(normalize_disallow_path(""), "");
    }

    #[test]
    fn test_normalize_disallow_path_whitespace_only_returns_empty() {
        assert_eq!(normalize_disallow_path("   "), "");
    }

    #[test]
    fn test_normalize_disallow_path_already_has_slash() {
        assert_eq!(normalize_disallow_path("/foo/bar"), "/foo/bar");
    }

    #[test]
    fn test_normalize_disallow_path_adds_leading_slash() {
        assert_eq!(normalize_disallow_path("foo/bar"), "/foo/bar");
    }

    // --- origin_for_robots edge cases ---

    #[test]
    fn test_origin_for_robots_invalid_url_returns_none() {
        assert_eq!(origin_for_robots("not_a_valid_url"), None);
    }

    #[test]
    fn test_origin_for_robots_strips_path_query_and_fragment() {
        assert_eq!(
            origin_for_robots("https://example.com/path?query=1#frag"),
            Some("https://example.com".to_string())
        );
    }

    #[test]
    fn test_origin_for_robots_includes_non_default_port() {
        assert_eq!(
            origin_for_robots("https://example.com:9443/path"),
            Some("https://example.com:9443".to_string())
        );
    }
}
