//! Phase 1 (P0): SQLite corruption, WAL issues.
//! Open corrupted DB file (via utility), expect DbError; optionally check WAL on good DB.

use downloader_core::Database;
use downloader_core::db::DbError;

use crate::support::critical_utils::{corrupted_database, truncated_database};

#[tokio::test]
async fn p0_corrupted_database_open_fails() {
    let (_temp, path) = corrupted_database();

    let result = Database::new(path.as_path()).await;

    assert!(result.is_err(), "opening corrupted DB should fail");
    let err = result.unwrap_err();
    assert!(
        matches!(&err, DbError::Connection(_) | DbError::Migration(_)),
        "expected DbError::Connection or DbError::Migration, got {:?}",
        err
    );
}

#[tokio::test]
#[ignore] // needs file I/O; run with --ignored in nightly
async fn p0_truncated_database_open_fails() {
    let (_temp, path) = truncated_database().await;

    let result = Database::new(path.as_path()).await;

    assert!(result.is_err(), "opening truncated DB should fail");
}

#[tokio::test]
async fn p0_valid_database_wal_enabled() {
    let temp_dir = tempfile::TempDir::new().expect("temp dir");
    let db_path = temp_dir.path().join("good.db");

    let db = Database::new(&db_path).await.expect("create valid db");
    let wal = db.is_wal_enabled().await.expect("pragma journal_mode");
    assert!(wal, "WAL mode should be enabled on file-based DB");
}
