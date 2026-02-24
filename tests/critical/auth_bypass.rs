//! Phase 3 (P0): Cookie injection, session hijacking.
//! Assert cookies not sent to wrong domain/path.

use std::io::Cursor;
use std::sync::Arc;

use downloader_core::download::HttpClient;
use downloader_core::{load_cookies_into_jar, parse_netscape_cookies};
use reqwest::cookie::CookieStore;
use tempfile::TempDir;
use wiremock::matchers::{header, method, path};
use wiremock::{Mock, ResponseTemplate};

use crate::support::socket_guard::{socket_skip_return, start_mock_server_or_skip};

fn jar_from_str(input: &str) -> Arc<reqwest::cookie::Jar> {
    let reader = Cursor::new(input.as_bytes());
    let result = parse_netscape_cookies(reader).expect("valid cookie input");
    load_cookies_into_jar(&result.cookies)
}

#[tokio::test]
async fn p0_cookie_not_sent_to_wrong_domain() {
    let Some(mock_server) = start_mock_server_or_skip().await else {
        return socket_skip_return();
    };

    Mock::given(method("GET"))
        .and(path("/file.pdf"))
        .respond_with(ResponseTemplate::new(200).set_body_bytes(b"pdf"))
        .expect(1)
        .mount(&mock_server)
        .await;

    let cookie_data = ".evil.com\tTRUE\t/\tFALSE\t0\tsession\tstolen\n";
    let jar = jar_from_str(cookie_data);

    let mock_url = mock_server.uri().parse::<url::Url>().unwrap();
    assert!(
        jar.cookies(&mock_url).is_none(),
        "jar must not send cookies for wrong domain"
    );

    let client = HttpClient::with_cookie_jar(jar);
    let url = format!("{}/file.pdf", mock_server.uri());
    let temp = TempDir::new().expect("temp dir");
    let result = client.download_to_file(&url, temp.path()).await;
    assert!(
        result.is_ok(),
        "download without leaking cookie: {:?}",
        result
    );
}

#[tokio::test]
async fn p0_cookie_sent_only_to_matching_domain() {
    let Some(mock_server) = start_mock_server_or_skip().await else {
        return socket_skip_return();
    };

    let uri = mock_server.uri();
    let parsed = url::Url::parse(&uri).unwrap();
    let host = parsed.host_str().unwrap();

    Mock::given(method("GET"))
        .and(path("/file.pdf"))
        .and(header("cookie", "sid=secret123"))
        .respond_with(ResponseTemplate::new(200).set_body_bytes(b"pdf"))
        .expect(1)
        .mount(&mock_server)
        .await;

    let cookie_data = format!("{host}\tFALSE\t/\tFALSE\t0\tsid\tsecret123\n");
    let jar = jar_from_str(&cookie_data);

    let client = HttpClient::with_cookie_jar(jar);
    let url = format!("{}/file.pdf", mock_server.uri());
    let temp = TempDir::new().expect("temp dir");
    let result = client.download_to_file(&url, temp.path()).await;
    assert!(
        result.is_ok(),
        "download with cookie for matching domain: {:?}",
        result
    );
}
