//! Project-local durable dedup registry.
//!
//! The registry is stored at `<output_dir>/.downloader/downloaded-registry.v1.json` and is the
//! source of truth for "already downloaded" checks.

use std::collections::HashMap;
use std::fs::{self, OpenOptions};
use std::io;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use fs2::FileExt;
use serde::{Deserialize, Serialize};
use tracing::warn;
use url::Url;

use crate::atomic_write::atomic_write_json;

const REGISTRY_SCHEMA_VERSION: u32 = 1;
const REGISTRY_FILENAME: &str = "downloaded-registry.v1.json";
const REGISTRY_LOCK_FILENAME: &str = "downloaded-registry.v1.lock";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegistryItem {
    pub dedup_key: String,
    pub canonical_doi: Option<String>,
    pub canonical_url: String,
    pub relative_path: String,
    pub first_seen_at: String,
    pub last_seen_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct RegistryFile {
    schema_version: u32,
    project_key: String,
    items: Vec<RegistryItem>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RegistryLookup {
    Hit { dedup_key: String, path: PathBuf },
    Miss,
    StaleRecovered,
}

#[derive(Debug)]
pub struct DownloadedRegistry {
    path: PathBuf,
    project_key: String,
    items: HashMap<String, RegistryItem>,
    dirty: bool,
    lock_file: std::fs::File,
}

impl DownloadedRegistry {
    #[must_use]
    pub fn path_for_output_dir(output_dir: &Path) -> PathBuf {
        output_dir.join(".downloader").join(REGISTRY_FILENAME)
    }

    #[must_use]
    pub fn lock_path_for_output_dir(output_dir: &Path) -> PathBuf {
        output_dir.join(".downloader").join(REGISTRY_LOCK_FILENAME)
    }

    #[must_use]
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Loads a project registry from disk or creates an empty in-memory registry.
    ///
    /// Corrupt or incompatible files are ignored and replaced on the next `save_if_dirty`.
    pub fn load(output_dir: &Path, project_key: &str) -> std::io::Result<Self> {
        let lock_file = Self::acquire_project_lock(output_dir)?;
        let path = Self::path_for_output_dir(output_dir);
        if !path.exists() {
            return Ok(Self {
                path,
                project_key: project_key.to_string(),
                items: HashMap::new(),
                dirty: false,
                lock_file,
            });
        }

        let mut dirty = false;
        let items = match std::fs::read_to_string(&path) {
            Ok(raw) => match serde_json::from_str::<RegistryFile>(&raw) {
                Ok(file)
                    if file.schema_version == REGISTRY_SCHEMA_VERSION
                        && file.project_key == project_key =>
                {
                    file.items
                        .into_iter()
                        .map(|item| (item.dedup_key.clone(), item))
                        .collect()
                }
                Ok(file) => {
                    warn!(
                        path = %path.display(),
                        expected_schema = REGISTRY_SCHEMA_VERSION,
                        found_schema = file.schema_version,
                        expected_project = %project_key,
                        found_project = %file.project_key,
                        "Registry schema/project mismatch; recreating registry"
                    );
                    dirty = true;
                    HashMap::new()
                }
                Err(err) => {
                    warn!(
                        path = %path.display(),
                        error = %err,
                        "Registry parse failed; recreating registry"
                    );
                    dirty = true;
                    HashMap::new()
                }
            },
            Err(err) => {
                warn!(
                    path = %path.display(),
                    error = %err,
                    "Registry read failed; recreating registry"
                );
                dirty = true;
                HashMap::new()
            }
        };

        Ok(Self {
            path,
            project_key: project_key.to_string(),
            items,
            dirty,
            lock_file,
        })
    }

    fn acquire_project_lock(output_dir: &Path) -> io::Result<std::fs::File> {
        let lock_path = Self::lock_path_for_output_dir(output_dir);
        if let Some(parent) = lock_path.parent() {
            fs::create_dir_all(parent)?;
        }
        let lock_file = OpenOptions::new()
            .create(true)
            .read(true)
            .write(true)
            .open(&lock_path)?;

        if let Err(err) = lock_file.try_lock_exclusive() {
            if err.kind() == io::ErrorKind::WouldBlock {
                return Err(io::Error::new(
                    io::ErrorKind::WouldBlock,
                    format!(
                        "registry lock is already held for project at {}",
                        output_dir.display()
                    ),
                ));
            }
            return Err(err);
        }

        Ok(lock_file)
    }

    #[must_use]
    pub fn is_dirty(&self) -> bool {
        self.dirty
    }

    /// Resolves whether an input should be treated as already downloaded.
    ///
    /// Priority: DOI key first, URL key second.
    /// Missing files mark keys stale and return [`RegistryLookup::StaleRecovered`].
    pub fn lookup(&mut self, output_dir: &Path, url: &str, doi: Option<&str>) -> RegistryLookup {
        let mut saw_stale = false;
        let mut keys = Vec::new();
        if let Some(doi_key) = dedup_key_for(doi, Some(url)) {
            keys.push(doi_key);
        }
        keys.push(dedup_key_for(None, Some(url)).unwrap_or_default());

        for key in keys.into_iter().filter(|key| !key.is_empty()) {
            let Some(entry) = self.items.get(&key).cloned() else {
                continue;
            };

            let mapped_path = resolve_mapped_path(output_dir, &entry.relative_path);
            if mapped_path.exists() {
                return RegistryLookup::Hit {
                    dedup_key: key,
                    path: mapped_path,
                };
            }

            self.items.remove(&key);
            self.dirty = true;
            saw_stale = true;
        }

        if saw_stale {
            RegistryLookup::StaleRecovered
        } else {
            RegistryLookup::Miss
        }
    }

    /// Records a successful download mapping for DOI and URL keys.
    pub fn record_success(
        &mut self,
        output_dir: &Path,
        url: &str,
        doi: Option<&str>,
        saved_path: &Path,
    ) {
        let now = unix_timestamp_string();
        let relative_path = saved_path
            .strip_prefix(output_dir)
            .ok()
            .and_then(|p| p.to_str())
            .map(ToString::to_string)
            .unwrap_or_else(|| saved_path.to_string_lossy().to_string());

        let canonical_doi = normalize_doi(doi.unwrap_or_default());
        let canonical_url = canonicalize_url(url);

        let mut upsert = |dedup_key: String| {
            let item = self
                .items
                .entry(dedup_key.clone())
                .or_insert_with(|| RegistryItem {
                    dedup_key: dedup_key.clone(),
                    canonical_doi: canonical_doi.clone(),
                    canonical_url: canonical_url.clone(),
                    relative_path: relative_path.clone(),
                    first_seen_at: now.clone(),
                    last_seen_at: now.clone(),
                });
            item.canonical_doi = canonical_doi.clone();
            item.canonical_url = canonical_url.clone();
            item.relative_path = relative_path.clone();
            item.last_seen_at = now.clone();
        };

        upsert(dedup_key_for(None, Some(url)).unwrap_or_else(|| format!("url:{canonical_url}")));
        if let Some(doi_value) = canonical_doi.as_deref() {
            upsert(format!("doi:{doi_value}"));
        }

        self.dirty = true;
    }

    pub fn mark_stale(&mut self, dedup_key: &str) {
        if self.items.remove(dedup_key).is_some() {
            self.dirty = true;
        }
    }

    pub fn save_if_dirty(&mut self) -> std::io::Result<()> {
        if !self.dirty {
            return Ok(());
        }

        let mut items: Vec<RegistryItem> = self.items.values().cloned().collect();
        items.sort_by(|a, b| a.dedup_key.cmp(&b.dedup_key));
        let payload = RegistryFile {
            schema_version: REGISTRY_SCHEMA_VERSION,
            project_key: self.project_key.clone(),
            items,
        };
        atomic_write_json(&self.path, &payload)?;
        self.dirty = false;
        Ok(())
    }
}

impl Drop for DownloadedRegistry {
    fn drop(&mut self) {
        let _ = self.lock_file.unlock();
    }
}

fn resolve_mapped_path(output_dir: &Path, relative_or_abs: &str) -> PathBuf {
    let candidate = PathBuf::from(relative_or_abs);
    if candidate.is_absolute() {
        candidate
    } else {
        output_dir.join(candidate)
    }
}

fn unix_timestamp_string() -> String {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs().to_string())
        .unwrap_or_else(|_| "0".to_string())
}

#[must_use]
pub fn normalize_doi(raw: &str) -> Option<String> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return None;
    }

    let mut doi = trimmed;
    let lower = doi.to_ascii_lowercase();

    for prefix in [
        "https://doi.org/",
        "http://doi.org/",
        "https://dx.doi.org/",
        "http://dx.doi.org/",
    ] {
        if lower.starts_with(prefix) {
            doi = &doi[prefix.len()..];
            break;
        }
    }

    if doi.len() >= 4 && doi[..4].eq_ignore_ascii_case("doi:") {
        doi = doi[4..].trim_start();
    }

    let decoded = urlencoding::decode(doi).map_or_else(|_| doi.to_string(), |value| value.into());
    let normalized = decoded.trim().to_ascii_lowercase();
    if normalized.is_empty() {
        None
    } else {
        Some(normalized)
    }
}

#[must_use]
pub fn canonicalize_url(raw: &str) -> String {
    let trimmed = raw.trim();
    if let Ok(mut parsed) = Url::parse(trimmed) {
        parsed.set_fragment(None);
        parsed.to_string()
    } else {
        trimmed.to_string()
    }
}

#[must_use]
pub fn dedup_key_for(doi: Option<&str>, url: Option<&str>) -> Option<String> {
    if let Some(doi) = doi.and_then(normalize_doi) {
        return Some(format!("doi:{doi}"));
    }

    url.map(canonicalize_url)
        .filter(|value| !value.is_empty())
        .map(|value| format!("url:{value}"))
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn test_normalize_doi_strips_prefixes_and_lowercases() {
        assert_eq!(
            normalize_doi("https://doi.org/10.1234/Foo.Bar").as_deref(),
            Some("10.1234/foo.bar")
        );
        assert_eq!(
            normalize_doi(" DOI: 10.5555/ABC ").as_deref(),
            Some("10.5555/abc")
        );
    }

    #[test]
    fn test_dedup_key_prefers_doi() {
        let key = dedup_key_for(
            Some("10.1234/ABC"),
            Some("https://example.com/paper.pdf#fragment"),
        )
        .unwrap();
        assert_eq!(key, "doi:10.1234/abc");
    }

    #[test]
    fn test_lookup_hit_and_stale_recovery() {
        let temp = tempfile::TempDir::new().unwrap();
        let output_dir = temp.path();
        let project_key = "/tmp/project";

        let mut registry = DownloadedRegistry::load(output_dir, project_key).unwrap();
        let file_path = output_dir.join("paper.pdf");
        std::fs::write(&file_path, b"pdf").unwrap();
        registry.record_success(
            output_dir,
            "https://example.com/paper.pdf",
            None,
            &file_path,
        );
        registry.save_if_dirty().unwrap();

        drop(registry);
        let mut registry = DownloadedRegistry::load(output_dir, project_key).unwrap();
        let hit = registry.lookup(output_dir, "https://example.com/paper.pdf", None);
        assert!(matches!(hit, RegistryLookup::Hit { .. }));

        std::fs::remove_file(&file_path).unwrap();
        let stale = registry.lookup(output_dir, "https://example.com/paper.pdf", None);
        assert_eq!(stale, RegistryLookup::StaleRecovered);
    }

    #[test]
    fn test_load_fails_fast_when_registry_lock_is_held() {
        let temp = tempfile::TempDir::new().unwrap();
        let output_dir = temp.path();

        let registry = DownloadedRegistry::load(output_dir, "/tmp/project").unwrap();
        let err = DownloadedRegistry::load(output_dir, "/tmp/project")
            .expect_err("concurrent load should fail fast");
        assert_eq!(err.kind(), std::io::ErrorKind::WouldBlock);
        drop(registry);
    }

    #[test]
    fn test_lock_is_released_on_drop() {
        let temp = tempfile::TempDir::new().unwrap();
        let output_dir = temp.path();

        {
            let _registry = DownloadedRegistry::load(output_dir, "/tmp/project").unwrap();
        }
        let _registry = DownloadedRegistry::load(output_dir, "/tmp/project").unwrap();
    }

    #[test]
    fn test_downloaded_registry_is_send() {
        fn assert_send<T: Send>() {}
        assert_send::<DownloadedRegistry>();
    }
}
