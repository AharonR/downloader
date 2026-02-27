//! Netscape cookie file parser and reqwest jar loader.
//!
//! Parses the Netscape HTTP cookie file format (7 TAB-separated fields per line)
//! and loads cookies into a `reqwest::cookie::Jar` for use with the HTTP client.

use std::fmt;
use std::io::BufRead;
use std::sync::Arc;

use reqwest::cookie::Jar;
use tracing::{debug, instrument, warn};

/// A single parsed cookie from a Netscape-format cookie file.
///
/// The value field is intentionally redacted in Debug output to prevent
/// accidental logging of sensitive cookie data.
#[derive(Clone)]
pub struct CookieLine {
    /// The domain the cookie belongs to (e.g., `.example.com`).
    pub domain: String,
    /// Whether subdomains should match.
    pub tailmatch: bool,
    /// The URL path scope for the cookie.
    pub path: String,
    /// Whether the cookie should only be sent over HTTPS.
    pub secure: bool,
    /// Unix timestamp for expiry (0 = session cookie).
    pub expires: u64,
    /// Cookie name.
    pub name: String,
    /// Cookie value (sensitive — never log).
    value: String,
}

impl CookieLine {
    /// Creates a new cookie entry.
    #[must_use]
    pub fn new(
        domain: String,
        tailmatch: bool,
        path: String,
        secure: bool,
        expires: u64,
        name: String,
        value: String,
    ) -> Self {
        Self {
            domain,
            tailmatch,
            path,
            secure,
            expires,
            name,
            value,
        }
    }

    /// Returns the cookie value.
    ///
    /// Cookie values are sensitive — avoid logging the return value.
    #[must_use]
    pub fn value(&self) -> &str {
        &self.value
    }
}

// Custom Debug impl that redacts the cookie value.
impl fmt::Debug for CookieLine {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("CookieLine")
            .field("domain", &self.domain)
            .field("tailmatch", &self.tailmatch)
            .field("path", &self.path)
            .field("secure", &self.secure)
            .field("expires", &self.expires)
            .field("name", &self.name)
            .field("value", &"[REDACTED]")
            .finish()
    }
}

/// Errors that can occur while parsing a cookie file.
#[derive(Debug, thiserror::Error)]
pub enum CookieError {
    /// A line in the cookie file has an invalid format.
    #[error("line {line_number}: {reason} (got: {content})")]
    InvalidLine {
        /// 1-based line number in the cookie file.
        line_number: usize,
        /// The offending line content (truncated, with value redacted).
        content: String,
        /// Description of what was wrong.
        reason: String,
    },

    /// I/O error reading the cookie file.
    #[error("failed to read cookie file: {0}")]
    Io(#[from] std::io::Error),

    /// No valid cookies found in a non-empty file.
    #[error("no valid cookies found in file ({malformed_count} lines failed to parse)")]
    NoCookiesFound {
        /// Number of malformed lines encountered.
        malformed_count: usize,
    },
}

/// Result of parsing a cookie file, including successfully parsed cookies
/// and any warnings about malformed lines.
#[derive(Debug)]
pub struct ParseResult {
    /// Successfully parsed cookies.
    pub cookies: Vec<CookieLine>,
    /// Warnings for malformed lines (line number and reason).
    pub warnings: Vec<(usize, String)>,
}

/// Parses a Netscape-format cookie file from a buffered reader.
///
/// Each non-comment, non-blank line must contain exactly 7 TAB-separated fields:
/// `domain`, `tailmatch`, `path`, `secure`, `expires`, `name`, `value`.
///
/// Lines starting with `#` and blank lines are skipped. The optional
/// `# Netscape HTTP Cookie File` header is accepted.
///
/// # Errors
///
/// Returns [`CookieError::Io`] on read failure, or
/// [`CookieError::NoCookiesFound`] when a non-empty file yields zero valid cookies.
/// Individual malformed lines are collected as warnings (partial success).
#[instrument(level = "debug", skip(reader))]
pub fn parse_netscape_cookies(reader: impl BufRead) -> Result<ParseResult, CookieError> {
    let mut cookies = Vec::new();
    let mut warnings = Vec::new();
    let mut non_blank_lines = 0;

    for (idx, line_result) in reader.lines().enumerate() {
        let line_number = idx + 1;
        let line = line_result?;
        // Handle CRLF: strip trailing \r
        let line = line.trim_end();

        // Skip blank lines
        if line.is_empty() {
            continue;
        }

        // Skip comment lines (including the optional Netscape header)
        if line.starts_with('#') {
            continue;
        }

        non_blank_lines += 1;

        match parse_cookie_line(line, line_number) {
            Ok(cookie) => {
                debug!(
                    line = line_number,
                    domain = %cookie.domain,
                    name = %cookie.name,
                    "parsed cookie"
                );
                cookies.push(cookie);
            }
            Err(e) => {
                warn!(line = line_number, reason = %e, "skipping malformed cookie line");
                warnings.push((line_number, e.to_string()));
            }
        }
    }

    // If we had non-blank data lines but no cookies parsed, that's an error
    if cookies.is_empty() && non_blank_lines > 0 {
        return Err(CookieError::NoCookiesFound {
            malformed_count: warnings.len(),
        });
    }

    Ok(ParseResult { cookies, warnings })
}

/// Parses a single cookie line into a `CookieLine`.
fn parse_cookie_line(line: &str, line_number: usize) -> Result<CookieLine, CookieError> {
    let fields: Vec<&str> = line.split('\t').collect();

    if fields.len() != 7 {
        return Err(CookieError::InvalidLine {
            line_number,
            content: redact_line_for_error(line),
            reason: format!("expected 7 TAB-separated fields, found {}", fields.len()),
        });
    }

    let domain = fields[0].to_string();
    let tailmatch = parse_bool_field(fields[1], "tailmatch", line_number, line)?;
    let path = fields[2].to_string();
    let secure = parse_bool_field(fields[3], "secure", line_number, line)?;

    let expires = fields[4]
        .parse::<u64>()
        .map_err(|_| CookieError::InvalidLine {
            line_number,
            content: redact_line_for_error(line),
            reason: format!(
                "expires field must be a non-negative integer, got '{}'",
                fields[4]
            ),
        })?;

    let name = fields[5].to_string();
    let value = fields[6].to_string();

    if domain.is_empty() {
        return Err(CookieError::InvalidLine {
            line_number,
            content: redact_line_for_error(line),
            reason: "domain field is empty".to_string(),
        });
    }

    if name.is_empty() {
        return Err(CookieError::InvalidLine {
            line_number,
            content: redact_line_for_error(line),
            reason: "cookie name field is empty".to_string(),
        });
    }

    Ok(CookieLine::new(
        domain, tailmatch, path, secure, expires, name, value,
    ))
}

/// Parses a `TRUE`/`FALSE` string field.
fn parse_bool_field(
    value: &str,
    field_name: &str,
    line_number: usize,
    line: &str,
) -> Result<bool, CookieError> {
    match value {
        "TRUE" => Ok(true),
        "FALSE" => Ok(false),
        _ => Err(CookieError::InvalidLine {
            line_number,
            content: redact_line_for_error(line),
            reason: format!("{field_name} field must be TRUE or FALSE, got '{value}'"),
        }),
    }
}

/// Redacts cookie value (7th field) from a line for safe error messages.
fn redact_line_for_error(line: &str) -> String {
    let fields: Vec<&str> = line.split('\t').collect();
    if fields.len() >= 7 {
        let mut redacted = fields[..6].join("\t");
        redacted.push_str("\t[REDACTED]");
        redacted
    } else {
        // Not enough fields to identify value — show as-is (no value present)
        line.to_string()
    }
}

/// Loads parsed cookies into a `reqwest::cookie::Jar`.
///
/// Each `CookieLine` is converted to a `Set-Cookie` header string and added
/// to the jar with the appropriate origin URL for domain matching.
///
/// # Returns
///
/// An `Arc<Jar>` suitable for passing to `reqwest::ClientBuilder::cookie_provider()`.
#[instrument(level = "debug", skip(cookies))]
pub fn load_cookies_into_jar(cookies: &[CookieLine]) -> Arc<Jar> {
    let jar = Arc::new(Jar::default());

    for cookie in cookies {
        let set_cookie = build_set_cookie_string(cookie);
        let origin_url = build_origin_url(cookie);

        if let Ok(url) = origin_url.parse::<url::Url>() {
            jar.add_cookie_str(&set_cookie, &url);
            debug!(
                domain = %cookie.domain,
                name = %cookie.name,
                "loaded cookie into jar"
            );
        } else {
            warn!(
                domain = %cookie.domain,
                name = %cookie.name,
                "skipping cookie with unparseable domain"
            );
        }
    }

    jar
}

/// Builds a `Set-Cookie` header string from a `CookieLine`.
fn build_set_cookie_string(cookie: &CookieLine) -> String {
    let mut parts = vec![format!("{}={}", cookie.name, cookie.value())];

    // Domain attribute
    parts.push(format!("Domain={}", cookie.domain));

    // Path attribute
    parts.push(format!("Path={}", cookie.path));

    // Secure flag
    if cookie.secure {
        parts.push("Secure".to_string());
    }

    // Expires (0 = session cookie, omit Expires)
    if cookie.expires > 0 {
        // Convert Unix timestamp to HTTP-date format
        if let Some(expires_str) = unix_to_http_date(cookie.expires) {
            parts.push(format!("Expires={expires_str}"));
        } else {
            warn!(
                domain = %cookie.domain,
                name = %cookie.name,
                expires = cookie.expires,
                "cookie expiry timestamp overflows SystemTime; treating as session cookie"
            );
        }
    }

    parts.join("; ")
}

/// Builds the origin URL for `Jar::add_cookie_str` from a `CookieLine`.
///
/// Uses `https://` for secure cookies and `http://` for non-secure.
/// Strips the leading dot from the domain for the URL.
fn build_origin_url(cookie: &CookieLine) -> String {
    let scheme = if cookie.secure { "https" } else { "http" };
    let domain = cookie.domain.strip_prefix('.').unwrap_or(&cookie.domain);
    format!("{scheme}://{domain}{}", cookie.path)
}

/// Converts a Unix timestamp to an HTTP-date string (RFC 7231).
fn unix_to_http_date(timestamp: u64) -> Option<String> {
    use std::time::{Duration, UNIX_EPOCH};

    let time = UNIX_EPOCH.checked_add(Duration::from_secs(timestamp))?;

    // Use httpdate crate (already a dependency via reqwest)
    Some(httpdate::fmt_http_date(time))
}

#[cfg(test)]
mod tests {
    use super::*;
    use reqwest::cookie::CookieStore;
    use std::io::Cursor;

    fn cursor(s: &str) -> Cursor<&[u8]> {
        Cursor::new(s.as_bytes())
    }

    // ---- Task 1 tests: Parsing ----

    #[test]
    fn test_parse_netscape_cookies_valid_file() {
        let input = "\
# Netscape HTTP Cookie File
.example.com\tTRUE\t/\tFALSE\t0\tsession\tabc123
.other.com\tTRUE\t/path\tTRUE\t1700000000\ttoken\txyz789
";
        let result = parse_netscape_cookies(cursor(input)).unwrap();
        assert_eq!(result.cookies.len(), 2);
        assert!(result.warnings.is_empty());

        assert_eq!(result.cookies[0].domain, ".example.com");
        assert!(result.cookies[0].tailmatch);
        assert_eq!(result.cookies[0].path, "/");
        assert!(!result.cookies[0].secure);
        assert_eq!(result.cookies[0].expires, 0);
        assert_eq!(result.cookies[0].name, "session");
        assert_eq!(result.cookies[0].value(), "abc123");

        assert_eq!(result.cookies[1].domain, ".other.com");
        assert!(result.cookies[1].secure);
        assert_eq!(result.cookies[1].expires, 1_700_000_000);
    }

    #[test]
    fn test_parse_netscape_cookies_comment_and_blank_lines() {
        let input = "\
# Netscape HTTP Cookie File
# This is a comment

.example.com\tTRUE\t/\tFALSE\t0\tname\tvalue

# Another comment
";
        let result = parse_netscape_cookies(cursor(input)).unwrap();
        assert_eq!(result.cookies.len(), 1);
        assert!(result.warnings.is_empty());
    }

    #[test]
    fn test_parse_netscape_cookies_no_header() {
        let input = ".example.com\tTRUE\t/\tFALSE\t0\tname\tvalue\n";
        let result = parse_netscape_cookies(cursor(input)).unwrap();
        assert_eq!(result.cookies.len(), 1);
    }

    #[test]
    fn test_parse_netscape_cookies_malformed_lines_with_line_numbers() {
        let input = "\
# Header
.good.com\tTRUE\t/\tFALSE\t0\tname\tvalue
bad line without tabs
.also-good.com\tTRUE\t/\tFALSE\t0\tother\tval
";
        let result = parse_netscape_cookies(cursor(input)).unwrap();
        assert_eq!(result.cookies.len(), 2, "should parse 2 valid cookies");
        assert_eq!(result.warnings.len(), 1, "should have 1 warning");
        assert_eq!(result.warnings[0].0, 3, "warning should be for line 3");
        assert!(
            result.warnings[0]
                .1
                .contains("expected 7 TAB-separated fields"),
            "warning should mention field count"
        );
    }

    #[test]
    fn test_parse_netscape_cookies_empty_file() {
        let input = "";
        let result = parse_netscape_cookies(cursor(input)).unwrap();
        assert!(result.cookies.is_empty());
        assert!(result.warnings.is_empty());
    }

    #[test]
    fn test_parse_netscape_cookies_only_comments() {
        let input = "# Netscape HTTP Cookie File\n# comment\n";
        let result = parse_netscape_cookies(cursor(input)).unwrap();
        assert!(result.cookies.is_empty());
    }

    #[test]
    fn test_parse_netscape_cookies_all_malformed_returns_error() {
        let input = "\
bad line one
another bad line
";
        let result = parse_netscape_cookies(cursor(input));
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(
            matches!(err, CookieError::NoCookiesFound { malformed_count: 2 }),
            "expected NoCookiesFound with 2 malformed, got: {err}"
        );
    }

    #[test]
    fn test_parse_netscape_cookies_invalid_bool_field() {
        let input = ".example.com\tYES\t/\tFALSE\t0\tname\tvalue\n";
        let result = parse_netscape_cookies(cursor(input));
        // Should be NoCookiesFound since the only data line is malformed
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_netscape_cookies_invalid_expires() {
        let input = ".example.com\tTRUE\t/\tFALSE\tnot-a-number\tname\tvalue\n";
        let result = parse_netscape_cookies(cursor(input));
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_netscape_cookies_empty_domain_rejected() {
        let input = "\tTRUE\t/\tFALSE\t0\tname\tvalue\n";
        let result = parse_netscape_cookies(cursor(input));
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_netscape_cookies_empty_name_rejected() {
        let input = ".example.com\tTRUE\t/\tFALSE\t0\t\tvalue\n";
        let result = parse_netscape_cookies(cursor(input));
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_netscape_cookies_crlf_line_endings() {
        let input = "# Header\r\n.example.com\tTRUE\t/\tFALSE\t0\tname\tvalue\r\n";
        let result = parse_netscape_cookies(cursor(input)).unwrap();
        assert_eq!(result.cookies.len(), 1);
        // Verify no trailing \r in value
        assert_eq!(result.cookies[0].value(), "value");
        assert!(!result.cookies[0].value().ends_with('\r'));
    }

    #[test]
    fn test_parse_netscape_cookies_large_expires() {
        let input = ".example.com\tTRUE\t/\tFALSE\t9999999999\tname\tvalue\n";
        let result = parse_netscape_cookies(cursor(input)).unwrap();
        assert_eq!(result.cookies[0].expires, 9_999_999_999);
    }

    #[test]
    fn test_parse_netscape_cookies_partial_success_with_warnings() {
        let input = "\
.good.com\tTRUE\t/\tFALSE\t0\tname1\tval1
bad
.good2.com\tTRUE\t/\tFALSE\t0\tname2\tval2
also bad
";
        let result = parse_netscape_cookies(cursor(input)).unwrap();
        assert_eq!(result.cookies.len(), 2, "2 valid cookies");
        assert_eq!(result.warnings.len(), 2, "2 warnings");
    }

    // ---- CookieLine Debug redaction ----

    #[test]
    fn test_cookie_line_debug_redacts_value() {
        let cookie = CookieLine {
            domain: ".example.com".to_string(),
            tailmatch: true,
            path: "/".to_string(),
            secure: false,
            expires: 0,
            name: "session".to_string(),
            value: "super_secret_token".to_string(),
        };
        let debug_str = format!("{cookie:?}");
        assert!(
            debug_str.contains("[REDACTED]"),
            "Debug output should contain [REDACTED]"
        );
        assert!(
            !debug_str.contains("super_secret_token"),
            "Debug output must NOT contain the actual value"
        );
    }

    // ---- Error message redaction ----

    #[test]
    fn test_redact_line_for_error_hides_value() {
        let line = ".example.com\tTRUE\t/\tFALSE\t0\tname\tsecret_value";
        let redacted = redact_line_for_error(line);
        assert!(
            !redacted.contains("secret_value"),
            "Redacted line must not contain the value"
        );
        assert!(redacted.contains("[REDACTED]"));
        assert!(redacted.contains("name"));
    }

    // ---- Task 2 tests: Jar loading ----

    #[test]
    fn test_load_cookies_into_jar_basic() {
        let cookies = vec![CookieLine {
            domain: ".example.com".to_string(),
            tailmatch: true,
            path: "/".to_string(),
            secure: false,
            expires: 0,
            name: "session".to_string(),
            value: "abc123".to_string(),
        }];

        let jar = load_cookies_into_jar(&cookies);

        // Verify cookie is retrievable for matching domain
        let url = "http://example.com/page".parse::<url::Url>().unwrap();
        let cookie_header = jar.cookies(&url);
        assert!(
            cookie_header.is_some(),
            "jar should return cookies for matching domain"
        );
        let header_val = cookie_header.unwrap();
        assert!(
            header_val.to_str().unwrap().contains("session=abc123"),
            "cookie header should contain the cookie"
        );
    }

    #[test]
    fn test_load_cookies_into_jar_subdomain_matching() {
        let cookies = vec![CookieLine {
            domain: ".example.com".to_string(),
            tailmatch: true,
            path: "/".to_string(),
            secure: false,
            expires: 0,
            name: "session".to_string(),
            value: "abc123".to_string(),
        }];

        let jar = load_cookies_into_jar(&cookies);

        // Should match subdomain
        let url = "http://sub.example.com/page".parse::<url::Url>().unwrap();
        let cookie_header = jar.cookies(&url);
        assert!(
            cookie_header.is_some(),
            "jar should return cookies for subdomain"
        );
    }

    #[test]
    fn test_load_cookies_into_jar_no_cross_domain() {
        let cookies = vec![CookieLine {
            domain: ".example.com".to_string(),
            tailmatch: true,
            path: "/".to_string(),
            secure: false,
            expires: 0,
            name: "session".to_string(),
            value: "abc123".to_string(),
        }];

        let jar = load_cookies_into_jar(&cookies);

        // Should NOT match different domain
        let url = "http://other.com/page".parse::<url::Url>().unwrap();
        let cookie_header = jar.cookies(&url);
        assert!(
            cookie_header.is_none(),
            "jar should NOT return cookies for unrelated domain"
        );
    }

    #[test]
    fn test_load_cookies_into_jar_secure_flag() {
        let cookies = vec![CookieLine {
            domain: ".secure.com".to_string(),
            tailmatch: true,
            path: "/".to_string(),
            secure: true,
            expires: 0,
            name: "token".to_string(),
            value: "secret".to_string(),
        }];

        let jar = load_cookies_into_jar(&cookies);

        // Should be available for HTTPS
        let https_url = "https://secure.com/page".parse::<url::Url>().unwrap();
        let cookie_header = jar.cookies(&https_url);
        assert!(
            cookie_header.is_some(),
            "secure cookie should be available for HTTPS"
        );
    }

    #[test]
    fn test_load_cookies_into_jar_empty_list() {
        let cookies: Vec<CookieLine> = vec![];
        let jar = load_cookies_into_jar(&cookies);
        let url = "http://example.com/".parse::<url::Url>().unwrap();
        assert!(jar.cookies(&url).is_none());
    }

    #[test]
    fn test_build_set_cookie_string_session_cookie() {
        let cookie = CookieLine {
            domain: ".example.com".to_string(),
            tailmatch: true,
            path: "/".to_string(),
            secure: false,
            expires: 0,
            name: "name".to_string(),
            value: "val".to_string(),
        };
        let s = build_set_cookie_string(&cookie);
        assert!(s.contains("name=val"));
        assert!(s.contains("Domain=.example.com"));
        assert!(s.contains("Path=/"));
        assert!(!s.contains("Secure"));
        assert!(!s.contains("Expires"));
    }

    #[test]
    fn test_build_set_cookie_string_with_expiry_and_secure() {
        let cookie = CookieLine {
            domain: ".example.com".to_string(),
            tailmatch: true,
            path: "/".to_string(),
            secure: true,
            expires: 1_700_000_000,
            name: "token".to_string(),
            value: "xyz".to_string(),
        };
        let s = build_set_cookie_string(&cookie);
        assert!(s.contains("Secure"), "should contain Secure flag");
        assert!(s.contains("Expires="), "should contain Expires");
    }

    #[test]
    fn test_build_origin_url_non_secure() {
        let cookie = CookieLine {
            domain: ".example.com".to_string(),
            tailmatch: true,
            path: "/".to_string(),
            secure: false,
            expires: 0,
            name: "n".to_string(),
            value: "v".to_string(),
        };
        assert_eq!(build_origin_url(&cookie), "http://example.com/");
    }

    #[test]
    fn test_build_origin_url_secure() {
        let cookie = CookieLine {
            domain: ".secure.com".to_string(),
            tailmatch: true,
            path: "/api".to_string(),
            secure: true,
            expires: 0,
            name: "n".to_string(),
            value: "v".to_string(),
        };
        assert_eq!(build_origin_url(&cookie), "https://secure.com/api");
    }

    #[test]
    fn test_build_origin_url_no_leading_dot() {
        let cookie = CookieLine {
            domain: "exact.com".to_string(),
            tailmatch: false,
            path: "/".to_string(),
            secure: false,
            expires: 0,
            name: "n".to_string(),
            value: "v".to_string(),
        };
        assert_eq!(build_origin_url(&cookie), "http://exact.com/");
    }
}
