//! End-to-end CLI tests for the downloader binary.

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
