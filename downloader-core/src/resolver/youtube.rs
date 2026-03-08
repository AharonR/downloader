//! `YouTube` resolver — fetches oEmbed metadata and transcript for `YouTube` URLs.
//!
//! The [`YouTubeResolver`] handles `youtube.com/watch?v=ID` and `youtu.be/ID` URLs.
//! It fetches structured metadata via the public oEmbed API (no auth required) and,
//! if available, a transcript via the public timedtext API.
//!
//! # v1 behaviour
//!
//! Returns the oEmbed JSON URL as the download target.  The download engine saves it
//! as a `.json` file.  Metadata keys populated: `title`, `authors`, `source_url`,
//! `youtube_video_id`.
//!
//! # v2 behaviour (transcript)
//!
//! After oEmbed succeeds, attempts to fetch the timedtext XML for `lang=en`.  If the
//! API returns a non-empty 200 response the transcript URL is returned as the download
//! target instead, with an additional `transcript_lang` metadata key.  Falls back to
//! v1 oEmbed JSON when the transcript is unavailable.

use std::collections::HashMap;

use async_trait::async_trait;
use reqwest::Client;
use serde::Deserialize;
use tracing::{debug, warn};

use crate::parser::InputType;

use super::http_client::{build_resolver_http_client, standard_user_agent};
use super::{ResolveContext, ResolveError, ResolveStep, ResolvedUrl, Resolver, ResolverPriority};

/// Default `YouTube` oEmbed base URL.
const DEFAULT_OEMBED_BASE: &str = "https://www.youtube.com/oembed";
/// Default `YouTube` timedtext (transcript) base URL.
const DEFAULT_TIMEDTEXT_BASE: &str = "https://www.youtube.com/api/timedtext";

// ==================== oEmbed Response Type ====================

/// Subset of the `YouTube` oEmbed JSON response we care about.
#[derive(Debug, Deserialize)]
struct OEmbedResponse {
    title: Option<String>,
    author_name: Option<String>,
}

// ==================== YouTubeResolver ====================

/// Resolves `YouTube` watch URLs to oEmbed metadata (and optionally transcripts).
pub struct YouTubeResolver {
    client: Client,
    oembed_base: String,
    timedtext_base: String,
}

impl YouTubeResolver {
    /// Creates a new `YouTubeResolver` with default production endpoints.
    ///
    /// # Errors
    ///
    /// Returns [`ResolveError`] if HTTP client construction fails.
    pub fn new() -> Result<Self, ResolveError> {
        Self::build(
            DEFAULT_OEMBED_BASE.to_string(),
            DEFAULT_TIMEDTEXT_BASE.to_string(),
        )
    }

    /// Creates a `YouTubeResolver` with custom base URLs (for testing with wiremock).
    ///
    /// # Errors
    ///
    /// Returns [`ResolveError`] if HTTP client construction fails.
    pub fn with_base_urls(
        oembed_base: impl Into<String>,
        timedtext_base: impl Into<String>,
    ) -> Result<Self, ResolveError> {
        Self::build(oembed_base.into(), timedtext_base.into())
    }

    fn build(oembed_base: String, timedtext_base: String) -> Result<Self, ResolveError> {
        let user_agent = standard_user_agent("youtube");
        let client = build_resolver_http_client("youtube", user_agent, None)?;
        Ok(Self {
            client,
            oembed_base,
            timedtext_base,
        })
    }

    /// Extracts the `YouTube` video ID from a supported URL.
    ///
    /// Accepts:
    /// - `https://www.youtube.com/watch?v=ID`
    /// - `https://youtube.com/watch?v=ID`
    /// - `https://youtu.be/ID`
    /// - `https://www.youtube.com/shorts/ID`
    /// - `https://youtube.com/shorts/ID`
    fn extract_video_id(input: &str) -> Option<String> {
        let url = reqwest::Url::parse(input).ok()?;
        let host = url.host_str()?;

        if host == "youtu.be" {
            // Path is "/<id>"
            let id = url.path().trim_start_matches('/');
            if id.is_empty() {
                return None;
            }
            return Some(id.to_string());
        }

        if host == "www.youtube.com" || host == "youtube.com" {
            // Standard watch URL
            if url.path() == "/watch" {
                return url
                    .query_pairs()
                    .find(|(k, _)| k == "v")
                    .map(|(_, v)| v.into_owned());
            }

            // YouTube Shorts URL: /shorts/<id>
            if let Some(id) = url.path().strip_prefix("/shorts/") {
                let id = id.trim_start_matches('/');
                if !id.is_empty() {
                    return Some(id.to_string());
                }
            }
        }

        None
    }
}

impl std::fmt::Debug for YouTubeResolver {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("YouTubeResolver")
            .field("oembed_base", &self.oembed_base)
            .field("timedtext_base", &self.timedtext_base)
            .finish_non_exhaustive()
    }
}

#[async_trait]
impl Resolver for YouTubeResolver {
    fn name(&self) -> &'static str {
        "youtube"
    }

    fn priority(&self) -> ResolverPriority {
        ResolverPriority::Specialized
    }

    fn can_handle(&self, input: &str, input_type: InputType) -> bool {
        if input_type != InputType::Url {
            return false;
        }
        Self::extract_video_id(input).is_some()
    }

    #[tracing::instrument(skip(self, _ctx), fields(resolver = "youtube", url = %input))]
    async fn resolve(
        &self,
        input: &str,
        _ctx: &ResolveContext,
    ) -> Result<ResolveStep, ResolveError> {
        let Some(video_id) = Self::extract_video_id(input) else {
            return Ok(ResolveStep::Failed(ResolveError::resolution_failed(
                input,
                "YouTube resolver: failed to extract video ID from URL",
            )));
        };

        // Build the oEmbed request URL
        let encoded_url = urlencoding::encode(input);
        let oembed_url = format!("{}?url={}&format=json", self.oembed_base, encoded_url);

        debug!(oembed_url = %oembed_url, "Calling YouTube oEmbed API");

        let response = match self.client.get(&oembed_url).send().await {
            Ok(resp) => resp,
            Err(e) => {
                warn!(error = %e, "YouTube oEmbed request failed");
                return Ok(ResolveStep::Failed(ResolveError::resolution_failed(
                    input,
                    "Cannot reach YouTube oEmbed API. Check your network connection.",
                )));
            }
        };

        let status = response.status();
        if !status.is_success() {
            let reason = if status.as_u16() == 404 {
                "YouTube oEmbed: video not found or private".to_string()
            } else {
                format!("YouTube oEmbed API returned HTTP {}", status.as_u16())
            };
            debug!(status = status.as_u16(), %reason, "YouTube oEmbed error");
            return Ok(ResolveStep::Failed(ResolveError::resolution_failed(
                input, &reason,
            )));
        }

        let oembed: OEmbedResponse = match response.json().await {
            Ok(parsed) => parsed,
            Err(e) => {
                warn!(error = %e, "Failed to parse YouTube oEmbed response JSON");
                return Ok(ResolveStep::Failed(ResolveError::resolution_failed(
                    input,
                    "Unexpected YouTube oEmbed response format",
                )));
            }
        };

        // Build metadata from oEmbed fields
        let mut metadata: HashMap<String, String> = HashMap::new();
        if let Some(title) = oembed.title {
            metadata.insert("title".to_string(), title);
        }
        if let Some(author) = oembed.author_name {
            metadata.insert("authors".to_string(), author);
        }
        metadata.insert("source_url".to_string(), input.to_string());
        metadata.insert("youtube_video_id".to_string(), video_id.clone());

        // v2: attempt transcript fetch
        let transcript_url = format!("{}?v={}&lang=en", self.timedtext_base, video_id);
        debug!(transcript_url = %transcript_url, "Attempting YouTube transcript fetch");

        match self.client.get(&transcript_url).send().await {
            Ok(resp) if resp.status().is_success() => {
                // Only use the transcript if the body is non-empty
                match resp.bytes().await {
                    Ok(body) if !body.is_empty() => {
                        debug!("Transcript available; using timedtext URL as download target");
                        metadata.insert("transcript_lang".to_string(), "en".to_string());
                        return Ok(ResolveStep::Url(ResolvedUrl::with_metadata(
                            transcript_url,
                            metadata,
                        )));
                    }
                    _ => {
                        debug!("Transcript body empty or unreadable; falling back to oEmbed JSON");
                    }
                }
            }
            Ok(resp) => {
                debug!(
                    status = resp.status().as_u16(),
                    "Transcript unavailable; falling back to oEmbed JSON"
                );
            }
            Err(e) => {
                debug!(error = %e, "Transcript fetch failed; falling back to oEmbed JSON");
            }
        }

        // v1 fallback: return oEmbed JSON URL
        Ok(ResolveStep::Url(ResolvedUrl::with_metadata(
            oembed_url, metadata,
        )))
    }
}

// ==================== Tests ====================

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use crate::test_support::socket_guard::start_mock_server_or_skip;
    use wiremock::matchers::{method, path, query_param};
    use wiremock::{Mock, ResponseTemplate};

    // ==================== can_handle tests ====================

    #[test]
    fn test_can_handle_youtube_watch_url() {
        let resolver = YouTubeResolver::new().unwrap();
        assert!(resolver.can_handle(
            "https://www.youtube.com/watch?v=NFLkNsBDd1Y",
            InputType::Url
        ));
    }

    #[test]
    fn test_can_handle_youtube_watch_url_no_www() {
        let resolver = YouTubeResolver::new().unwrap();
        assert!(resolver.can_handle("https://youtube.com/watch?v=NFLkNsBDd1Y", InputType::Url));
    }

    #[test]
    fn test_can_handle_youtu_be_url() {
        let resolver = YouTubeResolver::new().unwrap();
        assert!(resolver.can_handle("https://youtu.be/NFLkNsBDd1Y", InputType::Url));
    }

    #[test]
    fn test_cannot_handle_non_youtube_url() {
        let resolver = YouTubeResolver::new().unwrap();
        assert!(!resolver.can_handle("https://example.com/video", InputType::Url));
    }

    #[test]
    fn test_cannot_handle_doi() {
        let resolver = YouTubeResolver::new().unwrap();
        assert!(!resolver.can_handle("10.1234/test", InputType::Doi));
    }

    #[test]
    fn test_cannot_handle_youtube_homepage() {
        let resolver = YouTubeResolver::new().unwrap();
        assert!(!resolver.can_handle("https://www.youtube.com/", InputType::Url));
    }

    #[test]
    fn test_cannot_handle_youtu_be_no_id() {
        let resolver = YouTubeResolver::new().unwrap();
        assert!(!resolver.can_handle("https://youtu.be/", InputType::Url));
    }

    // ==================== extract_video_id tests ====================

    #[test]
    fn test_extract_video_id_watch_url() {
        let id = YouTubeResolver::extract_video_id("https://www.youtube.com/watch?v=NFLkNsBDd1Y");
        assert_eq!(id, Some("NFLkNsBDd1Y".to_string()));
    }

    #[test]
    fn test_extract_video_id_youtu_be() {
        let id = YouTubeResolver::extract_video_id("https://youtu.be/NFLkNsBDd1Y");
        assert_eq!(id, Some("NFLkNsBDd1Y".to_string()));
    }

    #[test]
    fn test_extract_video_id_extra_query_params() {
        let id = YouTubeResolver::extract_video_id(
            "https://www.youtube.com/watch?v=abc123&t=30&list=PLxyz",
        );
        assert_eq!(id, Some("abc123".to_string()));
    }

    #[test]
    fn test_extract_video_id_homepage_returns_none() {
        let id = YouTubeResolver::extract_video_id("https://www.youtube.com/");
        assert!(id.is_none());
    }

    #[test]
    fn test_extract_video_id_shorts_url() {
        let id = YouTubeResolver::extract_video_id("https://www.youtube.com/shorts/dQw4w9WgXcQ");
        assert_eq!(id, Some("dQw4w9WgXcQ".to_string()));
    }

    #[test]
    fn test_extract_video_id_shorts_url_no_www() {
        let id = YouTubeResolver::extract_video_id("https://youtube.com/shorts/dQw4w9WgXcQ");
        assert_eq!(id, Some("dQw4w9WgXcQ".to_string()));
    }

    #[test]
    fn test_extract_video_id_shorts_empty_id_returns_none() {
        let id = YouTubeResolver::extract_video_id("https://www.youtube.com/shorts/");
        assert!(id.is_none());
    }

    #[test]
    fn test_can_handle_youtube_shorts_url() {
        let resolver = YouTubeResolver::new().unwrap();
        assert!(resolver.can_handle("https://www.youtube.com/shorts/dQw4w9WgXcQ", InputType::Url));
    }

    #[test]
    fn test_can_handle_youtube_shorts_url_no_www() {
        let resolver = YouTubeResolver::new().unwrap();
        assert!(resolver.can_handle("https://youtube.com/shorts/dQw4w9WgXcQ", InputType::Url));
    }

    // ==================== resolve integration tests (wiremock) ====================

    fn oembed_json() -> serde_json::Value {
        serde_json::json!({
            "title": "Test Video Title",
            "author_name": "Test Channel",
            "author_url": "https://www.youtube.com/@TestChannel",
            "thumbnail_url": "https://i.ytimg.com/vi/abc123/hqdefault.jpg",
            "type": "video",
            "version": "1.0",
            "provider_name": "YouTube",
            "provider_url": "https://www.youtube.com/",
            "width": 480,
            "height": 270
        })
    }

    #[tokio::test]
    async fn test_resolve_oembed_success_populates_metadata() {
        let Some(mock_server) = start_mock_server_or_skip().await else {
            return;
        };

        // oEmbed returns 200 with JSON
        Mock::given(method("GET"))
            .and(path("/oembed"))
            .respond_with(ResponseTemplate::new(200).set_body_json(oembed_json()))
            .mount(&mock_server)
            .await;

        // Transcript returns 404 so we fall back to oEmbed
        Mock::given(method("GET"))
            .and(path("/timedtext"))
            .respond_with(ResponseTemplate::new(404))
            .mount(&mock_server)
            .await;

        let resolver = YouTubeResolver::with_base_urls(
            format!("{}/oembed", mock_server.uri()),
            format!("{}/timedtext", mock_server.uri()),
        )
        .unwrap();
        let ctx = ResolveContext::default();
        let result = resolver
            .resolve("https://www.youtube.com/watch?v=abc123", &ctx)
            .await
            .unwrap();

        match result {
            ResolveStep::Url(resolved) => {
                assert!(
                    resolved.url.contains("/oembed"),
                    "URL should be oEmbed endpoint, got: {}",
                    resolved.url
                );
                assert_eq!(resolved.metadata.get("title").unwrap(), "Test Video Title");
                assert_eq!(resolved.metadata.get("authors").unwrap(), "Test Channel");
                assert_eq!(resolved.metadata.get("youtube_video_id").unwrap(), "abc123");
                assert_eq!(
                    resolved.metadata.get("source_url").unwrap(),
                    "https://www.youtube.com/watch?v=abc123"
                );
                assert!(
                    !resolved.metadata.contains_key("transcript_lang"),
                    "No transcript_lang when transcript unavailable"
                );
            }
            other => panic!("Expected ResolveStep::Url, got: {other:?}"),
        }
    }

    #[tokio::test]
    async fn test_resolve_oembed_404_returns_failed() {
        let Some(mock_server) = start_mock_server_or_skip().await else {
            return;
        };

        Mock::given(method("GET"))
            .and(path("/oembed"))
            .respond_with(ResponseTemplate::new(404))
            .mount(&mock_server)
            .await;

        let resolver = YouTubeResolver::with_base_urls(
            format!("{}/oembed", mock_server.uri()),
            format!("{}/timedtext", mock_server.uri()),
        )
        .unwrap();
        let ctx = ResolveContext::default();
        let result = resolver
            .resolve("https://www.youtube.com/watch?v=abc123", &ctx)
            .await
            .unwrap();

        match result {
            ResolveStep::Failed(err) => {
                let msg = err.to_string();
                assert!(
                    msg.contains("not found") || msg.contains("private"),
                    "Error should mention not found/private: {msg}"
                );
            }
            other => panic!("Expected ResolveStep::Failed, got: {other:?}"),
        }
    }

    #[tokio::test]
    async fn test_resolve_transcript_used_when_available() {
        let Some(mock_server) = start_mock_server_or_skip().await else {
            return;
        };

        Mock::given(method("GET"))
            .and(path("/oembed"))
            .respond_with(ResponseTemplate::new(200).set_body_json(oembed_json()))
            .mount(&mock_server)
            .await;

        // Transcript returns non-empty XML
        Mock::given(method("GET"))
            .and(path("/timedtext"))
            .and(query_param("lang", "en"))
            .respond_with(ResponseTemplate::new(200).set_body_string(
                r#"<?xml version="1.0"?><transcript><text start="0.0">Hello</text></transcript>"#,
            ))
            .mount(&mock_server)
            .await;

        let resolver = YouTubeResolver::with_base_urls(
            format!("{}/oembed", mock_server.uri()),
            format!("{}/timedtext", mock_server.uri()),
        )
        .unwrap();
        let ctx = ResolveContext::default();
        let result = resolver
            .resolve("https://www.youtube.com/watch?v=abc123", &ctx)
            .await
            .unwrap();

        match result {
            ResolveStep::Url(resolved) => {
                assert!(
                    resolved.url.contains("/timedtext"),
                    "URL should be timedtext endpoint, got: {}",
                    resolved.url
                );
                assert_eq!(resolved.metadata.get("transcript_lang").unwrap(), "en");
                assert_eq!(resolved.metadata.get("youtube_video_id").unwrap(), "abc123");
            }
            other => panic!("Expected ResolveStep::Url (transcript), got: {other:?}"),
        }
    }

    #[tokio::test]
    async fn test_resolve_malformed_oembed_json_returns_failed() {
        let Some(mock_server) = start_mock_server_or_skip().await else {
            return;
        };

        Mock::given(method("GET"))
            .and(path("/oembed"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_string("not json at all")
                    .insert_header("content-type", "application/json"),
            )
            .mount(&mock_server)
            .await;

        let resolver = YouTubeResolver::with_base_urls(
            format!("{}/oembed", mock_server.uri()),
            format!("{}/timedtext", mock_server.uri()),
        )
        .unwrap();
        let ctx = ResolveContext::default();
        let result = resolver
            .resolve("https://www.youtube.com/watch?v=abc123", &ctx)
            .await
            .unwrap();

        assert!(
            matches!(result, ResolveStep::Failed(_)),
            "Malformed JSON should return Failed"
        );
    }

    // ==================== Resolver trait tests ====================

    #[test]
    fn test_name() {
        let resolver = YouTubeResolver::new().unwrap();
        assert_eq!(resolver.name(), "youtube");
    }

    #[test]
    fn test_priority_is_specialized() {
        let resolver = YouTubeResolver::new().unwrap();
        assert_eq!(resolver.priority(), ResolverPriority::Specialized);
    }
}
