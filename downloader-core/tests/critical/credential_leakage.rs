//! Phase 3 (P0): Memory dumps, log exposure.
//! Assert cookies/keys not in error messages or logs.

use std::env;

use downloader_core::auth::load_persisted_cookies;

#[test]
fn p0_storage_error_does_not_contain_secret() {
    let temp_dir = tempfile::TempDir::new().expect("temp dir");
    let config_dir = temp_dir.path().join("downloader");
    std::fs::create_dir_all(&config_dir).expect("mkdir");
    // SAFETY: test isolation; we restore vars at end
    unsafe {
        env::set_var("XDG_CONFIG_HOME", temp_dir.path());
        env::set_var("DOWNLOADER_MASTER_KEY", "secret-key-123");
    }

    std::fs::write(config_dir.join("cookies.enc"), b"garbage").expect("write");

    let result = load_persisted_cookies();

    unsafe {
        env::remove_var("XDG_CONFIG_HOME");
        env::remove_var("DOWNLOADER_MASTER_KEY");
    }

    assert!(result.is_err());
    let err = result.unwrap_err();
    let msg = err.to_string();
    assert!(
        !msg.contains("secret-key-123"),
        "StorageError must not contain master key: {}",
        msg
    );
}
