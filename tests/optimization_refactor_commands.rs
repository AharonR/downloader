//! Integration tests for the optimization-refactor command-handler extraction.
//!
//! These tests exercise the CLI command flows (config show, auth clear, log, search)
//! after handlers were moved to `src/commands/*`. They lock in behavior so the
//! extraction remains correct and can be run with: `cargo test optimization_refactor`

#![allow(deprecated)]

use assert_cmd::Command;
use downloader_core::{Database, DownloadAttemptStatus, NewDownloadAttempt, Queue};
use predicates::prelude::*;
use tempfile::TempDir;

fn seed_log_history(db_path: &std::path::Path, url: &str, title: &str) {
    std::fs::create_dir_all(db_path.parent().unwrap()).unwrap();
    tokio_test::block_on(async {
        let db = Database::new(db_path).await.unwrap();
        let queue = Queue::new(db);
        let attempt = NewDownloadAttempt {
            url,
            final_url: Some(url),
            status: DownloadAttemptStatus::Success,
            file_path: Some("/tmp/example.pdf"),
            file_size: None,
            content_type: Some("application/pdf"),
            error_message: None,
            error_type: None,
            retry_count: 0,
            project: None,
            original_input: Some(url),
            http_status: Some(200),
            duration_ms: Some(10),
            title: Some(title),
            authors: Some("Author, Test"),
            doi: None,
            topics: None,
            parse_confidence: None,
            parse_confidence_factors: None,
        };
        queue.log_download_attempt(&attempt).await.unwrap();
    });
}

/// Config show: command handler runs and prints config_path and output_dir.
#[test]
fn optimization_refactor_config_show_prints_expected_keys() {
    let tempdir = TempDir::new().unwrap();
    let config_home = tempdir.path().join("xdg-config");

    let mut cmd = Command::cargo_bin("downloader").unwrap();
    cmd.args(["config", "show"])
        .env("XDG_CONFIG_HOME", &config_home)
        .assert()
        .success()
        .stdout(predicate::str::contains("config_path"))
        .stdout(predicate::str::contains("output_dir"))
        .stdout(predicate::str::contains("concurrency"))
        .stdout(predicate::str::contains("rate_limit"))
        .stdout(predicate::str::contains("verbosity"));
}

/// Auth clear: command handler runs and exits 0 (no panic after extraction).
#[test]
fn optimization_refactor_auth_clear_succeeds() {
    let tempdir = TempDir::new().unwrap();
    let config_home = tempdir.path().join("xdg-config");
    std::fs::create_dir_all(config_home.join("downloader")).unwrap();

    let mut cmd = Command::cargo_bin("downloader").unwrap();
    cmd.args(["auth", "clear"])
        .env("XDG_CONFIG_HOME", &config_home)
        .env("RUST_LOG", "info")
        .assert()
        .success();
}

/// Log: command handler runs with seeded history and prints at least one row.
#[test]
fn optimization_refactor_log_command_prints_history_rows() {
    let tempdir = TempDir::new().unwrap();
    let output_dir = tempdir.path().join("downloads");
    let db_path = output_dir.join(".downloader").join("queue.db");
    seed_log_history(&db_path, "https://example.com/paper.pdf", "Test Paper");

    let mut cmd = Command::cargo_bin("downloader").unwrap();
    cmd.args([
        "log",
        "--output-dir",
        output_dir.to_str().unwrap(),
        "--limit",
        "5",
    ])
    .env("RUST_LOG", "info")
    .assert()
    .success()
    .stdout(predicate::str::contains("Test Paper").or(predicate::str::contains("SUCCESS")));
}

/// Search: command handler runs with seeded history and prints results or "No search".
#[test]
fn optimization_refactor_search_command_runs() {
    let tempdir = TempDir::new().unwrap();
    let output_dir = tempdir.path().join("downloads");
    let db_path = output_dir.join(".downloader").join("queue.db");
    seed_log_history(
        &db_path,
        "https://example.com/paper.pdf",
        "Attention Is All You Need",
    );

    let mut cmd = Command::cargo_bin("downloader").unwrap();
    let output = cmd
        .args([
            "search",
            "attention",
            "--output-dir",
            output_dir.to_str().unwrap(),
            "--limit",
            "5",
        ])
        .env("RUST_LOG", "info")
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "search command failed: {:?}",
        output.stderr
    );
    let out = String::from_utf8_lossy(&output.stdout);
    assert!(
        out.contains("Attention") || out.contains("No search") || out.contains("No download"),
        "expected match or no-results message, got: {out}"
    );
}
