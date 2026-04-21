use std::fs::{self, File, OpenOptions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

#[cfg(windows)]
use std::os::windows::ffi::OsStrExt;
#[cfg(windows)]
use std::time::Duration;

use serde::Serialize;
#[cfg(windows)]
use windows_sys::Win32::Foundation::{
    ERROR_ACCESS_DENIED, ERROR_FILE_NOT_FOUND, ERROR_LOCK_VIOLATION, ERROR_PATH_NOT_FOUND,
    ERROR_SHARING_VIOLATION,
};
#[cfg(windows)]
use windows_sys::Win32::Storage::FileSystem::ReplaceFileW;

static TEMP_SEQ: AtomicU64 = AtomicU64::new(0);

fn temp_path_for(target: &Path) -> io::Result<PathBuf> {
    let parent = target.parent().unwrap_or_else(|| Path::new("."));
    let filename = target
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("artifact");
    let seq = TEMP_SEQ.fetch_add(1, Ordering::Relaxed);
    let pid = std::process::id();
    Ok(parent.join(format!(".{filename}.tmp-{pid}-{seq}")))
}

#[cfg(unix)]
fn fsync_dir(parent: &Path) -> io::Result<()> {
    File::open(parent)?.sync_all()
}

#[cfg(not(unix))]
fn fsync_dir(_parent: &Path) -> io::Result<()> {
    Ok(())
}

#[cfg(windows)]
fn to_windows_wide(path: &Path) -> Vec<u16> {
    path.as_os_str()
        .encode_wide()
        .chain(std::iter::once(0))
        .collect()
}

#[cfg(windows)]
fn replace_file_windows(destination: &Path, replacement: &Path) -> io::Result<()> {
    let destination_wide = to_windows_wide(destination);
    let replacement_wide = to_windows_wide(replacement);
    let replaced = unsafe {
        ReplaceFileW(
            destination_wide.as_ptr(),
            replacement_wide.as_ptr(),
            std::ptr::null(),
            0,
            std::ptr::null_mut(),
            std::ptr::null_mut(),
        )
    };
    if replaced == 0 {
        Err(io::Error::last_os_error())
    } else {
        Ok(())
    }
}

#[cfg(windows)]
fn is_windows_transient_replace_error(err: &io::Error) -> bool {
    matches!(
        err.raw_os_error(),
        Some(ERROR_SHARING_VIOLATION as i32)
            | Some(ERROR_ACCESS_DENIED as i32)
            | Some(ERROR_LOCK_VIOLATION as i32)
    )
}

#[cfg(windows)]
fn is_windows_missing_target_error(err: &io::Error) -> bool {
    matches!(
        err.raw_os_error(),
        Some(ERROR_FILE_NOT_FOUND as i32) | Some(ERROR_PATH_NOT_FOUND as i32)
    )
}

#[cfg(windows)]
fn replace_target_file(temp_path: &Path, path: &Path) -> io::Result<()> {
    const MAX_RETRIES: usize = 3;
    const RETRY_BACKOFF_MS: u64 = 25;

    let mut last_error: Option<io::Error> = None;
    for attempt in 0..=MAX_RETRIES {
        match replace_file_windows(path, temp_path) {
            Ok(()) => return Ok(()),
            Err(err) if is_windows_missing_target_error(&err) => {
                match fs::rename(temp_path, path) {
                    Ok(()) => return Ok(()),
                    Err(rename_err) if rename_err.kind() == io::ErrorKind::AlreadyExists => {
                        // Destination appeared between replace attempt and fallback rename.
                        last_error = Some(rename_err);
                    }
                    Err(rename_err)
                        if attempt < MAX_RETRIES
                            && is_windows_transient_replace_error(&rename_err) =>
                    {
                        std::thread::sleep(Duration::from_millis(RETRY_BACKOFF_MS));
                        last_error = Some(rename_err);
                    }
                    Err(rename_err) => return Err(rename_err),
                }
            }
            Err(err) if attempt < MAX_RETRIES && is_windows_transient_replace_error(&err) => {
                std::thread::sleep(Duration::from_millis(RETRY_BACKOFF_MS));
                last_error = Some(err);
            }
            Err(err) => return Err(err),
        }
    }

    Err(last_error
        .unwrap_or_else(|| io::Error::other("failed to replace target file after retries")))
}

#[cfg(not(windows))]
fn replace_target_file(temp_path: &Path, path: &Path) -> io::Result<()> {
    fs::rename(temp_path, path)
}

#[cfg(windows)]
fn flush_replaced_target(path: &Path) -> io::Result<()> {
    File::open(path)?.sync_all()
}

#[cfg(not(windows))]
fn flush_replaced_target(_path: &Path) -> io::Result<()> {
    Ok(())
}

pub fn atomic_write(path: &Path, bytes: &[u8]) -> io::Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }

    let parent = path.parent().unwrap_or_else(|| Path::new("."));
    let temp_path = temp_path_for(path)?;

    // Retry a few times if a stale temp file exists from an interrupted run.
    let mut file = None;
    for _ in 0..3 {
        match OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temp_path)
        {
            Ok(handle) => {
                file = Some(handle);
                break;
            }
            Err(err) if err.kind() == io::ErrorKind::AlreadyExists => {
                let _ = fs::remove_file(&temp_path);
            }
            Err(err) => return Err(err),
        }
    }

    let mut file = match file {
        Some(file) => file,
        None => {
            return Err(io::Error::new(
                io::ErrorKind::AlreadyExists,
                "failed to allocate atomic temp file",
            ));
        }
    };

    if let Err(err) = (|| -> io::Result<()> {
        file.write_all(bytes)?;
        file.flush()?;
        file.sync_all()?;
        Ok(())
    })() {
        let _ = fs::remove_file(&temp_path);
        return Err(err);
    }

    if let Err(err) = replace_target_file(&temp_path, path) {
        let _ = fs::remove_file(&temp_path);
        return Err(err);
    }

    flush_replaced_target(path)?;
    fsync_dir(parent)
}

pub fn atomic_write_json<T: Serialize>(path: &Path, value: &T) -> io::Result<()> {
    let bytes = serde_json::to_vec_pretty(value).map_err(io::Error::other)?;
    atomic_write(path, &bytes)
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn test_atomic_write_creates_new_file() {
        let temp = tempfile::TempDir::new().unwrap();
        let path = temp.path().join("artifact.txt");

        atomic_write(&path, b"first").unwrap();

        let stored = std::fs::read(&path).unwrap();
        assert_eq!(stored, b"first");
    }

    #[test]
    fn test_atomic_write_replaces_existing_file() {
        let temp = tempfile::TempDir::new().unwrap();
        let path = temp.path().join("artifact.txt");
        std::fs::write(&path, b"old").unwrap();

        atomic_write(&path, b"new").unwrap();

        let stored = std::fs::read(&path).unwrap();
        assert_eq!(stored, b"new");
    }

    #[cfg(windows)]
    #[test]
    fn test_replace_target_file_replaces_existing_destination() {
        let temp = tempfile::TempDir::new().unwrap();
        let path = temp.path().join("artifact.txt");
        let temp_path = temp.path().join("artifact.tmp");
        std::fs::write(&path, b"old").unwrap();
        std::fs::write(&temp_path, b"new").unwrap();

        replace_target_file(&temp_path, &path).unwrap();

        let stored = std::fs::read(&path).unwrap();
        assert_eq!(stored, b"new");
    }

    #[cfg(windows)]
    #[test]
    fn test_replace_target_file_succeeds_when_destination_missing() {
        let temp = tempfile::TempDir::new().unwrap();
        let path = temp.path().join("artifact.txt");
        let temp_path = temp.path().join("artifact.tmp");
        std::fs::write(&temp_path, b"new").unwrap();

        replace_target_file(&temp_path, &path).unwrap();

        let stored = std::fs::read(&path).unwrap();
        assert_eq!(stored, b"new");
    }

    #[cfg(windows)]
    #[test]
    fn test_windows_transient_error_codes_are_detected() {
        assert!(is_windows_transient_replace_error(
            &io::Error::from_raw_os_error(ERROR_SHARING_VIOLATION as i32,)
        ));
        assert!(is_windows_transient_replace_error(
            &io::Error::from_raw_os_error(ERROR_ACCESS_DENIED as i32,)
        ));
        assert!(is_windows_transient_replace_error(
            &io::Error::from_raw_os_error(ERROR_LOCK_VIOLATION as i32,)
        ));
    }
}
