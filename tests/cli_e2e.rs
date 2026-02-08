//! End-to-end CLI tests for the downloader binary.
#![allow(deprecated)]

use assert_cmd::Command;
use predicates::prelude::*;

/// Test that the binary can be invoked and exits with code 0.
#[test]
fn test_binary_invocation_returns_zero() {
    let mut cmd = Command::cargo_bin("downloader").unwrap();
    cmd.assert().success();
}

/// Test that --help displays usage information and exits with code 0.
#[test]
fn test_binary_help_displays_usage() {
    let mut cmd = Command::cargo_bin("downloader").unwrap();
    cmd.arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("Batch download and organize"));
}

/// Test that --version displays version and exits with code 0.
#[test]
fn test_binary_version_displays_version() {
    let mut cmd = Command::cargo_bin("downloader").unwrap();
    cmd.arg("--version")
        .assert()
        .success()
        .stdout(predicate::str::contains("downloader"));
}

/// Test that invalid flags cause non-zero exit.
#[test]
fn test_binary_invalid_flag_returns_error() {
    let mut cmd = Command::cargo_bin("downloader").unwrap();
    cmd.arg("--invalid-flag")
        .assert()
        .failure()
        .stderr(predicate::str::contains("error"));
}

/// Test that -v flag works (verbose mode).
#[test]
fn test_binary_verbose_flag_accepted() {
    let mut cmd = Command::cargo_bin("downloader").unwrap();
    cmd.arg("-v").assert().success();
}

/// Test that -q flag works (quiet mode).
#[test]
fn test_binary_quiet_flag_accepted() {
    let mut cmd = Command::cargo_bin("downloader").unwrap();
    cmd.arg("-q").assert().success();
}

/// Test that piped stdin with no valid URLs exits cleanly.
#[test]
fn test_binary_stdin_no_urls_exits_cleanly() {
    let mut cmd = Command::cargo_bin("downloader").unwrap();
    cmd.write_stdin("no urls here, just text")
        .assert()
        .success();
}

/// Test that piped stdin with valid URLs is accepted (no crash).
#[test]
fn test_binary_stdin_with_invalid_domain_exits_cleanly() {
    let mut cmd = Command::cargo_bin("downloader").unwrap();
    // Use TEST-NET-1 address expected to be unreachable in normal environments.
    cmd.write_stdin("https://192.0.2.1/test.pdf")
        .arg("-q")
        .assert()
        .success();
}

/// Test that invalid URLs from stdin are handled without any network I/O.
#[test]
fn test_binary_stdin_with_invalid_url_exits_cleanly() {
    let mut cmd = Command::cargo_bin("downloader").unwrap();
    // Exceeds parser MAX_URL_LENGTH, so it is always rejected before download.
    let long_url = format!("https://example.com/{}", "a".repeat(2100));
    cmd.write_stdin(long_url).arg("-q").assert().success();
}

/// Test that flags after positional URLs are parsed as flags, not URLs.
#[test]
fn test_binary_flag_after_positional_url_is_parsed_as_flag() {
    let mut cmd = Command::cargo_bin("downloader").unwrap();
    // Non-URL positional token avoids network I/O while still exercising flag ordering.
    cmd.arg("not-a-url-token")
        .arg("-q")
        .assert()
        .success()
        .stderr(predicate::str::is_empty());
}
