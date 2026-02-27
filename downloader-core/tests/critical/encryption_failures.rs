//! Phase 3 (P0): Keychain access failures, key rotation.
//! Corrupted cookies.enc, wrong key; assert clean error and no plaintext leak.

use std::env;
use std::fs;

use downloader_core::CookieLine;
use downloader_core::auth::{StorageError, load_persisted_cookies, store_persisted_cookies};
use tempfile::TempDir;

fn sample_cookie() -> CookieLine {
    CookieLine::new(
        ".example.com".to_string(),
        true,
        "/".to_string(),
        true,
        4_102_444_800,
        "sid".to_string(),
        "secret".to_string(),
    )
}

#[test]
#[ignore] // requires env isolation (XDG_CONFIG_HOME); run with --ignored
fn p0_corrupted_encrypted_file_returns_error_not_panic() {
    let temp_dir = TempDir::new().expect("temp dir");
    let config_dir = temp_dir.path().join("downloader");
    fs::create_dir_all(&config_dir).expect("mkdir");
    // SAFETY: test isolation; we restore vars at end
    unsafe {
        env::set_var("XDG_CONFIG_HOME", temp_dir.path());
        env::set_var("DOWNLOADER_MASTER_KEY", "test-key-for-corruption");
    }

    let path = config_dir.join("cookies.enc");
    fs::write(&path, b"invalid encrypted payload").expect("write");

    let result = load_persisted_cookies();

    unsafe {
        env::remove_var("XDG_CONFIG_HOME");
        env::remove_var("DOWNLOADER_MASTER_KEY");
    }

    assert!(result.is_err());
    let err = result.unwrap_err();
    match &err {
        StorageError::InvalidPayload | StorageError::DecryptionFailed => {}
        _ => panic!("expected InvalidPayload or DecryptionFailed, got {:?}", err),
    }
    let msg = err.to_string();
    assert!(
        !msg.contains("secret") && !msg.contains("sid"),
        "error message must not leak cookie data"
    );
}

#[test]
#[ignore] // requires env isolation (XDG_CONFIG_HOME); run with --ignored
fn p0_store_and_load_with_master_key_roundtrip() {
    let temp_dir = TempDir::new().expect("temp dir");
    let config_dir = temp_dir.path().join("downloader");
    fs::create_dir_all(&config_dir).expect("mkdir");
    // SAFETY: test isolation; we restore vars at end
    unsafe {
        env::set_var("XDG_CONFIG_HOME", temp_dir.path());
        env::set_var("DOWNLOADER_MASTER_KEY", "roundtrip-key");
    }

    let cookies = vec![sample_cookie()];
    store_persisted_cookies(&cookies).expect("store");
    let loaded = load_persisted_cookies().expect("load").expect("some");

    unsafe {
        env::remove_var("XDG_CONFIG_HOME");
        env::remove_var("DOWNLOADER_MASTER_KEY");
    }

    assert_eq!(loaded.len(), 1);
    assert_eq!(loaded[0].domain, ".example.com");
    assert_eq!(loaded[0].value(), "secret");
}
