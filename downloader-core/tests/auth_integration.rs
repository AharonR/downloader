//! Integration tests for cookie file loading and domain matching.

use std::io::Cursor;
use std::sync::Arc;

use reqwest::cookie::CookieStore;
use wiremock::matchers::{header, method, path};
use wiremock::{Mock, ResponseTemplate};

use downloader_core::download::HttpClient;
use downloader_core::{load_cookies_into_jar, parse_netscape_cookies};
mod support;
use support::socket_guard::start_mock_server_or_skip;

/// Helper: parse cookies from a string and load into a jar.
fn jar_from_str(input: &str) -> Arc<reqwest::cookie::Jar> {
    let reader = Cursor::new(input.as_bytes());
    let result = parse_netscape_cookies(reader).expect("valid cookie input");
    load_cookies_into_jar(&result.cookies)
}

// ---- Integration test: cookies sent with matching domain (AC1, AC4) ----

#[tokio::test]
async fn test_cookie_jar_sends_cookie_to_matching_domain() {
    let Some(mock_server) = start_mock_server_or_skip().await else {
        return;
    };
    let temp_dir = tempfile::TempDir::new().unwrap();

    // Set up mock that requires the specific cookie value
    Mock::given(method("GET"))
        .and(path("/file.pdf"))
        .and(header("cookie", "session_id=test_value_123"))
        .respond_with(ResponseTemplate::new(200).set_body_bytes(b"pdf-content"))
        .expect(1)
        .mount(&mock_server)
        .await;

    // Parse mock server host:port for cookie domain
    let uri = mock_server.uri();
    let parsed = url::Url::parse(&uri).unwrap();
    let host = parsed.host_str().unwrap();

    // Create cookies matching the mock server domain
    let cookie_data = format!("{host}\tFALSE\t/\tFALSE\t0\tsession_id\ttest_value_123\n");
    let jar = jar_from_str(&cookie_data);

    let client = HttpClient::with_cookie_jar(jar);
    let url = format!("{}/file.pdf", mock_server.uri());
    let result = client.download_to_file(&url, temp_dir.path()).await;

    assert!(result.is_ok(), "download should succeed: {result:?}");
}

// ---- Integration test: cookies NOT sent to non-matching domain (AC4) ----

#[tokio::test]
async fn test_cookie_jar_does_not_leak_cookies_to_other_domains() {
    let Some(mock_server) = start_mock_server_or_skip().await else {
        return;
    };
    let temp_dir = tempfile::TempDir::new().unwrap();

    // Set up mock that should NOT receive a Cookie header
    Mock::given(method("GET"))
        .and(path("/file.pdf"))
        .respond_with(ResponseTemplate::new(200).set_body_bytes(b"pdf-content"))
        .expect(1)
        .mount(&mock_server)
        .await;

    // Cookies for a completely different domain
    let cookie_data = ".unrelated-domain.com\tTRUE\t/\tFALSE\t0\tsession\tsecret\n";
    let jar = jar_from_str(cookie_data);

    // Verify the jar does NOT return cookies for the mock server
    let mock_url = mock_server.uri().parse::<url::Url>().unwrap();
    assert!(
        jar.cookies(&mock_url).is_none(),
        "cookie jar should NOT have cookies for mock server domain"
    );

    let client = HttpClient::with_cookie_jar(jar);
    let url = format!("{}/file.pdf", mock_server.uri());
    let result = client.download_to_file(&url, temp_dir.path()).await;
    assert!(
        result.is_ok(),
        "download should succeed without cookies: {result:?}"
    );
}

// ---- Integration test: malformed cookie file produces error with line number (AC3) ----

#[test]
fn test_malformed_cookie_file_reports_line_numbers() {
    let input = "\
# Netscape HTTP Cookie File
.good.com\tTRUE\t/\tFALSE\t0\tname\tvalue
this line is totally wrong
.good2.com\tTRUE\t/\tFALSE\t0\tother\tval
also broken
";
    let reader = Cursor::new(input.as_bytes());
    let result = parse_netscape_cookies(reader).unwrap();

    assert_eq!(result.cookies.len(), 2, "should have 2 valid cookies");
    assert_eq!(result.warnings.len(), 2, "should have 2 warnings");

    // Check line numbers
    assert_eq!(result.warnings[0].0, 3, "first warning on line 3");
    assert_eq!(result.warnings[1].0, 5, "second warning on line 5");

    // Check error description
    assert!(
        result.warnings[0].1.contains("7 TAB-separated fields"),
        "should mention field count: {}",
        result.warnings[0].1
    );
}

// ---- Integration test: all-malformed file produces NoCookiesFound error (AC3, audit) ----

#[test]
fn test_all_malformed_file_returns_error() {
    let input = "bad line one\nanother bad line\n";
    let reader = Cursor::new(input.as_bytes());
    let result = parse_netscape_cookies(reader);

    assert!(result.is_err(), "should fail when all lines are malformed");
    let err = result.unwrap_err();
    let msg = err.to_string();
    assert!(
        msg.contains("no valid cookies found"),
        "error should mention no valid cookies: {msg}"
    );
}

// ---- Integration test: subdomain matching works (AC4) ----

#[test]
fn test_subdomain_cookie_matching() {
    let cookie_data = ".example.com\tTRUE\t/\tFALSE\t0\tauth\ttoken123\n";
    let jar = jar_from_str(cookie_data);

    // Should match subdomain
    let sub_url = "http://sub.example.com/page".parse::<url::Url>().unwrap();
    assert!(
        jar.cookies(&sub_url).is_some(),
        "cookie should match subdomain"
    );

    // Should match root domain
    let root_url = "http://example.com/page".parse::<url::Url>().unwrap();
    assert!(
        jar.cookies(&root_url).is_some(),
        "cookie should match root domain"
    );

    // Should NOT match different domain
    let other_url = "http://other.com/page".parse::<url::Url>().unwrap();
    assert!(
        jar.cookies(&other_url).is_none(),
        "cookie should NOT match different domain"
    );
}

// ---- Security test: cookie values never in debug output (AC5) ----

#[test]
fn test_cookie_line_debug_never_contains_value() {
    let cookie_data = ".example.com\tTRUE\t/\tFALSE\t0\tsession\tmy_super_secret_value\n";
    let reader = Cursor::new(cookie_data.as_bytes());
    let result = parse_netscape_cookies(reader).unwrap();

    for cookie in &result.cookies {
        let debug_output = format!("{cookie:?}");
        assert!(
            !debug_output.contains("my_super_secret_value"),
            "Debug output must NOT contain cookie value: {debug_output}"
        );
        assert!(
            debug_output.contains("[REDACTED]"),
            "Debug output should show [REDACTED]: {debug_output}"
        );
    }
}
