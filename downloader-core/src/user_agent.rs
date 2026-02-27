//! Shared User-Agent strings for download and resolver HTTP clients.
//!
//! Single source for project URL and UA format so download and resolver traffic
//! stay consistent and easy to update (good citizenship; RFC 9308).

/// Project URL for User-Agent identification (good citizenship; RFC 9308).
const PROJECT_UA_URL: &str = "https://github.com/nicksrandall/Downloader";

/// Default User-Agent for download requests (identifies the tool).
#[must_use]
pub(crate) fn default_download_user_agent() -> String {
    let version = env!("CARGO_PKG_VERSION");
    format!("downloader/{version} (academic-research-tool; +{PROJECT_UA_URL})")
}

/// Default User-Agent for resolver requests (single shared format; no per-resolver name in header).
#[must_use]
pub(crate) fn default_resolver_user_agent() -> String {
    let version = env!("CARGO_PKG_VERSION");
    format!("downloader/{version} (research-tool; +{PROJECT_UA_URL})")
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Both UAs must use the same project URL and crate version (shared format).
    /// The test uses this module's private PROJECT_UA_URL intentionally so the assertion
    /// stays in sync with the single source of truth.
    #[test]
    fn test_shared_format_consistency() {
        let download_ua = default_download_user_agent();
        let resolver_ua = default_resolver_user_agent();
        assert!(
            download_ua.contains(PROJECT_UA_URL),
            "download UA must contain project URL"
        );
        assert!(
            resolver_ua.contains(PROJECT_UA_URL),
            "resolver UA must contain project URL"
        );
        assert_eq!(
            env!("CARGO_PKG_VERSION"),
            download_ua
                .strip_prefix("downloader/")
                .and_then(|s| s.split(' ').next())
                .expect("download UA has version"),
            "download UA must contain crate version"
        );
        assert_eq!(
            env!("CARGO_PKG_VERSION"),
            resolver_ua
                .strip_prefix("downloader/")
                .and_then(|s| s.split(' ').next())
                .expect("resolver UA has version"),
            "resolver UA must contain crate version"
        );
    }

    #[test]
    fn test_ua_format_keywords() {
        let download_ua = default_download_user_agent();
        let resolver_ua = default_resolver_user_agent();
        assert!(
            download_ua.contains("academic-research-tool"),
            "download UA must identify as academic-research-tool: {download_ua}"
        );
        assert!(
            resolver_ua.contains("research-tool"),
            "resolver UA must identify as research-tool: {resolver_ua}"
        );
    }
}
