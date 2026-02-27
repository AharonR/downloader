//! Browser cookie capture parsing and validation helpers.
//!
//! Supports common browser extension export formats:
//! - Netscape HTTP Cookie File format
//! - JSON cookie exports (array or `{ "cookies": [...] }`)

use std::collections::HashSet;
use std::io::BufReader;
use std::time::{SystemTime, UNIX_EPOCH};

use serde::Deserialize;
use tracing::instrument;

use super::{CookieError, CookieLine, parse_netscape_cookies};

/// Cookie payload format detected during capture.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CapturedCookieFormat {
    /// Netscape HTTP Cookie File format.
    Netscape,
    /// JSON export format.
    Json,
}

/// Parsed and validated cookies captured from user input.
#[derive(Debug)]
pub struct CapturedCookies {
    /// Valid cookies after format parsing and validation.
    pub cookies: Vec<CookieLine>,
    /// Non-fatal warnings encountered while parsing/validating.
    pub warnings: Vec<String>,
    /// Input format that was parsed.
    pub format: CapturedCookieFormat,
}

/// Errors that can occur while parsing browser cookie capture input.
#[derive(Debug, thiserror::Error)]
pub enum CaptureError {
    /// Input was empty.
    #[error("cookie input is empty")]
    EmptyInput,
    /// Netscape-format parser failed.
    #[error(transparent)]
    Netscape(#[from] CookieError),
    /// JSON parser failed.
    #[error("invalid cookie JSON: {0}")]
    Json(#[from] serde_json::Error),
    /// No valid cookies remained after validation.
    #[error("no valid cookies found after validation")]
    NoValidCookies,
}

/// Parse and validate cookie capture input from either Netscape or JSON format.
///
/// # Errors
///
/// Returns [`CaptureError`] when input is empty, parsing fails, or all cookies
/// are invalid/expired.
#[instrument(level = "debug", skip(input))]
pub fn parse_captured_cookies(input: &str) -> Result<CapturedCookies, CaptureError> {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return Err(CaptureError::EmptyInput);
    }

    let (cookies, mut warnings, format) = if looks_like_json(trimmed) {
        let (cookies, warnings) = parse_json_cookies(trimmed)?;
        (cookies, warnings, CapturedCookieFormat::Json)
    } else {
        let result = parse_netscape_cookies(BufReader::new(trimmed.as_bytes()))?;
        let warnings = result
            .warnings
            .iter()
            .map(|(line, reason)| format!("line {line}: {reason}"))
            .collect::<Vec<_>>();
        (result.cookies, warnings, CapturedCookieFormat::Netscape)
    };

    let (valid_cookies, validation_warnings) = validate_cookies(cookies, unix_now());
    warnings.extend(validation_warnings);

    if valid_cookies.is_empty() {
        return Err(CaptureError::NoValidCookies);
    }

    Ok(CapturedCookies {
        cookies: valid_cookies,
        warnings,
        format,
    })
}

/// Counts unique cookie domains in the provided cookie list.
#[must_use]
#[instrument(level = "debug", skip(cookies))]
pub fn unique_domain_count(cookies: &[CookieLine]) -> usize {
    cookies
        .iter()
        .map(|cookie| cookie.domain.trim_start_matches('.').to_string())
        .collect::<HashSet<_>>()
        .len()
}

fn looks_like_json(input: &str) -> bool {
    input.starts_with('[') || input.starts_with('{')
}

fn validate_cookies(cookies: Vec<CookieLine>, now: u64) -> (Vec<CookieLine>, Vec<String>) {
    let mut valid = Vec::new();
    let mut warnings = Vec::new();

    for mut cookie in cookies {
        if cookie.domain.trim().is_empty() {
            warnings.push("skipped cookie with empty domain".to_string());
            continue;
        }
        if cookie.name.trim().is_empty() {
            warnings.push("skipped cookie with empty name".to_string());
            continue;
        }
        if cookie.value().is_empty() {
            warnings.push(format!(
                "skipped cookie '{}' for domain '{}' because value is empty",
                cookie.name, cookie.domain
            ));
            continue;
        }
        if cookie.path.trim().is_empty() {
            cookie.path = "/".to_string();
        }
        if cookie.expires > 0 && cookie.expires <= now {
            warnings.push(format!(
                "skipped expired cookie '{}' for domain '{}'",
                cookie.name, cookie.domain
            ));
            continue;
        }

        valid.push(cookie);
    }

    (valid, warnings)
}

fn unix_now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |duration| duration.as_secs())
}

fn parse_json_cookies(input: &str) -> Result<(Vec<CookieLine>, Vec<String>), CaptureError> {
    let payload: JsonCookiePayload = serde_json::from_str(input)?;
    let entries = match payload {
        JsonCookiePayload::Array(entries) => entries,
        JsonCookiePayload::Wrapped { cookies } => cookies,
    };

    let mut cookies = Vec::new();
    let mut warnings = Vec::new();

    for (index, entry) in entries.into_iter().enumerate() {
        match convert_json_cookie(entry) {
            Ok(cookie) => cookies.push(cookie),
            Err(reason) => warnings.push(format!("entry {}: {}", index + 1, reason)),
        }
    }

    Ok((cookies, warnings))
}

fn convert_json_cookie(entry: JsonCookieEntry) -> Result<CookieLine, String> {
    let mut domain = entry
        .domain
        .or(entry.host)
        .unwrap_or_default()
        .trim()
        .to_string();

    if domain.is_empty() {
        return Err("missing required field: domain".to_string());
    }

    if let Some(stripped) = domain.strip_prefix("http://") {
        domain = stripped.to_string();
    } else if let Some(stripped) = domain.strip_prefix("https://") {
        domain = stripped.to_string();
    }
    if let Some((host, _rest)) = domain.split_once('/') {
        domain = host.to_string();
    }

    let tailmatch = if let Some(host_only) = entry.host_only {
        !host_only
    } else {
        domain.starts_with('.')
    };

    if tailmatch && !domain.starts_with('.') {
        domain = format!(".{domain}");
    }
    if !tailmatch {
        domain = domain.trim_start_matches('.').to_string();
    }

    let mut path = entry.path.unwrap_or_else(|| "/".to_string());
    if path.trim().is_empty() {
        path = "/".to_string();
    } else if !path.starts_with('/') {
        path = format!("/{path}");
    }

    let name = entry.name.unwrap_or_default().trim().to_string();
    if name.is_empty() {
        return Err("missing required field: name".to_string());
    }

    let value = entry.value.unwrap_or_default();
    if value.is_empty() {
        return Err(format!(
            "cookie '{name}' for domain '{domain}' has empty value"
        ));
    }

    let expires = entry
        .expiration_date
        .or(entry.expires)
        .map_or(0, normalized_expiry);

    Ok(CookieLine::new(
        domain,
        tailmatch,
        path,
        entry.secure.unwrap_or(false),
        expires,
        name,
        value,
    ))
}

fn normalized_expiry(raw_expiry: f64) -> u64 {
    if !raw_expiry.is_finite() || raw_expiry <= 0.0 {
        return 0;
    }

    let floored = raw_expiry.floor();
    let integer_text = format!("{floored:.0}");
    // Overflow → treat as far-future (permanent cookie); only reachable with
    // expiry values exceeding u64::MAX (~year 584 billion).
    integer_text.parse::<u64>().unwrap_or(u64::MAX)
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum JsonCookiePayload {
    Array(Vec<JsonCookieEntry>),
    Wrapped { cookies: Vec<JsonCookieEntry> },
}

#[derive(Debug, Deserialize)]
struct JsonCookieEntry {
    domain: Option<String>,
    host: Option<String>,
    #[serde(rename = "hostOnly")]
    host_only: Option<bool>,
    path: Option<String>,
    secure: Option<bool>,
    name: Option<String>,
    value: Option<String>,
    #[serde(rename = "expirationDate")]
    expiration_date: Option<f64>,
    expires: Option<f64>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_captured_cookies_netscape_format_success() {
        let input = ".example.com\tTRUE\t/\tFALSE\t4102444800\tsession\tabc123";
        let parsed = parse_captured_cookies(input).unwrap();
        assert_eq!(parsed.format, CapturedCookieFormat::Netscape);
        assert_eq!(parsed.cookies.len(), 1);
        assert!(parsed.warnings.is_empty());
    }

    #[test]
    fn test_parse_captured_cookies_json_array_success() {
        let input = r#"
[
  {
    "domain": ".example.com",
    "name": "session",
    "value": "abc123",
    "path": "/",
    "secure": true,
    "expirationDate": 4102444800
  }
]
"#;
        let parsed = parse_captured_cookies(input).unwrap();
        assert_eq!(parsed.format, CapturedCookieFormat::Json);
        assert_eq!(parsed.cookies.len(), 1);
        assert_eq!(parsed.cookies[0].domain, ".example.com");
        assert_eq!(unique_domain_count(&parsed.cookies), 1);
    }

    #[test]
    fn test_parse_captured_cookies_json_wrapped_success() {
        let input = r#"
{
  "cookies": [
    {
      "domain": "example.com",
      "hostOnly": true,
      "name": "sid",
      "value": "xyz",
      "path": "/"
    }
  ]
}
"#;
        let parsed = parse_captured_cookies(input).unwrap();
        assert_eq!(parsed.cookies.len(), 1);
        assert!(!parsed.cookies[0].tailmatch);
        assert_eq!(parsed.cookies[0].domain, "example.com");
    }

    #[test]
    fn test_parse_captured_cookies_expired_cookie_filtered() {
        let input = ".example.com\tTRUE\t/\tFALSE\t1\tsession\texpired";
        let result = parse_captured_cookies(input);
        assert!(matches!(result, Err(CaptureError::NoValidCookies)));
    }

    #[test]
    fn test_parse_captured_cookies_json_invalid_entries_warn_and_keep_valid() {
        let input = r#"
[
  {
    "domain": ".ok.com",
    "name": "ok",
    "value": "value",
    "path": "/",
    "expirationDate": 4102444800
  },
  {
    "domain": ".bad.com",
    "name": "",
    "value": "missing-name",
    "path": "/"
  }
]
"#;
        let parsed = parse_captured_cookies(input).unwrap();
        assert_eq!(parsed.cookies.len(), 1);
        assert!(!parsed.warnings.is_empty());
    }

    #[test]
    fn test_parse_captured_cookies_empty_input_fails() {
        let result = parse_captured_cookies("   ");
        assert!(matches!(result, Err(CaptureError::EmptyInput)));
    }

    #[test]
    fn test_validate_cookies_filters_expired_with_explicit_time() {
        let cookies = vec![
            CookieLine::new(
                ".example.com".to_string(),
                true,
                "/".to_string(),
                false,
                1000,
                "expired".to_string(),
                "val".to_string(),
            ),
            CookieLine::new(
                ".example.com".to_string(),
                true,
                "/".to_string(),
                false,
                2000,
                "valid".to_string(),
                "val".to_string(),
            ),
        ];

        let (valid, warnings) = validate_cookies(cookies, 1500);
        assert_eq!(valid.len(), 1, "only the non-expired cookie should remain");
        assert_eq!(valid[0].name, "valid");
        assert_eq!(warnings.len(), 1, "one expiry warning expected");
        assert!(
            warnings[0].contains("expired"),
            "warning should mention expiry"
        );
    }
}
