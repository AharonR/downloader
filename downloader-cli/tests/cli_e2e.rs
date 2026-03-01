//! End-to-end CLI tests for the downloader binary.

// `Command::cargo_bin` is deprecated in assert_cmd >=2.0.17 in favor of
// `cargo::cargo_bin_cmd!` macro. Suppressed until migration to the new API.
#![allow(deprecated)]

use assert_cmd::Command;
use downloader_core::{
    Database, DownloadAttemptStatus, DownloadErrorType, NewDownloadAttempt, Queue,
};
use predicates::prelude::*;
use tempfile::TempDir;

fn write_downloader_config(config_home: &std::path::Path, contents: &str) {
    let config_dir = config_home.join("downloader");
    std::fs::create_dir_all(&config_dir).unwrap();
    std::fs::write(config_dir.join("config.toml"), contents).unwrap();
}

fn toml_path(path: &std::path::Path) -> String {
    path.to_string_lossy().replace('\\', "\\\\")
}

fn seed_success_history_row(
    db_path: &std::path::Path,
    url: &str,
    title: &str,
    project: Option<&str>,
) {
    seed_success_history_row_with_confidence(db_path, url, title, project, None, None);
}

fn seed_success_history_row_with_confidence(
    db_path: &std::path::Path,
    url: &str,
    title: &str,
    project: Option<&str>,
    parse_confidence: Option<&str>,
    parse_confidence_factors: Option<&str>,
) {
    std::fs::create_dir_all(db_path.parent().expect("db should have a parent")).unwrap();

    tokio_test::block_on(async {
        let db = Database::new(db_path).await.unwrap();
        let queue = Queue::new(db);
        let attempt = NewDownloadAttempt {
            url,
            final_url: Some(url),
            status: DownloadAttemptStatus::Success,
            file_path: None,
            file_size: None,
            content_type: Some("application/pdf"),
            error_message: None,
            error_type: None,
            retry_count: 0,
            project,
            original_input: Some(url),
            http_status: Some(200),
            duration_ms: Some(10),
            title: Some(title),
            authors: Some("Test, User"),
            doi: None,
            topics: None,
            parse_confidence,
            parse_confidence_factors,
        };
        queue.log_download_attempt(&attempt).await.unwrap();
    });
}

fn seed_search_history_row(
    db_path: &std::path::Path,
    url: &str,
    title: &str,
    authors: &str,
    doi: Option<&str>,
    project: Option<&str>,
    file_path: Option<&str>,
) {
    std::fs::create_dir_all(db_path.parent().expect("db should have a parent")).unwrap();

    tokio_test::block_on(async {
        let db = Database::new(db_path).await.unwrap();
        let queue = Queue::new(db);
        let attempt = NewDownloadAttempt {
            url,
            final_url: Some(url),
            status: DownloadAttemptStatus::Success,
            file_path,
            file_size: None,
            content_type: Some("application/pdf"),
            error_message: None,
            error_type: None,
            retry_count: 0,
            project,
            original_input: Some(url),
            http_status: Some(200),
            duration_ms: Some(10),
            title: Some(title),
            authors: Some(authors),
            doi,
            topics: None,
            parse_confidence: None,
            parse_confidence_factors: None,
        };
        queue.log_download_attempt(&attempt).await.unwrap();
    });
}

fn seed_failed_history_row(
    db_path: &std::path::Path,
    url: &str,
    title: &str,
    error_message: &str,
    error_type: DownloadErrorType,
) {
    std::fs::create_dir_all(db_path.parent().expect("db should have a parent")).unwrap();

    tokio_test::block_on(async {
        let db = Database::new(db_path).await.unwrap();
        let queue = Queue::new(db);
        let attempt = NewDownloadAttempt {
            url,
            final_url: Some(url),
            status: DownloadAttemptStatus::Failed,
            file_path: None,
            file_size: None,
            content_type: Some("application/pdf"),
            error_message: Some(error_message),
            error_type: Some(error_type),
            retry_count: 0,
            project: None,
            original_input: Some(url),
            http_status: Some(404),
            duration_ms: Some(10),
            title: Some(title),
            authors: Some("Test, User"),
            doi: None,
            topics: None,
            parse_confidence: Some("low"),
            parse_confidence_factors: Some(
                r#"{"has_authors":false,"has_year":true,"has_title":false,"author_count":0}"#,
            ),
        };
        queue.log_download_attempt(&attempt).await.unwrap();
    });
}

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

/// Test that `--help` documents process exit codes.
#[test]
fn test_binary_help_displays_exit_codes() {
    let mut cmd = Command::cargo_bin("downloader").unwrap();
    cmd.arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("Exit codes:"))
        .stdout(predicate::str::contains("0 = all items succeeded"))
        .stdout(predicate::str::contains("1 = partial success"))
        .stdout(predicate::str::contains(
            "2 = complete failure or fatal error",
        ));
}

/// Regression: clap help must win even if stdin has data.
#[test]
fn test_binary_help_with_stdin_bypasses_quick_start_guidance() {
    let mut cmd = Command::cargo_bin("downloader").unwrap();
    cmd.arg("--help")
        .write_stdin("https://example.com/file.pdf\n")
        .assert()
        .success()
        .stdout(predicate::str::contains("Batch download and organize"))
        .stdout(predicate::str::contains("No input provided").not())
        .stdout(predicate::str::contains("Received empty stdin input").not());
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

/// Test that successful command paths return exit code 0.
#[test]
fn test_binary_exit_code_success_is_zero() {
    let mut cmd = Command::cargo_bin("downloader").unwrap();
    let assert = cmd.assert().success();
    assert_eq!(assert.get_output().status.code(), Some(0));
}

/// Test that complete-failure paths return exit code 2.
#[test]
fn test_binary_exit_code_complete_failure_is_two() {
    let tempdir = TempDir::new().unwrap();
    let config_home = tempdir.path().join("xdg-config");
    let mut cmd = Command::cargo_bin("downloader").unwrap();
    let assert = cmd
        .arg("--output-dir")
        .arg(tempdir.path())
        .env("XDG_CONFIG_HOME", &config_home)
        .arg("-q")
        .write_stdin("Smith, J. (2024). Paper Title. Journal Name, 1(2), 3-4.")
        .assert()
        .failure();
    assert_eq!(assert.get_output().status.code(), Some(2));
}

/// Test that `--debug` enables detailed trace/debug output.
#[test]
fn test_binary_debug_flag_emits_debug_parsed_args_line() {
    let tempdir = TempDir::new().unwrap();
    let mut cmd = Command::cargo_bin("downloader").unwrap();
    let assert = cmd
        .env("RUST_LOG", "warn")
        .arg("--debug")
        .arg("--output-dir")
        .arg(tempdir.path())
        .arg("not-a-url-token")
        .assert()
        .success();
    let output = assert.get_output();
    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        combined.contains("CLI arguments parsed"),
        "expected debug parsed-args output, got: {combined}"
    );
}

/// Test that default verbosity omits debug parsed-args line.
#[test]
fn test_binary_default_omits_debug_parsed_args_line() {
    let tempdir = TempDir::new().unwrap();
    let mut cmd = Command::cargo_bin("downloader").unwrap();
    let assert = cmd
        .env("RUST_LOG", "warn")
        .arg("--output-dir")
        .arg(tempdir.path())
        .arg("not-a-url-token")
        .assert()
        .success();
    let output = assert.get_output();
    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        !combined.contains("CLI arguments parsed"),
        "did not expect debug parsed-args output at default verbosity: {combined}"
    );
}

/// Test that empty-stdin quick-start output includes common examples.
#[test]
fn test_binary_empty_stdin_shows_quick_start_examples() {
    let tempdir = TempDir::new().unwrap();
    let mut cmd = Command::cargo_bin("downloader").unwrap();
    cmd.arg("--output-dir")
        .arg(tempdir.path())
        .write_stdin("")
        .assert()
        .success()
        .stdout(predicate::str::contains("Received empty stdin input"))
        .stdout(predicate::str::contains(
            "Example: echo 'https://example.com/file.pdf' | downloader",
        ))
        .stdout(predicate::str::contains(
            "Example: downloader https://example.com/file.pdf",
        ));
}

/// Test that quick-start lines fit within 80 columns.
#[test]
fn test_binary_quick_start_lines_fit_80_columns() {
    let tempdir = TempDir::new().unwrap();
    let mut cmd = Command::cargo_bin("downloader").unwrap();
    let assert = cmd
        .arg("--output-dir")
        .arg(tempdir.path())
        .env("COLUMNS", "80")
        .write_stdin("")
        .assert()
        .success();
    let output = assert.get_output();
    let stdout = String::from_utf8_lossy(&output.stdout);

    assert!(
        stdout.contains("Example:"),
        "expected quick-start output on stdout, got: {stdout}"
    );

    for line in stdout.lines().filter(|line| !line.trim().is_empty()) {
        assert!(
            line.chars().count() <= 80,
            "expected line width <= 80, got {}: {line}",
            line.chars().count()
        );
    }
}

/// Test that invalid small COLUMNS values fall back to readable quick-start output.
#[test]
fn test_binary_quick_start_small_columns_env_falls_back_to_default_width() {
    let tempdir = TempDir::new().unwrap();
    let mut cmd = Command::cargo_bin("downloader").unwrap();
    let assert = cmd
        .arg("--output-dir")
        .arg(tempdir.path())
        .env("COLUMNS", "5")
        .write_stdin("")
        .assert()
        .success();
    let output = assert.get_output();
    let stdout = String::from_utf8_lossy(&output.stdout);

    assert!(
        stdout.contains("Example: echo 'https://example.com/file.pdf' | downloader"),
        "expected fallback width behavior for tiny COLUMNS, got: {stdout}"
    );
}

/// Test that `config show` reports defaults when no config file exists.
#[test]
fn test_binary_config_show_missing_file_uses_defaults() {
    let tempdir = TempDir::new().unwrap();
    let config_home = tempdir.path().join("xdg-config");

    let mut cmd = Command::cargo_bin("downloader").unwrap();
    cmd.args(["config", "show"])
        .env("XDG_CONFIG_HOME", &config_home)
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "config_file = not found (using defaults)",
        ))
        .stdout(predicate::str::contains("concurrency = 10"))
        .stdout(predicate::str::contains("rate_limit = 1000"))
        .stdout(predicate::str::contains("verbosity = default"));
}

/// Test that `config show` loads values from XDG config path.
#[test]
fn test_binary_config_show_loads_xdg_file() {
    let tempdir = TempDir::new().unwrap();
    let config_home = tempdir.path().join("xdg-config");
    let configured_output = tempdir.path().join("configured-downloads");
    write_downloader_config(
        &config_home,
        &format!(
            "output_dir = \"{}\"\nconcurrency = 7\nrate_limit = 2500\nverbosity = \"debug\"\n",
            toml_path(&configured_output)
        ),
    );

    let mut cmd = Command::cargo_bin("downloader").unwrap();
    cmd.args(["config", "show"])
        .env("XDG_CONFIG_HOME", &config_home)
        .assert()
        .success()
        .stdout(predicate::str::contains("config_file = loaded"))
        .stdout(predicate::str::contains(format!(
            "output_dir = {}",
            configured_output.display()
        )))
        .stdout(predicate::str::contains("concurrency = 7"))
        .stdout(predicate::str::contains("rate_limit = 2500"))
        .stdout(predicate::str::contains("verbosity = debug"));
}

/// Test that `config show` falls back to HOME when XDG_CONFIG_HOME is unset.
#[test]
fn test_binary_config_show_loads_home_fallback_file() {
    let tempdir = TempDir::new().unwrap();
    let home_dir = tempdir.path().join("home");
    let configured_output = tempdir.path().join("home-configured-downloads");
    write_downloader_config(
        &home_dir.join(".config"),
        &format!(
            "output_dir = \"{}\"\nconcurrency = 9\nrate_limit = 3000\nverbosity = \"quiet\"\n",
            toml_path(&configured_output)
        ),
    );

    let mut cmd = Command::cargo_bin("downloader").unwrap();
    cmd.args(["config", "show"])
        .env_remove("XDG_CONFIG_HOME")
        .env("HOME", &home_dir)
        .assert()
        .success()
        .stdout(predicate::str::contains("config_file = loaded"))
        .stdout(predicate::str::contains(format!(
            "output_dir = {}",
            configured_output.display()
        )))
        .stdout(predicate::str::contains("concurrency = 9"))
        .stdout(predicate::str::contains("rate_limit = 3000"))
        .stdout(predicate::str::contains("verbosity = quiet"));
}

/// Test that download mode picks output_dir from config defaults when CLI is unset.
#[test]
fn test_binary_download_uses_config_output_dir_default() {
    let tempdir = TempDir::new().unwrap();
    let config_home = tempdir.path().join("xdg-config");
    let configured_output = tempdir.path().join("configured-downloads");
    write_downloader_config(
        &config_home,
        &format!("output_dir = \"{}\"\n", toml_path(&configured_output)),
    );

    let mut cmd = Command::cargo_bin("downloader").unwrap();
    cmd.env("XDG_CONFIG_HOME", &config_home)
        .arg("not-a-url-token")
        .assert()
        .success();

    assert!(
        configured_output.join(".downloader").exists(),
        "expected .downloader state under config output dir"
    );
}

/// Test that CLI output_dir overrides config output_dir.
#[test]
fn test_binary_download_cli_output_dir_overrides_config() {
    let tempdir = TempDir::new().unwrap();
    let config_home = tempdir.path().join("xdg-config");
    let configured_output = tempdir.path().join("configured-downloads");
    let cli_output = tempdir.path().join("cli-downloads");
    write_downloader_config(
        &config_home,
        &format!("output_dir = \"{}\"\n", toml_path(&configured_output)),
    );

    let mut cmd = Command::cargo_bin("downloader").unwrap();
    cmd.env("XDG_CONFIG_HOME", &config_home)
        .arg("--output-dir")
        .arg(&cli_output)
        .arg("not-a-url-token")
        .assert()
        .success();

    assert!(
        cli_output.join(".downloader").exists(),
        "expected .downloader state under CLI output dir"
    );
    assert!(
        !configured_output.join(".downloader").exists(),
        "did not expect config output dir to be used when CLI override exists"
    );
}

/// Test that `downloader log` reports a helpful message when no history DB exists.
#[test]
fn test_binary_log_command_without_history_reports_empty_state() {
    let tempdir = TempDir::new().unwrap();
    let mut cmd = Command::cargo_bin("downloader").unwrap();
    cmd.args(["log", "--output-dir"])
        .arg(tempdir.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("No download history found"));
}

/// Test that `downloader log` scans all nested project DBs by default.
#[test]
fn test_binary_log_defaults_to_global_history_scope() {
    let tempdir = TempDir::new().unwrap();
    let root_db = tempdir.path().join(".downloader/queue.db");
    let project_db = tempdir.path().join("ProjectA/.downloader/queue.db");

    seed_success_history_row(&root_db, "https://root.example.com/r.pdf", "Root Row", None);
    seed_success_history_row(
        &project_db,
        "https://project.example.com/p.pdf",
        "Project Row",
        None,
    );

    let mut cmd = Command::cargo_bin("downloader").unwrap();
    cmd.args(["log", "--output-dir"])
        .arg(tempdir.path())
        .arg("--limit")
        .arg("10")
        .assert()
        .success()
        .stdout(predicate::str::contains("Root Row"))
        .stdout(predicate::str::contains("Project Row"));
}

/// Test that `--project` limits `downloader log` to a single project DB.
#[test]
fn test_binary_log_project_override_limits_scope() {
    let tempdir = TempDir::new().unwrap();
    let root_db = tempdir.path().join(".downloader/queue.db");
    let project_output_dir = tempdir.path().join("ProjectA");
    let project_db = project_output_dir.join(".downloader/queue.db");
    std::fs::create_dir_all(&project_output_dir).unwrap();
    let project_key = std::fs::canonicalize(&project_output_dir)
        .unwrap()
        .to_string_lossy()
        .to_string();

    seed_success_history_row(&root_db, "https://root.example.com/r.pdf", "Root Row", None);
    seed_success_history_row(
        &project_db,
        "https://project.example.com/p.pdf",
        "Project Row",
        Some(&project_key),
    );

    let mut cmd = Command::cargo_bin("downloader").unwrap();
    cmd.args(["log", "--output-dir"])
        .arg(tempdir.path())
        .arg("--project")
        .arg("ProjectA")
        .assert()
        .success()
        .stdout(predicate::str::contains("Project Row"))
        .stdout(predicate::str::contains("Root Row").not());
}

/// Test that log output is explicit when rows are capped by --limit.
#[test]
fn test_binary_log_reports_when_output_is_limited() {
    let tempdir = TempDir::new().unwrap();
    let root_db = tempdir.path().join(".downloader/queue.db");

    seed_success_history_row(&root_db, "https://example.com/1.pdf", "Row 1", None);
    seed_success_history_row(&root_db, "https://example.com/2.pdf", "Row 2", None);
    seed_success_history_row(&root_db, "https://example.com/3.pdf", "Row 3", None);

    let mut cmd = Command::cargo_bin("downloader").unwrap();
    cmd.args(["log", "--output-dir"])
        .arg(tempdir.path())
        .arg("--limit")
        .arg("2")
        .assert()
        .success()
        .stdout(predicate::str::contains("Showing first 2 rows"));
}

/// Test that `downloader log --uncertain` returns only low-confidence rows.
#[test]
fn test_binary_log_uncertain_filters_low_confidence_rows() {
    let tempdir = TempDir::new().unwrap();
    let root_db = tempdir.path().join(".downloader/queue.db");

    seed_success_history_row_with_confidence(
        &root_db,
        "https://low.example.com/needs-review.pdf",
        "Low Confidence Row",
        None,
        Some("low"),
        Some(r#"{"has_authors":false,"has_year":true,"has_title":false,"author_count":0}"#),
    );
    seed_success_history_row_with_confidence(
        &root_db,
        "https://medium.example.com/likely-ok.pdf",
        "Medium Confidence Row",
        None,
        Some("medium"),
        Some(r#"{"has_authors":true,"has_year":true,"has_title":false,"author_count":1}"#),
    );
    seed_success_history_row(
        &root_db,
        "https://none.example.com/legacy.pdf",
        "Legacy Row",
        None,
    );

    let mut cmd = Command::cargo_bin("downloader").unwrap();
    cmd.args(["log", "--output-dir"])
        .arg(tempdir.path())
        .arg("--uncertain")
        .arg("--limit")
        .arg("10")
        .assert()
        .success()
        .stdout(predicate::str::contains("Low Confidence Row"))
        .stdout(predicate::str::contains("confidence=low"))
        .stdout(predicate::str::contains("Medium Confidence Row").not())
        .stdout(predicate::str::contains("Legacy Row").not());
}

/// Test that `--uncertain` composes with other log filters like `--domain`.
#[test]
fn test_binary_log_uncertain_composes_with_domain_filter() {
    let tempdir = TempDir::new().unwrap();
    let root_db = tempdir.path().join(".downloader/queue.db");

    seed_success_history_row_with_confidence(
        &root_db,
        "https://target.example.org/match.pdf",
        "Target Low Row",
        None,
        Some("low"),
        Some(r#"{"has_authors":false,"has_year":true,"has_title":false,"author_count":0}"#),
    );
    seed_success_history_row_with_confidence(
        &root_db,
        "https://other.example.net/other.pdf",
        "Other Low Row",
        None,
        Some("low"),
        Some(r#"{"has_authors":false,"has_year":true,"has_title":false,"author_count":0}"#),
    );

    let mut cmd = Command::cargo_bin("downloader").unwrap();
    cmd.args(["log", "--output-dir"])
        .arg(tempdir.path())
        .arg("--uncertain")
        .arg("--domain")
        .arg("target.example.org")
        .assert()
        .success()
        .stdout(predicate::str::contains("Target Low Row"))
        .stdout(predicate::str::contains("Other Low Row").not());
}

/// Test that `--uncertain` composes with `--since`.
#[test]
fn test_binary_log_uncertain_composes_with_since_filter() {
    let tempdir = TempDir::new().unwrap();
    let root_db = tempdir.path().join(".downloader/queue.db");

    seed_success_history_row_with_confidence(
        &root_db,
        "https://low.example.com/needs-review.pdf",
        "Low Confidence Row",
        None,
        Some("low"),
        Some(r#"{"has_authors":false,"has_year":true,"has_title":false,"author_count":0}"#),
    );

    let mut cmd = Command::cargo_bin("downloader").unwrap();
    cmd.args(["log", "--output-dir"])
        .arg(tempdir.path())
        .arg("--uncertain")
        .arg("--since")
        .arg("9999-01-01 00:00:00")
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "No history rows matched the current filters.",
        ));
}

/// Test that `--uncertain` composes with `--project` and project-scoped DB selection.
#[test]
fn test_binary_log_uncertain_composes_with_project_scope() {
    let tempdir = TempDir::new().unwrap();
    let root_db = tempdir.path().join(".downloader/queue.db");
    let project_output_dir = tempdir.path().join("ProjectA");
    let project_db = project_output_dir.join(".downloader/queue.db");
    std::fs::create_dir_all(&project_output_dir).unwrap();
    let project_key = std::fs::canonicalize(&project_output_dir)
        .unwrap()
        .to_string_lossy()
        .to_string();

    seed_success_history_row_with_confidence(
        &root_db,
        "https://root.example.com/needs-review.pdf",
        "Root Low Row",
        None,
        Some("low"),
        Some(r#"{"has_authors":false,"has_year":true,"has_title":false,"author_count":0}"#),
    );
    seed_success_history_row_with_confidence(
        &project_db,
        "https://project.example.com/needs-review.pdf",
        "Project Low Row",
        Some(&project_key),
        Some("low"),
        Some(r#"{"has_authors":false,"has_year":true,"has_title":false,"author_count":0}"#),
    );

    let mut cmd = Command::cargo_bin("downloader").unwrap();
    cmd.args(["log", "--output-dir"])
        .arg(tempdir.path())
        .arg("--project")
        .arg("ProjectA")
        .arg("--uncertain")
        .assert()
        .success()
        .stdout(predicate::str::contains("Project Low Row"))
        .stdout(predicate::str::contains("Root Low Row").not());
}

/// Test that `--uncertain` respects `--limit` and prints truncation guidance.
#[test]
fn test_binary_log_uncertain_respects_limit_and_reports_truncation() {
    let tempdir = TempDir::new().unwrap();
    let root_db = tempdir.path().join(".downloader/queue.db");

    seed_success_history_row_with_confidence(
        &root_db,
        "https://example.com/low-1.pdf",
        "Low Row 1",
        None,
        Some("low"),
        Some(r#"{"has_authors":false,"has_year":true,"has_title":false,"author_count":0}"#),
    );
    seed_success_history_row_with_confidence(
        &root_db,
        "https://example.com/low-2.pdf",
        "Low Row 2",
        None,
        Some("low"),
        Some(r#"{"has_authors":false,"has_year":true,"has_title":false,"author_count":0}"#),
    );
    seed_success_history_row_with_confidence(
        &root_db,
        "https://example.com/low-3.pdf",
        "Low Row 3",
        None,
        Some("low"),
        Some(r#"{"has_authors":false,"has_year":true,"has_title":false,"author_count":0}"#),
    );
    seed_success_history_row_with_confidence(
        &root_db,
        "https://example.com/medium.pdf",
        "Medium Row",
        None,
        Some("medium"),
        Some(r#"{"has_authors":true,"has_year":true,"has_title":false,"author_count":1}"#),
    );

    let mut cmd = Command::cargo_bin("downloader").unwrap();
    cmd.args(["log", "--output-dir"])
        .arg(tempdir.path())
        .arg("--uncertain")
        .arg("--limit")
        .arg("2")
        .assert()
        .success()
        .stdout(predicate::str::contains("Low Row 3"))
        .stdout(predicate::str::contains("Low Row 2"))
        .stdout(predicate::str::contains("Low Row 1").not())
        .stdout(predicate::str::contains("Medium Row").not())
        .stdout(predicate::str::contains("Showing first 2 rows"));
}

/// Test that `--uncertain` composes with all scope filters in one invocation.
#[test]
fn test_binary_log_uncertain_composes_all_scope_filters_together() {
    let tempdir = TempDir::new().unwrap();
    let root_db = tempdir.path().join(".downloader/queue.db");
    let project_output_dir = tempdir.path().join("ProjectA");
    let project_db = project_output_dir.join(".downloader/queue.db");
    std::fs::create_dir_all(&project_output_dir).unwrap();
    let project_key = std::fs::canonicalize(&project_output_dir)
        .unwrap()
        .to_string_lossy()
        .to_string();

    // Out-of-scope root row (project filter should exclude this).
    seed_success_history_row_with_confidence(
        &root_db,
        "https://target.example.org/root.pdf",
        "Root Target Low",
        None,
        Some("low"),
        Some(r#"{"has_authors":false,"has_year":true,"has_title":false,"author_count":0}"#),
    );

    // Project-scoped rows with mixed filter outcomes.
    seed_success_history_row_with_confidence(
        &project_db,
        "https://target.example.org/old.pdf",
        "Project Target Low Old",
        Some(&project_key),
        Some("low"),
        Some(r#"{"has_authors":false,"has_year":true,"has_title":false,"author_count":0}"#),
    );
    seed_success_history_row_with_confidence(
        &project_db,
        "https://other.example.net/other.pdf",
        "Project Other Low",
        Some(&project_key),
        Some("low"),
        Some(r#"{"has_authors":false,"has_year":true,"has_title":false,"author_count":0}"#),
    );
    seed_success_history_row_with_confidence(
        &project_db,
        "https://target.example.org/medium.pdf",
        "Project Target Medium",
        Some(&project_key),
        Some("medium"),
        Some(r#"{"has_authors":true,"has_year":true,"has_title":false,"author_count":1}"#),
    );
    seed_success_history_row_with_confidence(
        &project_db,
        "https://target.example.org/new.pdf",
        "Project Target Low New",
        Some(&project_key),
        Some("low"),
        Some(r#"{"has_authors":false,"has_year":true,"has_title":false,"author_count":0}"#),
    );

    let mut cmd = Command::cargo_bin("downloader").unwrap();
    cmd.args(["log", "--output-dir"])
        .arg(tempdir.path())
        .arg("--project")
        .arg("ProjectA")
        .arg("--uncertain")
        .arg("--since")
        .arg("1970-01-01 00:00:00")
        .arg("--domain")
        .arg("target.example.org")
        .arg("--limit")
        .arg("1")
        .assert()
        .success()
        .stdout(predicate::str::contains("Project Target Low New"))
        .stdout(predicate::str::contains("Project Target Low Old").not())
        .stdout(predicate::str::contains("Project Other Low").not())
        .stdout(predicate::str::contains("Project Target Medium").not())
        .stdout(predicate::str::contains("Root Target Low").not())
        .stdout(predicate::str::contains("Showing first 1 rows"));
}

/// Output contract: failed rows rendered with --failed include What/Why/Fix guidance.
#[test]
fn test_binary_log_failed_rows_include_actionable_failure_guidance() {
    let tempdir = TempDir::new().unwrap();
    let root_db = tempdir.path().join(".downloader/queue.db");
    seed_failed_history_row(
        &root_db,
        "https://example.com/missing.pdf",
        "Missing Paper",
        "HTTP 404 downloading https://example.com/missing.pdf\n  Suggestion: Verify source",
        DownloadErrorType::NotFound,
    );

    let mut cmd = Command::cargo_bin("downloader").unwrap();
    cmd.args(["log", "--output-dir"])
        .arg(tempdir.path())
        .arg("--failed")
        .arg("--limit")
        .arg("1")
        .assert()
        .success()
        .stdout(predicate::str::contains("Missing Paper"))
        .stdout(predicate::str::contains("What: Source not found"))
        .stdout(predicate::str::contains("Why:"))
        .stdout(predicate::str::contains(
            "Fix: Verify the source URL/DOI/reference",
        ));
}

/// Test that `downloader search` reports a helpful message when no history DB exists.
#[test]
fn test_binary_search_without_history_reports_empty_state() {
    let tempdir = TempDir::new().unwrap();
    let mut cmd = Command::cargo_bin("downloader").unwrap();
    cmd.args(["search", "attention", "--output-dir"])
        .arg(tempdir.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("No download history found"));
}

/// Test that search matches title/authors/doi fields.
#[test]
fn test_binary_search_matches_title_authors_and_doi_fields() {
    let tempdir = TempDir::new().unwrap();
    let root_db = tempdir.path().join(".downloader/queue.db");
    seed_search_history_row(
        &root_db,
        "https://example.org/attention.pdf",
        "Attention Is All You Need",
        "Vaswani, Ashish",
        Some("10.48550/arxiv.1706.03762"),
        None,
        Some("/tmp/attention.pdf"),
    );

    let mut by_title = Command::cargo_bin("downloader").unwrap();
    by_title
        .args(["search", "attention", "--output-dir"])
        .arg(tempdir.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("Attention Is All You Need"))
        .stdout(predicate::str::contains("match=title"));

    let mut by_author = Command::cargo_bin("downloader").unwrap();
    by_author
        .args(["search", "vaswani", "--output-dir"])
        .arg(tempdir.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("Attention Is All You Need"))
        .stdout(predicate::str::contains("match=authors"));

    let mut by_doi = Command::cargo_bin("downloader").unwrap();
    by_doi
        .args(["search", "10.48550/arxiv.1706.03762", "--output-dir"])
        .arg(tempdir.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("Attention Is All You Need"))
        .stdout(predicate::str::contains("match=doi"));
}

/// Test that search project scoping uses project-local DB rows.
#[test]
fn test_binary_search_project_filter_limits_scope() {
    let tempdir = TempDir::new().unwrap();
    let root_db = tempdir.path().join(".downloader/queue.db");
    let project_output_dir = tempdir.path().join("ProjectA");
    let project_db = project_output_dir.join(".downloader/queue.db");
    std::fs::create_dir_all(&project_output_dir).unwrap();
    let project_key = std::fs::canonicalize(&project_output_dir)
        .unwrap()
        .to_string_lossy()
        .to_string();

    seed_search_history_row(
        &root_db,
        "https://root.example.com/r.pdf",
        "Root Search Row",
        "Root, User",
        None,
        None,
        Some("/tmp/root.pdf"),
    );
    seed_search_history_row(
        &project_db,
        "https://project.example.com/p.pdf",
        "Project Search Row",
        "Project, User",
        None,
        Some(&project_key),
        Some("/tmp/project.pdf"),
    );

    let mut cmd = Command::cargo_bin("downloader").unwrap();
    cmd.args(["search", "search", "--output-dir"])
        .arg(tempdir.path())
        .arg("--project")
        .arg("ProjectA")
        .assert()
        .success()
        .stdout(predicate::str::contains("Project Search Row"))
        .stdout(predicate::str::contains("Root Search Row").not());
}

/// Test that relative file paths are rendered relative to each history DB root.
#[test]
fn test_binary_search_resolves_relative_file_path_against_history_root() {
    let tempdir = TempDir::new().unwrap();
    let root_db = tempdir.path().join(".downloader/queue.db");
    let relative_file = tempdir.path().join("relative").join("openable.pdf");
    std::fs::create_dir_all(relative_file.parent().unwrap()).unwrap();
    std::fs::write(&relative_file, b"pdf").unwrap();

    seed_search_history_row(
        &root_db,
        "https://example.org/relative.pdf",
        "Relative Path Row",
        "Doe, Jane",
        None,
        None,
        Some("relative/openable.pdf"),
    );

    let mut cmd = Command::cargo_bin("downloader").unwrap();
    cmd.args(["search", "relative", "--output-dir"])
        .env("COLUMNS", "300")
        .arg(tempdir.path())
        .assert()
        .success()
        .stdout(predicate::str::contains(
            relative_file.to_string_lossy().as_ref(),
        ));
}

/// Test that fuzzy typo matching can still locate relevant rows.
#[test]
fn test_binary_search_fuzzy_typo_returns_result() {
    let tempdir = TempDir::new().unwrap();
    let root_db = tempdir.path().join(".downloader/queue.db");
    seed_search_history_row(
        &root_db,
        "https://example.org/attention.pdf",
        "Attention Is All You Need",
        "Vaswani, Ashish",
        None,
        None,
        Some("/tmp/attention.pdf"),
    );

    let mut cmd = Command::cargo_bin("downloader").unwrap();
    cmd.args(["search", "attenton", "--output-dir"])
        .arg(tempdir.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("Attention Is All You Need"));
}

/// Test that `--open` prints What/Why/Fix guidance when selected file is missing.
#[test]
fn test_binary_search_open_missing_path_shows_actionable_guidance() {
    let tempdir = TempDir::new().unwrap();
    let root_db = tempdir.path().join(".downloader/queue.db");
    seed_search_history_row(
        &root_db,
        "https://example.org/missing.pdf",
        "Missing File Row",
        "Doe, Jane",
        None,
        None,
        Some("/tmp/definitely-missing-search-file.pdf"),
    );

    let mut cmd = Command::cargo_bin("downloader").unwrap();
    cmd.args(["search", "missing", "--output-dir"])
        .arg(tempdir.path())
        .arg("--open")
        .assert()
        .success()
        .stdout(predicate::str::contains("Missing File Row"))
        .stdout(predicate::str::contains(
            "What: Cannot open search result file",
        ))
        .stdout(predicate::str::contains(
            "Fix: Re-run without --open or redownload the item.",
        ));
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

/// Test that --project creates a sanitized folder under --output-dir.
#[test]
fn test_binary_project_flag_creates_sanitized_folder() {
    let tempdir = TempDir::new().unwrap();
    let expected = tempdir.path().join("Climate-Research");

    let mut cmd = Command::cargo_bin("downloader").unwrap();
    cmd.arg("--output-dir")
        .arg(tempdir.path())
        .arg("--project")
        .arg("Climate Research")
        .arg("not-a-url-token")
        .assert()
        .success();

    assert!(
        expected.is_dir(),
        "expected project folder to exist at {:?}",
        expected
    );
}

/// Test that repeated runs reuse the same project folder without creating duplicates.
#[test]
fn test_binary_project_folder_is_reused_not_duplicated() {
    let tempdir = TempDir::new().unwrap();

    for _ in 0..2 {
        let mut cmd = Command::cargo_bin("downloader").unwrap();
        cmd.arg("--output-dir")
            .arg(tempdir.path())
            .arg("--project")
            .arg("Climate Research")
            .arg("not-a-url-token")
            .assert()
            .success();
    }

    let matching: Vec<_> = std::fs::read_dir(tempdir.path())
        .unwrap()
        .filter_map(Result::ok)
        .filter(|entry| {
            let name = entry.file_name();
            name.to_string_lossy().starts_with("Climate-Research")
        })
        .collect();

    assert_eq!(
        matching.len(),
        1,
        "expected exactly one reused project folder, found {}",
        matching.len()
    );
}

/// Test that nested project paths create nested folders using `/` separators.
#[test]
fn test_binary_project_nested_path_creates_nested_folders() {
    let tempdir = TempDir::new().unwrap();
    let expected = tempdir.path().join("Climate/Emissions/2024");

    let mut cmd = Command::cargo_bin("downloader").unwrap();
    cmd.arg("--output-dir")
        .arg(tempdir.path())
        .arg("--project")
        .arg("Climate/Emissions/2024")
        .arg("not-a-url-token")
        .assert()
        .success();

    assert!(
        expected.is_dir(),
        "expected nested project folders at {:?}",
        expected
    );
}

/// Test that repeated separators are rejected as invalid empty segments.
#[test]
fn test_binary_project_rejects_repeated_separators() {
    let tempdir = TempDir::new().unwrap();

    let mut cmd = Command::cargo_bin("downloader").unwrap();
    cmd.arg("--output-dir")
        .arg(tempdir.path())
        .arg("--project")
        .arg("Climate//2024")
        .arg("not-a-url-token")
        .assert()
        .failure()
        .stderr(predicate::str::contains("project name is empty"));
}

/// Test that nested project depth beyond the guard limit fails cleanly.
#[test]
fn test_binary_project_depth_limit_enforced() {
    let tempdir = TempDir::new().unwrap();
    let too_deep = (0..11)
        .map(|i| format!("n{i}"))
        .collect::<Vec<String>>()
        .join("/");

    let mut cmd = Command::cargo_bin("downloader").unwrap();
    cmd.arg("--output-dir")
        .arg(tempdir.path())
        .arg("--project")
        .arg(too_deep)
        .arg("not-a-url-token")
        .assert()
        .failure()
        .stderr(predicate::str::contains("nesting depth"));
}

/// Regression guard: non-project runs keep using the output directory directly.
#[test]
fn test_binary_without_project_keeps_default_output_layout() {
    let tempdir = TempDir::new().unwrap();

    let mut cmd = Command::cargo_bin("downloader").unwrap();
    cmd.arg("--output-dir")
        .arg(tempdir.path())
        .arg("not-a-url-token")
        .assert()
        .success();

    let entries: Vec<_> = std::fs::read_dir(tempdir.path())
        .unwrap()
        .filter_map(Result::ok)
        .collect();

    assert!(
        entries
            .iter()
            .any(|entry| entry.file_name().to_string_lossy() == ".downloader"),
        "expected queue state directory directly under output dir"
    );
    assert!(
        entries.iter().all(|entry| !entry
            .file_name()
            .to_string_lossy()
            .contains("Climate-Research")),
        "non-project runs should not create project-named subfolders"
    );
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

/// Test that --no-color flag is accepted.
#[test]
fn test_binary_no_color_flag_accepted() {
    let mut cmd = Command::cargo_bin("downloader").unwrap();
    cmd.arg("--no-color").assert().success();
}

/// Test that NO_COLOR disables ANSI escape codes in emitted output.
#[test]
fn test_binary_no_color_env_disables_ansi_sequences() {
    let mut cmd = Command::cargo_bin("downloader").unwrap();
    let assert = cmd
        .env("NO_COLOR", "1")
        .env("RUST_LOG", "trace")
        .arg("--debug")
        .arg("not-a-url-token")
        .assert()
        .success();
    let output = assert.get_output();
    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        !combined.contains("\u{1b}["),
        "did not expect ANSI escape sequences when NO_COLOR is set: {combined}"
    );
}

/// Test that TERM=dumb forces plain-text output without ANSI escapes.
#[test]
fn test_binary_dumb_terminal_disables_ansi_sequences() {
    let mut cmd = Command::cargo_bin("downloader").unwrap();
    let assert = cmd
        .env("TERM", "dumb")
        .env("RUST_LOG", "trace")
        .arg("--debug")
        .arg("not-a-url-token")
        .assert()
        .success();
    let output = assert.get_output();
    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        !combined.contains("\u{1b}["),
        "did not expect ANSI escape sequences when TERM=dumb: {combined}"
    );
}

/// Test that dry-run mode prints the explicit completion message.
#[test]
fn test_binary_dry_run_prints_explicit_completion_message() {
    let tempdir = TempDir::new().unwrap();
    let mut cmd = Command::cargo_bin("downloader").unwrap();
    cmd.arg("--output-dir")
        .arg(tempdir.path())
        .arg("--dry-run")
        .arg("not-a-url-token")
        .assert()
        .success()
        .stdout(predicate::str::contains("Dry run - no files downloaded"));
}

/// Test that dry-run mode resolves direct URL input for preview output.
#[test]
fn test_binary_dry_run_shows_resolved_url_preview() {
    let tempdir = TempDir::new().unwrap();
    let mut cmd = Command::cargo_bin("downloader").unwrap();
    cmd.arg("--output-dir")
        .arg(tempdir.path())
        .arg("--dry-run")
        .arg("https://example.com/paper.pdf")
        .assert()
        .success()
        .stdout(predicate::str::contains("[resolved][URL]"))
        .stdout(predicate::str::contains("https://example.com/paper.pdf"))
        .stdout(predicate::str::contains("Dry run - no files downloaded"));
}

/// Test that dry-run mode does not create queue DB artifacts.
#[test]
fn test_binary_dry_run_does_not_create_queue_db() {
    let tempdir = TempDir::new().unwrap();
    let db_path = tempdir.path().join(".downloader").join("queue.db");

    let mut cmd = Command::cargo_bin("downloader").unwrap();
    cmd.arg("--output-dir")
        .arg(tempdir.path())
        .arg("-n")
        .arg("https://example.com/paper.pdf")
        .assert()
        .success()
        .stdout(predicate::str::contains("Dry run - no files downloaded"));

    assert!(
        !db_path.exists(),
        "dry-run should not create queue DB at {:?}",
        db_path
    );
}

/// Test that dry-run does not create queue DB artifacts even with project scoping.
#[test]
fn test_binary_dry_run_project_mode_does_not_create_queue_db() {
    let tempdir = TempDir::new().unwrap();
    let db_path = tempdir
        .path()
        .join("Climate-Research")
        .join(".downloader")
        .join("queue.db");

    let mut cmd = Command::cargo_bin("downloader").unwrap();
    cmd.arg("--output-dir")
        .arg(tempdir.path())
        .arg("--project")
        .arg("Climate Research")
        .arg("--dry-run")
        .arg("https://example.com/paper.pdf")
        .assert()
        .success()
        .stdout(predicate::str::contains("Dry run - no files downloaded"));

    assert!(
        !db_path.exists(),
        "dry-run with --project should not create queue DB at {:?}",
        db_path
    );
}

/// Test that dry-run keeps cookie/stdin conflict guardrails intact.
#[test]
fn test_binary_dry_run_rejects_dual_stdin_cookie_and_url_usage() {
    let mut cmd = Command::cargo_bin("downloader").unwrap();
    cmd.args(["--dry-run", "--cookies", "-"])
        .write_stdin(".example.com\tTRUE\t/\tFALSE\t4102444800\tsession\tabc123\n")
        .assert()
        .failure()
        .stderr(predicate::str::contains(
            "Cannot read both cookies and URLs from stdin",
        ));
}

/// Test that piped stdin with no valid URLs exits cleanly.
#[test]
fn test_binary_stdin_no_urls_exits_cleanly() {
    let mut cmd = Command::cargo_bin("downloader").unwrap();
    cmd.write_stdin("no urls here, just text")
        .assert()
        .success();
}

/// Test that positional args and piped stdin are combined into one parse pass.
#[test]
fn test_binary_combines_positional_and_stdin_inputs() {
    let tempdir = TempDir::new().unwrap();
    let mut cmd = Command::cargo_bin("downloader").unwrap();
    let assert = cmd
        .arg("--output-dir")
        .arg(tempdir.path())
        .arg("-q")
        .arg("Smith, J. (2024). Argument Input. Journal.")
        .write_stdin("Doe, A. (2024). Stdin Input. Journal.")
        .assert()
        .failure();

    let output = assert.get_output();
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("All parsed items failed URL resolution (2/2)"),
        "expected both positional and stdin inputs to be parsed, got: {stderr}"
    );
}

/// Test that empty stdin yields explicit guidance when no prior queue state exists.
#[test]
fn test_binary_empty_stdin_shows_helpful_guidance() {
    let tempdir = TempDir::new().unwrap();
    let mut cmd = Command::cargo_bin("downloader").unwrap();
    let assert = cmd
        .arg("--output-dir")
        .arg(tempdir.path())
        .env("RUST_LOG", "info")
        .write_stdin("")
        .assert()
        .success();

    let output = assert.get_output();
    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        combined.contains("Received empty stdin input"),
        "expected explicit empty-stdin guidance, got: {combined}"
    );
}

/// Test that empty stdin guidance does not short-circuit resume-aware paths.
#[test]
fn test_binary_empty_stdin_with_prior_state_does_not_use_empty_guidance() {
    let tempdir = TempDir::new().unwrap();

    let mut bootstrap = Command::cargo_bin("downloader").unwrap();
    bootstrap
        .arg("--output-dir")
        .arg(tempdir.path())
        .env("RUST_LOG", "info")
        .arg("not-a-url-token")
        .assert()
        .success();

    let mut cmd = Command::cargo_bin("downloader").unwrap();
    let assert = cmd
        .arg("--output-dir")
        .arg(tempdir.path())
        .env("RUST_LOG", "info")
        .write_stdin("")
        .assert()
        .success();

    let output = assert.get_output();
    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        !combined.contains("Received empty stdin input"),
        "did not expect empty-stdin guidance when prior state exists: {combined}"
    );
}

/// Test that malformed stdin URL-like input exits cleanly (no crash).
#[test]
fn test_binary_stdin_with_invalid_domain_exits_cleanly() {
    let mut cmd = Command::cargo_bin("downloader").unwrap();
    // Use non-URL stdin that parser rejects before any network client
    // initialization. This keeps the test deterministic in sandboxed CI.
    cmd.write_stdin("not-a-url-token")
        .arg("-q")
        .arg("-r")
        .arg("0")
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

/// Test that malformed-only input still surfaces skipped diagnostics.
#[test]
fn test_binary_malformed_input_surfaces_skipped_output() {
    let mut cmd = Command::cargo_bin("downloader").unwrap();
    cmd.write_stdin("@article{bad, title={Broken}, year={2024}")
        .arg("-v")
        .assert()
        .success()
        .stderr(predicate::str::contains("Skipped unrecognized input"));
}

/// Test that fully unresolved parsed input returns a failure instead of silent success.
#[test]
fn test_binary_all_resolver_failures_exit_nonzero() {
    let tempdir = TempDir::new().unwrap();
    let config_home = tempdir.path().join("xdg-config");
    let mut cmd = Command::cargo_bin("downloader").unwrap();
    cmd.arg("--output-dir")
        .arg(tempdir.path())
        .env("XDG_CONFIG_HOME", &config_home)
        .arg("-q")
        .write_stdin("Smith, J. (2024). Paper Title. Journal Name, 1(2), 3-4.")
        .assert()
        .failure()
        .stderr(predicate::str::contains(
            "All parsed items failed URL resolution",
        ));
}

/// Test that `auth capture` accepts Netscape cookie data from stdin.
#[test]
fn test_auth_capture_accepts_netscape_input() {
    let mut cmd = Command::cargo_bin("downloader").unwrap();
    cmd.args(["auth", "capture"])
        .env("RUST_LOG", "info")
        .write_stdin(".example.com\tTRUE\t/\tFALSE\t4102444800\tsession\tabc123\n")
        .assert()
        .success()
        .stderr(predicate::str::contains(
            "Install a cookie export extension",
        ))
        .stderr(predicate::str::contains("Cookies captured for 1 domains"));
}

/// Test that `auth` without subcommand fails with clap usage guidance.
#[test]
fn test_auth_without_subcommand_exits_nonzero() {
    let mut cmd = Command::cargo_bin("downloader").unwrap();
    cmd.args(["auth"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("Usage:"))
        .stderr(predicate::str::contains("auth <COMMAND>"));
}

/// Test that unknown flags under `auth capture` fail through clap validation.
#[test]
fn test_auth_capture_unknown_flag_exits_nonzero() {
    let mut cmd = Command::cargo_bin("downloader").unwrap();
    cmd.args(["auth", "capture", "--unknown"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("unexpected argument '--unknown'"));
}

/// Test that misplaced auth namespace after download flags is rejected.
#[test]
fn test_misplaced_auth_namespace_after_download_flag_exits_nonzero() {
    let mut cmd = Command::cargo_bin("downloader").unwrap();
    cmd.args(["-q", "auth", "clear"])
        .assert()
        .failure()
        .stderr(predicate::str::contains(
            "Auth commands must be invoked as subcommands",
        ));
}

/// Test that misplaced auth namespace with missing subcommand is rejected.
#[test]
fn test_misplaced_auth_namespace_missing_subcommand_exits_nonzero() {
    let mut cmd = Command::cargo_bin("downloader").unwrap();
    cmd.args(["-q", "auth"])
        .assert()
        .failure()
        .stderr(predicate::str::contains(
            "Auth commands must be invoked as subcommands",
        ));
}

/// Regression guard: misplaced auth namespace with unknown token is rejected.
#[test]
fn test_misplaced_auth_namespace_unknown_token_exits_nonzero() {
    let mut cmd = Command::cargo_bin("downloader").unwrap();
    cmd.args(["-q", "auth", "foo"])
        .assert()
        .failure()
        .stderr(predicate::str::contains(
            "Auth commands must be invoked as subcommands",
        ));
}

/// Regression guard: misplaced auth namespace with help-like token is rejected.
#[test]
fn test_misplaced_auth_namespace_help_like_token_exits_nonzero() {
    let mut cmd = Command::cargo_bin("downloader").unwrap();
    cmd.args(["-q", "auth", "help"])
        .assert()
        .failure()
        .stderr(predicate::str::contains(
            "Auth commands must be invoked as subcommands",
        ));
}

/// Regression guard: misplaced auth namespace takes precedence over save-cookie download checks.
#[test]
fn test_misplaced_auth_namespace_precedes_save_cookie_validation() {
    let mut cmd = Command::cargo_bin("downloader").unwrap();
    cmd.args(["-q", "auth", "capture", "--save-cookies"])
        .assert()
        .failure()
        .stderr(predicate::str::contains(
            "Auth commands must be invoked as subcommands",
        ))
        .stderr(predicate::str::contains("--save-cookies requires --cookies FILE").not());
}

/// Regression guard: mixed-case auth token is still treated as auth namespace misuse.
#[test]
fn test_misplaced_auth_namespace_mixed_case_token_exits_nonzero() {
    let mut cmd = Command::cargo_bin("downloader").unwrap();
    cmd.args(["-q", "Auth", "clear"])
        .assert()
        .failure()
        .stderr(predicate::str::contains(
            "Auth commands must be invoked as subcommands",
        ));
}

/// Regression guard: download mode still enforces strict save-cookies pairing.
#[test]
fn test_download_mode_save_cookies_without_cookies_file_exits_nonzero() {
    let mut cmd = Command::cargo_bin("downloader").unwrap();
    cmd.args(["--save-cookies", "not-a-url-token"])
        .assert()
        .failure()
        .stderr(predicate::str::contains(
            "--save-cookies requires --cookies FILE",
        ));
}

/// Test that auth clear rejects download-only save flag through clap parsing.
#[test]
fn test_auth_clear_save_cookies_flag_exits_nonzero() {
    let mut cmd = Command::cargo_bin("downloader").unwrap();
    cmd.args(["auth", "clear", "--save-cookies"])
        .assert()
        .failure()
        .stderr(predicate::str::contains(
            "unexpected argument '--save-cookies'",
        ));
}

/// Test that `auth capture` accepts JSON cookie exports from stdin.
#[test]
fn test_auth_capture_accepts_json_input() {
    let mut cmd = Command::cargo_bin("downloader").unwrap();
    cmd.args(["auth", "capture"])
        .env("RUST_LOG", "info")
        .write_stdin(
            r#"[{"domain":".example.com","name":"sid","value":"abc","path":"/","secure":true,"expirationDate":4102444800}]"#,
        )
        .assert()
        .success()
        .stderr(predicate::str::contains("Cookies captured for 1 domains"));
}

/// Test that expired-only cookie capture input fails validation.
#[test]
fn test_auth_capture_expired_input_fails() {
    let mut cmd = Command::cargo_bin("downloader").unwrap();
    cmd.args(["auth", "capture"])
        .write_stdin(".example.com\tTRUE\t/\tFALSE\t1\tsession\texpired\n")
        .assert()
        .failure()
        .stderr(predicate::str::contains("Cookie capture failed"));
}

/// Test that `auth capture --save-cookies` persists encrypted cookies to config dir.
#[test]
fn test_auth_capture_save_cookies_persists_file() {
    let tempdir = TempDir::new().unwrap();
    let config_home = tempdir.path().join("xdg-config");
    let expected_path = config_home.join("downloader").join("cookies.enc");

    let mut cmd = Command::cargo_bin("downloader").unwrap();
    cmd.env("XDG_CONFIG_HOME", &config_home)
        .env("DOWNLOADER_MASTER_KEY", "test-master-key")
        .env("RUST_LOG", "info")
        .args(["auth", "capture", "--save-cookies"])
        .write_stdin(".example.com\tTRUE\t/\tFALSE\t4102444800\tsession\tabc123\n")
        .assert()
        .success()
        .stderr(predicate::str::contains("Saved encrypted cookies"));

    assert!(
        expected_path.exists(),
        "expected persisted file at {:?}",
        expected_path
    );
}

/// Test that `auth clear` removes persisted cookies from config dir.
#[test]
fn test_auth_clear_removes_persisted_file() {
    let tempdir = TempDir::new().unwrap();
    let config_home = tempdir.path().join("xdg-config");
    let cookie_dir = config_home.join("downloader");
    std::fs::create_dir_all(&cookie_dir).unwrap();
    let cookie_file = cookie_dir.join("cookies.enc");
    std::fs::write(&cookie_file, b"dummy").unwrap();

    let mut cmd = Command::cargo_bin("downloader").unwrap();
    cmd.env("XDG_CONFIG_HOME", &config_home)
        .env("DOWNLOADER_MASTER_KEY", "test-master-key")
        .env("RUST_LOG", "info")
        .args(["auth", "clear"])
        .assert()
        .success()
        .stderr(predicate::str::contains("Cleared persisted auth cookies"));

    assert!(
        !cookie_file.exists(),
        "expected persisted cookie file to be removed"
    );
}

/// Test that persisted cookies are automatically loaded on subsequent runs.
#[test]
fn test_persisted_cookies_auto_loaded_in_download_mode() {
    let tempdir = TempDir::new().unwrap();
    let config_home = tempdir.path().join("xdg-config");

    // First run: persist cookies via auth capture.
    let mut capture_cmd = Command::cargo_bin("downloader").unwrap();
    capture_cmd
        .env("XDG_CONFIG_HOME", &config_home)
        .env("DOWNLOADER_MASTER_KEY", "test-master-key")
        .args(["auth", "capture", "--save-cookies"])
        .write_stdin(".example.com\tTRUE\t/\tFALSE\t4102444800\tsession\tabc123\n")
        .assert()
        .success();

    // Second run: no --cookies flag, but persisted cookies should be loaded.
    let mut run_cmd = Command::cargo_bin("downloader").unwrap();
    run_cmd
        .env("XDG_CONFIG_HOME", &config_home)
        .env("DOWNLOADER_MASTER_KEY", "test-master-key")
        .env("RUST_LOG", "info")
        .arg("not-a-url-token")
        .assert()
        .success()
        .stderr(predicate::str::contains(
            "Loaded encrypted persisted cookies",
        ));
}

// ==================== Sidecar Flag E2E Tests ====================

/// Test that `--sidecar` flag is accepted by the CLI without error.
#[test]
fn test_cli_sidecar_flag_accepted_without_error() {
    let mut cmd = Command::cargo_bin("downloader").unwrap();
    cmd.arg("--sidecar")
        .assert()
        // no input → exits with code 0 (no-input guidance path)
        .success();
}

/// Test that `--sidecar` appears in help output.
#[test]
fn test_cli_sidecar_flag_appears_in_help() {
    let mut cmd = Command::cargo_bin("downloader").unwrap();
    cmd.arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("sidecar"));
}
