//! E2E test: binary returns exit code 1 when some downloads succeed and some fail (partial success),
//! per Epic 7 / help text contract.

#![allow(deprecated)]

mod support;
use support::socket_guard::{socket_skip_return, start_mock_server_or_skip};

use assert_cmd::Command;
use tempfile::TempDir;
use wiremock::matchers::{method, path};
use wiremock::{Mock, ResponseTemplate};

#[tokio::test]
async fn test_binary_exit_code_partial_success_is_one() {
    let Some(mock_server) = start_mock_server_or_skip().await else {
        return socket_skip_return();
    };

    Mock::given(method("GET"))
        .and(path("/ok"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_bytes(b"PDF")
                .insert_header("Content-Type", "application/pdf"),
        )
        .mount(&mock_server)
        .await;

    Mock::given(method("GET"))
        .and(path("/fail"))
        .respond_with(ResponseTemplate::new(404))
        .mount(&mock_server)
        .await;

    let tempdir = TempDir::new().unwrap();
    let config_home = tempdir.path().join("xdg-config");
    std::fs::create_dir_all(config_home.join("downloader")).unwrap();

    let mut cmd = Command::cargo_bin("downloader").unwrap();
    cmd.arg("--output-dir")
        .arg(tempdir.path())
        .env("XDG_CONFIG_HOME", &config_home)
        .arg("-q")
        .arg(format!("{}/ok", mock_server.uri()))
        .arg(format!("{}/fail", mock_server.uri()));

    let assert = cmd.assert().failure();
    assert_eq!(
        assert.get_output().status.code(),
        Some(1),
        "partial success must yield exit code 1"
    );
}

/// Same scenario as above but without -q; asserts stdout contains partial/failure summary.
#[tokio::test]
async fn test_binary_exit_code_partial_success_stdout_contains_failure_summary() {
    let Some(mock_server) = start_mock_server_or_skip().await else {
        return socket_skip_return();
    };

    Mock::given(method("GET"))
        .and(path("/ok"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_bytes(b"PDF")
                .insert_header("Content-Type", "application/pdf"),
        )
        .mount(&mock_server)
        .await;

    Mock::given(method("GET"))
        .and(path("/fail"))
        .respond_with(ResponseTemplate::new(404))
        .mount(&mock_server)
        .await;

    let tempdir = TempDir::new().unwrap();
    let config_home = tempdir.path().join("xdg-config");
    std::fs::create_dir_all(config_home.join("downloader")).unwrap();

    let mut cmd = Command::cargo_bin("downloader").unwrap();
    cmd.arg("--output-dir")
        .arg(tempdir.path())
        .env("XDG_CONFIG_HOME", &config_home)
        .arg(format!("{}/ok", mock_server.uri()))
        .arg(format!("{}/fail", mock_server.uri()));

    let assert = cmd.assert().failure();
    assert_eq!(
        assert.get_output().status.code(),
        Some(1),
        "partial success must yield exit code 1"
    );

    // Completion summary is printed to stdout via println! in output::print_completion_summary.
    let stdout = String::from_utf8_lossy(&assert.get_output().stdout);
    assert!(
        stdout.contains("failed") || stdout.contains("Failure summary"),
        "stdout should indicate partial completion or failure summary; got: {:?}",
        stdout.as_ref()
    );
}
