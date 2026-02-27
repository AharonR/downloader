//! Secure cookie persistence with encrypted-at-rest storage.
//!
//! Cookie persistence is opt-in and writes encrypted data to:
//! `~/.config/downloader/cookies.enc` (or `$XDG_CONFIG_HOME/downloader/cookies.enc`).

use std::env;
use std::ffi::OsString;
use std::fs;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::path::{Path, PathBuf};

use chacha20poly1305::aead::{Aead, KeyInit};
use chacha20poly1305::{Key, XChaCha20Poly1305, XNonce};
use rand::RngCore;
use sha2::{Digest, Sha256};

use super::CookieLine;

const COOKIE_FILE_NAME: &str = "cookies.enc";
const KEYRING_SERVICE: &str = "downloader";
const KEYRING_ENTRY_NAME: &str = "cookie-master-key-v1";
const MAGIC: &[u8; 4] = b"DLC1";
const NONCE_LEN: usize = 24;
const KEY_LEN: usize = 32;

/// Errors for persisted cookie storage operations.
#[derive(Debug, thiserror::Error)]
pub enum StorageError {
    /// No suitable user config directory is available.
    #[error("unable to determine config directory (set XDG_CONFIG_HOME or HOME)")]
    ConfigDirUnavailable,
    /// Filesystem I/O failed.
    #[error(transparent)]
    Io(#[from] std::io::Error),
    /// Serialization/deserialization failed.
    #[error(transparent)]
    Json(#[from] serde_json::Error),
    /// Could not access keychain and no env fallback key was provided.
    #[error(
        "unable to access system keychain for cookie encryption key; set DOWNLOADER_MASTER_KEY or configure keychain access"
    )]
    KeychainUnavailable,
    /// Stored encrypted payload is malformed.
    #[error("persisted cookie payload is invalid")]
    InvalidPayload,
    /// Encryption failed.
    #[error("failed to encrypt persisted cookies")]
    EncryptionFailed,
    /// Decryption failed.
    #[error("failed to decrypt persisted cookies")]
    DecryptionFailed,
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
struct StoredCookie {
    domain: String,
    tailmatch: bool,
    path: String,
    secure: bool,
    expires: u64,
    name: String,
    value: String,
}

impl StoredCookie {
    fn from_cookie_line(cookie: &CookieLine) -> Self {
        Self {
            domain: cookie.domain.clone(),
            tailmatch: cookie.tailmatch,
            path: cookie.path.clone(),
            secure: cookie.secure,
            expires: cookie.expires,
            name: cookie.name.clone(),
            value: cookie.value().to_string(),
        }
    }

    fn into_cookie_line(self) -> CookieLine {
        CookieLine::new(
            self.domain,
            self.tailmatch,
            self.path,
            self.secure,
            self.expires,
            self.name,
            self.value,
        )
    }
}

/// Returns the default persisted cookie path (`~/.config/downloader/cookies.enc`).
///
/// # Errors
///
/// Returns [`StorageError::ConfigDirUnavailable`] if no usable config dir is found.
pub fn persisted_cookie_path() -> Result<PathBuf, StorageError> {
    Ok(default_config_dir()?.join(COOKIE_FILE_NAME))
}

/// Stores cookies encrypted at rest in the default cookie file location.
///
/// # Errors
///
/// Returns [`StorageError`] when key retrieval, encryption, or file writing fails.
pub fn store_persisted_cookies(cookies: &[CookieLine]) -> Result<PathBuf, StorageError> {
    let path = persisted_cookie_path()?;
    let key = load_or_create_key()?;
    store_persisted_cookies_with_key(cookies, &path, &key)?;
    Ok(path)
}

/// Loads and decrypts persisted cookies from disk.
///
/// Returns `Ok(None)` when no persisted cookie file exists.
///
/// # Errors
///
/// Returns [`StorageError`] when key retrieval, decryption, or parsing fails.
pub fn load_persisted_cookies() -> Result<Option<Vec<CookieLine>>, StorageError> {
    let path = persisted_cookie_path()?;
    if !path.exists() {
        return Ok(None);
    }

    let key = load_or_create_key()?;
    let cookies = load_persisted_cookies_with_key(&path, &key)?;
    Ok(Some(cookies))
}

/// Rotates the encryption key for persisted cookies.
///
/// This decrypts the existing cookies, generates a new key, and re-encrypts them.
///
/// # Errors
///
/// Returns [`StorageError`] if loading, clearing, or storing fails.
pub fn rotate_key() -> Result<(), StorageError> {
    if let Some(cookies) = load_persisted_cookies()? {
        clear_persisted_cookies()?;
        store_persisted_cookies(&cookies)?;
    }
    Ok(())
}

/// Removes persisted cookies and best-effort clears keychain key.
///
/// Returns `true` when the cookie file existed and was deleted.
///
/// # Errors
///
/// Returns [`StorageError`] when file removal fails.
pub fn clear_persisted_cookies() -> Result<bool, StorageError> {
    let path = persisted_cookie_path()?;
    let removed = if path.exists() {
        fs::remove_file(&path)?;
        true
    } else {
        false
    };

    if env::var_os("DOWNLOADER_MASTER_KEY").is_none() {
        let _ = delete_keychain_key();
    }

    Ok(removed)
}

fn default_config_dir() -> Result<PathBuf, StorageError> {
    resolve_config_dir(
        sanitize_env_path(env::var_os("XDG_CONFIG_HOME")),
        sanitize_env_path(env::var_os("HOME")),
        sanitize_env_path(env::var_os("APPDATA")),
    )
}

fn sanitize_env_path(value: Option<OsString>) -> Option<PathBuf> {
    let value = value?;
    if value.to_string_lossy().trim().is_empty() {
        return None;
    }

    Some(PathBuf::from(value))
}

fn resolve_config_dir(
    xdg_config_home: Option<PathBuf>,
    home: Option<PathBuf>,
    app_data: Option<PathBuf>,
) -> Result<PathBuf, StorageError> {
    if let Some(xdg) = xdg_config_home {
        return Ok(xdg.join("downloader"));
    }
    if let Some(home) = home {
        return Ok(home.join(".config").join("downloader"));
    }
    if let Some(app_data) = app_data {
        return Ok(app_data.join("downloader"));
    }

    Err(StorageError::ConfigDirUnavailable)
}

fn load_or_create_key() -> Result<String, StorageError> {
    if let Some(from_env) = env::var_os("DOWNLOADER_MASTER_KEY") {
        let key = from_env.to_string_lossy().trim().to_string();
        if !key.is_empty() {
            return Ok(key);
        }
    }

    let entry = safe_keyring_entry()?;

    match safe_keyring_get_password(&entry) {
        Ok(existing) if !existing.trim().is_empty() => Ok(existing),
        _ => {
            let generated = generate_key_material();
            safe_keyring_set_password(&entry, &generated)?;
            Ok(generated)
        }
    }
}

fn delete_keychain_key() -> Result<(), StorageError> {
    let entry = safe_keyring_entry()?;
    let _ = safe_keyring_delete_credential(&entry);
    Ok(())
}

fn safe_keyring_entry() -> Result<keyring::Entry, StorageError> {
    catch_unwind(|| keyring::Entry::new(KEYRING_SERVICE, KEYRING_ENTRY_NAME))
        .map_err(|_| StorageError::KeychainUnavailable)?
        .map_err(|_| StorageError::KeychainUnavailable)
}

fn safe_keyring_get_password(entry: &keyring::Entry) -> Result<String, StorageError> {
    catch_unwind(AssertUnwindSafe(|| entry.get_password()))
        .map_err(|_| StorageError::KeychainUnavailable)?
        .map_err(|_| StorageError::KeychainUnavailable)
}

fn safe_keyring_set_password(entry: &keyring::Entry, password: &str) -> Result<(), StorageError> {
    catch_unwind(AssertUnwindSafe(|| entry.set_password(password)))
        .map_err(|_| StorageError::KeychainUnavailable)?
        .map_err(|_| StorageError::KeychainUnavailable)
}

fn safe_keyring_delete_credential(entry: &keyring::Entry) -> Result<(), StorageError> {
    catch_unwind(AssertUnwindSafe(|| entry.delete_credential()))
        .map_err(|_| StorageError::KeychainUnavailable)?
        .map_err(|_| StorageError::KeychainUnavailable)
}

fn generate_key_material() -> String {
    let mut bytes = [0_u8; KEY_LEN];
    rand::thread_rng().fill_bytes(&mut bytes);
    hex_encode(&bytes)
}

fn hex_encode(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut out = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        out.push(char::from(HEX[usize::from(byte >> 4)]));
        out.push(char::from(HEX[usize::from(byte & 0x0f)]));
    }
    out
}

fn derive_key_bytes(key_material: &str) -> [u8; KEY_LEN] {
    let digest = Sha256::digest(key_material.as_bytes());
    let mut key = [0_u8; KEY_LEN];
    key.copy_from_slice(&digest[..KEY_LEN]);
    key
}

fn store_persisted_cookies_with_key(
    cookies: &[CookieLine],
    path: &Path,
    key_material: &str,
) -> Result<(), StorageError> {
    let stored = cookies
        .iter()
        .map(StoredCookie::from_cookie_line)
        .collect::<Vec<_>>();
    let plaintext = serde_json::to_vec(&stored)?;
    let encrypted = encrypt_bytes(&plaintext, key_material)?;
    write_encrypted_payload(path, &encrypted)?;
    Ok(())
}

fn write_encrypted_payload(path: &Path, payload: &[u8]) -> Result<(), StorageError> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }

    fs::write(path, payload)?;
    set_owner_only_permissions(path)?;
    Ok(())
}

#[cfg(unix)]
fn set_owner_only_permissions(path: &Path) -> Result<(), StorageError> {
    use std::os::unix::fs::PermissionsExt;

    let permissions = fs::Permissions::from_mode(0o600);
    fs::set_permissions(path, permissions)?;
    Ok(())
}

#[cfg(not(unix))]
fn set_owner_only_permissions(_path: &Path) -> Result<(), StorageError> {
    Ok(())
}

fn load_persisted_cookies_with_key(
    path: &Path,
    key_material: &str,
) -> Result<Vec<CookieLine>, StorageError> {
    let bytes = fs::read(path)?;
    let plaintext = decrypt_bytes(&bytes, key_material)?;
    let stored = serde_json::from_slice::<Vec<StoredCookie>>(&plaintext)?;
    Ok(stored
        .into_iter()
        .map(StoredCookie::into_cookie_line)
        .collect())
}

fn encrypt_bytes(plaintext: &[u8], key_material: &str) -> Result<Vec<u8>, StorageError> {
    let key_bytes = derive_key_bytes(key_material);
    let cipher = XChaCha20Poly1305::new(Key::from_slice(&key_bytes));

    let mut nonce = [0_u8; NONCE_LEN];
    rand::thread_rng().fill_bytes(&mut nonce);
    let nonce_ref = XNonce::from_slice(&nonce);

    let ciphertext = cipher
        .encrypt(nonce_ref, plaintext)
        .map_err(|_| StorageError::EncryptionFailed)?;

    let mut output = Vec::with_capacity(MAGIC.len() + NONCE_LEN + ciphertext.len());
    output.extend_from_slice(MAGIC);
    output.extend_from_slice(&nonce);
    output.extend_from_slice(&ciphertext);
    Ok(output)
}

fn decrypt_bytes(payload: &[u8], key_material: &str) -> Result<Vec<u8>, StorageError> {
    if payload.len() < MAGIC.len() + NONCE_LEN || &payload[..MAGIC.len()] != MAGIC {
        return Err(StorageError::InvalidPayload);
    }

    let key_bytes = derive_key_bytes(key_material);
    let cipher = XChaCha20Poly1305::new(Key::from_slice(&key_bytes));
    let nonce_start = MAGIC.len();
    let nonce_end = nonce_start + NONCE_LEN;
    let nonce = XNonce::from_slice(&payload[nonce_start..nonce_end]);
    let ciphertext = &payload[nonce_end..];

    cipher
        .decrypt(nonce, ciphertext)
        .map_err(|_| StorageError::DecryptionFailed)
}

#[cfg(test)]
mod tests {
    use std::ffi::OsString;

    use tempfile::TempDir;

    use super::*;

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
    fn test_store_and_load_round_trip_with_explicit_key() {
        let tempdir = TempDir::new().unwrap();
        let path = tempdir.path().join("cookies.enc");
        let cookie = sample_cookie();

        store_persisted_cookies_with_key(&[cookie], &path, "test-key").unwrap();
        let loaded = load_persisted_cookies_with_key(&path, "test-key").unwrap();
        assert_eq!(loaded.len(), 1);
        assert_eq!(loaded[0].domain, ".example.com");
        assert_eq!(loaded[0].name, "sid");
        assert_eq!(loaded[0].value(), "secret");
    }

    #[test]
    fn test_load_with_wrong_key_fails() {
        let tempdir = TempDir::new().unwrap();
        let path = tempdir.path().join("cookies.enc");
        store_persisted_cookies_with_key(&[sample_cookie()], &path, "key-a").unwrap();

        let result = load_persisted_cookies_with_key(&path, "key-b");
        assert!(matches!(result, Err(StorageError::DecryptionFailed)));
    }

    #[test]
    fn test_invalid_payload_fails() {
        let tempdir = TempDir::new().unwrap();
        let path = tempdir.path().join("cookies.enc");
        fs::write(&path, b"not-encrypted-data").unwrap();

        let result = load_persisted_cookies_with_key(&path, "test-key");
        assert!(matches!(result, Err(StorageError::InvalidPayload)));
    }

    #[test]
    fn test_hex_encode_length() {
        let encoded = hex_encode(&[1_u8, 255_u8, 16_u8]);
        assert_eq!(encoded.len(), 6);
        assert_eq!(encoded, "01ff10");
    }

    #[test]
    fn test_sanitize_env_path_rejects_blank_values() {
        assert!(sanitize_env_path(Some(OsString::from(""))).is_none());
        assert!(sanitize_env_path(Some(OsString::from("   "))).is_none());
    }

    #[test]
    fn test_resolve_config_dir_prefers_xdg_over_home() {
        let resolved = resolve_config_dir(
            Some(PathBuf::from("/tmp/xdg")),
            Some(PathBuf::from("/tmp/home")),
            Some(PathBuf::from("/tmp/appdata")),
        )
        .unwrap();
        assert_eq!(resolved, PathBuf::from("/tmp/xdg/downloader"));
    }

    #[test]
    fn test_resolve_config_dir_falls_back_to_home() {
        let resolved = resolve_config_dir(
            None,
            Some(PathBuf::from("/tmp/home")),
            Some(PathBuf::from("/tmp/appdata")),
        )
        .unwrap();
        assert_eq!(resolved, PathBuf::from("/tmp/home/.config/downloader"));
    }

    #[test]
    fn test_resolve_config_dir_falls_back_to_appdata() {
        let resolved = resolve_config_dir(None, None, Some(PathBuf::from("/tmp/appdata"))).unwrap();
        assert_eq!(resolved, PathBuf::from("/tmp/appdata/downloader"));
    }

    #[test]
    fn test_resolve_config_dir_errors_when_all_sources_missing() {
        let result = resolve_config_dir(None, None, None);
        assert!(matches!(result, Err(StorageError::ConfigDirUnavailable)));
    }

    #[cfg(unix)]
    #[test]
    fn test_store_sets_owner_only_permissions() {
        use std::os::unix::fs::PermissionsExt;

        let tempdir = TempDir::new().unwrap();
        let path = tempdir.path().join("cookies.enc");
        store_persisted_cookies_with_key(&[sample_cookie()], &path, "test-key").unwrap();

        let mode = fs::metadata(&path).unwrap().permissions().mode() & 0o777;
        assert_eq!(mode, 0o600);
    }

    #[test]
    fn test_rotation_logic_with_explicit_keys() {
        let tempdir = TempDir::new().unwrap();
        let path = tempdir.path().join("cookies.enc");
        let cookie = sample_cookie();
        let key1 = "key-1";

        // Initial store
        store_persisted_cookies_with_key(&[cookie.clone()], &path, key1).unwrap();
        let initial_data = fs::read(&path).unwrap();

        // Simulate rotate: Load with old key
        let loaded = load_persisted_cookies_with_key(&path, key1).unwrap();

        // Clear (simulated by deleting file)
        fs::remove_file(&path).unwrap();

        // Re-store with new key
        let key2 = "key-2";
        store_persisted_cookies_with_key(&loaded, &path, key2).unwrap();

        let new_data = fs::read(&path).unwrap();

        // Verify data changed (re-encrypted)
        assert_ne!(initial_data, new_data);

        // Verify can load with new key
        let reloaded = load_persisted_cookies_with_key(&path, key2).unwrap();
        assert_eq!(reloaded[0].domain, cookie.domain);

        // Verify cannot load with old key
        let result = load_persisted_cookies_with_key(&path, key1);
        assert!(result.is_err());
    }
}
