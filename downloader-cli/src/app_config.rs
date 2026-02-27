//! Application configuration loading for CLI defaults.

use std::env;
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};

/// TOML-backed file configuration for downloader defaults.
#[derive(Debug, Clone, Default)]
pub struct FileConfig {
    /// Default output directory for downloads.
    pub output_dir: Option<PathBuf>,
    /// Default concurrency (same range as CLI).
    pub concurrency: Option<u8>,
    /// Default per-domain rate limit in milliseconds.
    pub rate_limit: Option<u64>,
    /// Use conservative settings for sensitive environments (overrides concurrency, rate_limit, max_retries).
    pub respectful: Option<bool>,
    /// Check robots.txt before downloading.
    pub check_robots: Option<bool>,
    /// Default verbosity mode.
    pub verbosity: Option<VerbositySetting>,
    /// Enable topic auto-detection from paper metadata (Story 8.1).
    pub detect_topics: Option<bool>,
    /// Path to custom topics file for prioritized topic matching (Story 8.1).
    pub topics_file: Option<PathBuf>,
    /// Write JSON-LD sidecar files alongside downloads (Story 8.2).
    pub sidecar: Option<bool>,
    /// Optional download client connect timeout in seconds.
    pub download_connect_timeout_secs: Option<u64>,
    /// Optional download client read timeout in seconds.
    pub download_read_timeout_secs: Option<u64>,
    /// Optional resolver client connect timeout in seconds.
    pub resolver_connect_timeout_secs: Option<u64>,
    /// Optional resolver client read timeout in seconds.
    pub resolver_read_timeout_secs: Option<u64>,
    /// Optional database pool max connections (1..=20).
    pub db_max_connections: Option<u32>,
    /// Optional database busy timeout in milliseconds.
    pub db_busy_timeout_ms: Option<u32>,
}

impl FileConfig {
    /// Validates config values against runtime and CLI constraints.
    pub fn validate(&self) -> Result<()> {
        if let Some(concurrency) = self.concurrency
            && !(1..=100).contains(&concurrency)
        {
            bail!("Invalid config value for `concurrency`: {concurrency}. Expected range: 1..=100");
        }

        if let Some(rate_limit) = self.rate_limit
            && rate_limit > 60_000
        {
            bail!("Invalid config value for `rate_limit`: {rate_limit}. Expected range: 0..=60000");
        }
        validate_timeout_secs(
            "download_connect_timeout_secs",
            self.download_connect_timeout_secs,
        )?;
        validate_timeout_secs(
            "download_read_timeout_secs",
            self.download_read_timeout_secs,
        )?;
        validate_timeout_secs(
            "resolver_connect_timeout_secs",
            self.resolver_connect_timeout_secs,
        )?;
        validate_timeout_secs(
            "resolver_read_timeout_secs",
            self.resolver_read_timeout_secs,
        )?;
        validate_db_max_connections(self.db_max_connections)?;
        validate_db_busy_timeout_ms(self.db_busy_timeout_ms)?;

        Ok(())
    }
}

fn validate_db_max_connections(value: Option<u32>) -> Result<()> {
    let Some(value) = value else {
        return Ok(());
    };
    if !(1..=20).contains(&value) {
        bail!("Invalid config value for `db_max_connections`: {value}. Expected range: 1..=20");
    }
    Ok(())
}

fn validate_db_busy_timeout_ms(value: Option<u32>) -> Result<()> {
    let Some(value) = value else {
        return Ok(());
    };
    if value > 120_000 {
        bail!("Invalid config value for `db_busy_timeout_ms`: {value}. Expected range: 0..=120000");
    }
    Ok(())
}

fn validate_timeout_secs(field: &str, value: Option<u64>) -> Result<()> {
    let Some(value) = value else {
        return Ok(());
    };
    if !(1..=3600).contains(&value) {
        bail!("Invalid config value for `{field}`: {value}. Expected range: 1..=3600");
    }
    Ok(())
}

/// Supported config verbosity labels.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VerbositySetting {
    Default,
    Verbose,
    Quiet,
    Debug,
}

impl VerbositySetting {
    /// Returns the stable string label for display output.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Default => "default",
            Self::Verbose => "verbose",
            Self::Quiet => "quiet",
            Self::Debug => "debug",
        }
    }
}

/// Loaded config metadata.
#[derive(Debug, Clone)]
pub struct LoadedConfig {
    /// Resolved config path if a base directory is known.
    pub path: Option<PathBuf>,
    /// Parsed file config when a config file exists and was valid.
    pub config: Option<FileConfig>,
    /// Indicates whether configuration was loaded from disk.
    pub loaded_from_file: bool,
}

/// Resolves default config path.
///
/// Priority:
/// 1. `$XDG_CONFIG_HOME/downloader/config.toml`
/// 2. `$HOME/.config/downloader/config.toml`
#[must_use]
pub fn resolve_default_config_path() -> Option<PathBuf> {
    if let Some(xdg_config_home) = env_var_non_empty_os("XDG_CONFIG_HOME") {
        return Some(
            PathBuf::from(xdg_config_home)
                .join("downloader")
                .join("config.toml"),
        );
    }

    let home = env_var_non_empty_os("HOME")?;
    Some(
        PathBuf::from(home)
            .join(".config")
            .join("downloader")
            .join("config.toml"),
    )
}

fn env_var_non_empty_os(name: &str) -> Option<std::ffi::OsString> {
    let value = env::var_os(name)?;
    if value.is_empty() { None } else { Some(value) }
}

/// Loads config from default path if present.
pub fn load_default_file_config() -> Result<LoadedConfig> {
    let path = resolve_default_config_path();
    let Some(path_ref) = path.as_deref() else {
        return Ok(LoadedConfig {
            path,
            config: None,
            loaded_from_file: false,
        });
    };

    if !path_ref.exists() {
        return Ok(LoadedConfig {
            path,
            config: None,
            loaded_from_file: false,
        });
    }

    let config = load_file_config(path_ref)?;
    Ok(LoadedConfig {
        path,
        config: Some(config),
        loaded_from_file: true,
    })
}

fn load_file_config(path: &Path) -> Result<FileConfig> {
    let raw = fs::read_to_string(path)
        .with_context(|| format!("Failed to read config file '{}'", path.display()))?;
    parse_config_str(&raw)
        .with_context(|| format!("Failed to parse config file '{}'", path.display()))
}

fn parse_config_str(raw: &str) -> Result<FileConfig> {
    let mut cfg = FileConfig::default();
    for (line_index, raw_line) in raw.lines().enumerate() {
        let line = strip_inline_comment(raw_line).trim();
        if line.is_empty() {
            continue;
        }

        let Some((raw_key, raw_value)) = line.split_once('=') else {
            bail!(
                "Invalid config syntax on line {}: expected key = value",
                line_index + 1
            );
        };

        let key = raw_key.trim();
        let value = raw_value.trim();

        match key {
            "output_dir" => {
                let parsed = parse_string_literal(value).with_context(|| {
                    format!("Invalid `output_dir` value on line {}", line_index + 1)
                })?;
                cfg.output_dir = Some(PathBuf::from(parsed));
            }
            "concurrency" => {
                let parsed = parse_integer_u8(value).with_context(|| {
                    format!("Invalid `concurrency` value on line {}", line_index + 1)
                })?;
                cfg.concurrency = Some(parsed);
            }
            "rate_limit" => {
                let parsed = parse_integer_u64(value).with_context(|| {
                    format!("Invalid `rate_limit` value on line {}", line_index + 1)
                })?;
                cfg.rate_limit = Some(parsed);
            }
            "respectful" => {
                let parsed = parse_boolean(value).with_context(|| {
                    format!("Invalid `respectful` value on line {}", line_index + 1)
                })?;
                cfg.respectful = Some(parsed);
            }
            "check_robots" => {
                let parsed = parse_boolean(value).with_context(|| {
                    format!("Invalid `check_robots` value on line {}", line_index + 1)
                })?;
                cfg.check_robots = Some(parsed);
            }
            "verbosity" => {
                let parsed = parse_string_literal(value).with_context(|| {
                    format!("Invalid `verbosity` value on line {}", line_index + 1)
                })?;
                cfg.verbosity = Some(parse_verbosity(&parsed).with_context(|| {
                    format!(
                        "Invalid `verbosity` value '{}' on line {}",
                        parsed,
                        line_index + 1
                    )
                })?);
            }
            "detect_topics" => {
                let parsed = parse_boolean(value).with_context(|| {
                    format!("Invalid `detect_topics` value on line {}", line_index + 1)
                })?;
                cfg.detect_topics = Some(parsed);
            }
            "topics_file" => {
                let parsed = parse_string_literal(value).with_context(|| {
                    format!("Invalid `topics_file` value on line {}", line_index + 1)
                })?;
                cfg.topics_file = Some(PathBuf::from(parsed));
            }
            "sidecar" => {
                let parsed = parse_boolean(value).with_context(|| {
                    format!("Invalid `sidecar` value on line {}", line_index + 1)
                })?;
                cfg.sidecar = Some(parsed);
            }
            "download_connect_timeout_secs" => {
                let parsed = parse_integer_u64(value).with_context(|| {
                    format!(
                        "Invalid `download_connect_timeout_secs` value on line {}",
                        line_index + 1
                    )
                })?;
                cfg.download_connect_timeout_secs = Some(parsed);
            }
            "download_read_timeout_secs" => {
                let parsed = parse_integer_u64(value).with_context(|| {
                    format!(
                        "Invalid `download_read_timeout_secs` value on line {}",
                        line_index + 1
                    )
                })?;
                cfg.download_read_timeout_secs = Some(parsed);
            }
            "resolver_connect_timeout_secs" => {
                let parsed = parse_integer_u64(value).with_context(|| {
                    format!(
                        "Invalid `resolver_connect_timeout_secs` value on line {}",
                        line_index + 1
                    )
                })?;
                cfg.resolver_connect_timeout_secs = Some(parsed);
            }
            "resolver_read_timeout_secs" => {
                let parsed = parse_integer_u64(value).with_context(|| {
                    format!(
                        "Invalid `resolver_read_timeout_secs` value on line {}",
                        line_index + 1
                    )
                })?;
                cfg.resolver_read_timeout_secs = Some(parsed);
            }
            "db_max_connections" => {
                let parsed = parse_integer_u64(value).with_context(|| {
                    format!(
                        "Invalid `db_max_connections` value on line {}",
                        line_index + 1
                    )
                })?;
                let n = u32::try_from(parsed)
                    .map_err(|_| anyhow::anyhow!("db_max_connections out of range for u32"))?;
                cfg.db_max_connections = Some(n);
            }
            "db_busy_timeout_ms" => {
                let parsed = parse_integer_u64(value).with_context(|| {
                    format!(
                        "Invalid `db_busy_timeout_ms` value on line {}",
                        line_index + 1
                    )
                })?;
                let n = u32::try_from(parsed)
                    .map_err(|_| anyhow::anyhow!("db_busy_timeout_ms out of range for u32"))?;
                cfg.db_busy_timeout_ms = Some(n);
            }
            unknown => {
                bail!(
                    "Unknown configuration key: '{}' on line {}",
                    unknown,
                    line_index + 1
                );
            }
        }
    }
    cfg.validate()?;
    Ok(cfg)
}

fn strip_inline_comment(line: &str) -> &str {
    let mut in_string = false;
    for (index, ch) in line.char_indices() {
        match ch {
            '"' => in_string = !in_string,
            '#' if !in_string => return &line[..index],
            _ => {}
        }
    }
    line
}

fn parse_string_literal(raw_value: &str) -> Result<String> {
    if raw_value.len() < 2 || !raw_value.starts_with('"') || !raw_value.ends_with('"') {
        bail!("Expected double-quoted string");
    }
    Ok(raw_value[1..raw_value.len() - 1].to_string())
}

fn parse_integer_u8(raw_value: &str) -> Result<u8> {
    let token = raw_value.trim();
    if token.is_empty() {
        bail!("Expected integer value");
    }
    let value = token.parse::<u16>()?;
    u8::try_from(value).map_err(|_| anyhow::anyhow!("Integer value out of range for u8"))
}

fn parse_integer_u64(raw_value: &str) -> Result<u64> {
    let token = raw_value.trim();
    if token.is_empty() {
        bail!("Expected integer value");
    }
    let value = token.parse::<i128>()?;
    if value < 0 {
        bail!("Expected non-negative integer");
    }
    u64::try_from(value).map_err(|_| anyhow::anyhow!("Integer value out of range for u64"))
}

fn parse_verbosity(value: &str) -> Result<VerbositySetting> {
    match value {
        "default" => Ok(VerbositySetting::Default),
        "verbose" => Ok(VerbositySetting::Verbose),
        "quiet" => Ok(VerbositySetting::Quiet),
        "debug" => Ok(VerbositySetting::Debug),
        _ => bail!("Expected one of: default, verbose, quiet, debug"),
    }
}

fn parse_boolean(raw_value: &str) -> Result<bool> {
    match raw_value.trim() {
        "true" => Ok(true),
        "false" => Ok(false),
        _ => bail!("Expected 'true' or 'false'"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_config_partial_fields() {
        let cfg = parse_config_str(
            r#"
concurrency = 8
verbosity = "verbose"
"#,
        )
        .expect("partial config should parse");
        assert_eq!(cfg.concurrency, Some(8));
        assert_eq!(cfg.verbosity, Some(VerbositySetting::Verbose));
        assert!(cfg.output_dir.is_none());
    }

    #[test]
    fn test_parse_config_rejects_invalid_concurrency() {
        let err = parse_config_str("concurrency = 0").expect_err("invalid concurrency expected");
        assert!(
            err.to_string().contains("concurrency"),
            "expected concurrency validation error"
        );
    }

    #[test]
    fn test_parse_config_rejects_invalid_rate_limit() {
        let err = parse_config_str("rate_limit = 60001").expect_err("invalid rate_limit expected");
        assert!(
            err.to_string().contains("rate_limit"),
            "expected rate_limit validation error"
        );
    }

    #[test]
    fn test_parse_config_rejects_numeric_values_with_trailing_tokens() {
        let err = parse_config_str("concurrency = 4 trailing")
            .expect_err("expected trailing token error");
        assert!(err.to_string().contains("concurrency"));
    }

    #[test]
    fn test_parse_config_rejects_rate_limit_too_large_for_u64() {
        let err = parse_config_str("rate_limit = 18446744073709551616")
            .expect_err("expected out-of-range u64 error");
        assert!(err.to_string().contains("rate_limit"));
    }

    #[test]
    fn test_parse_config_supports_inline_comments() {
        let cfg = parse_config_str(
            r#"
concurrency = 4 # workers
verbosity = "quiet" # preferred noise level
"#,
        )
        .expect("config with comments should parse");
        assert_eq!(cfg.concurrency, Some(4));
        assert_eq!(cfg.verbosity, Some(VerbositySetting::Quiet));
    }

    #[test]
    fn test_verbosity_as_str() {
        assert_eq!(VerbositySetting::Default.as_str(), "default");
        assert_eq!(VerbositySetting::Verbose.as_str(), "verbose");
        assert_eq!(VerbositySetting::Quiet.as_str(), "quiet");
        assert_eq!(VerbositySetting::Debug.as_str(), "debug");
    }

    #[test]
    fn test_parse_config_topic_detection_enabled() {
        let cfg = parse_config_str(r#"detect_topics = true"#).expect("detect_topics should parse");
        assert_eq!(cfg.detect_topics, Some(true));
    }

    #[test]
    fn test_parse_config_topic_detection_disabled() {
        let cfg = parse_config_str(r#"detect_topics = false"#).expect("detect_topics should parse");
        assert_eq!(cfg.detect_topics, Some(false));
    }

    #[test]
    fn test_parse_config_topics_file_path() {
        let cfg = parse_config_str(r#"topics_file = "/path/to/topics.txt""#)
            .expect("topics_file should parse");
        assert_eq!(cfg.topics_file, Some(PathBuf::from("/path/to/topics.txt")));
    }

    #[test]
    fn test_parse_config_rejects_invalid_boolean() {
        let err = parse_config_str("detect_topics = yes").expect_err("invalid boolean expected");
        assert!(err.to_string().contains("detect_topics"));
    }

    #[test]
    fn test_parse_config_sidecar_enabled() {
        let cfg = parse_config_str("sidecar = true").expect("sidecar should parse");
        assert_eq!(cfg.sidecar, Some(true));
    }

    #[test]
    fn test_parse_config_sidecar_disabled() {
        let cfg = parse_config_str("sidecar = false").expect("sidecar should parse");
        assert_eq!(cfg.sidecar, Some(false));
    }

    #[test]
    fn test_parse_config_sidecar_not_set_by_default() {
        let cfg = parse_config_str("concurrency = 4").expect("partial config should parse");
        assert!(cfg.sidecar.is_none());
    }

    #[test]
    fn test_parse_config_rejects_invalid_sidecar_value() {
        let err = parse_config_str("sidecar = yes").expect_err("invalid boolean expected");
        assert!(err.to_string().contains("sidecar"));
    }

    #[test]
    fn test_parse_config_timeout_fields() {
        let cfg = parse_config_str(
            r#"
download_connect_timeout_secs = 15
download_read_timeout_secs = 120
resolver_connect_timeout_secs = 7
resolver_read_timeout_secs = 45
"#,
        )
        .expect("timeout config should parse");
        assert_eq!(cfg.download_connect_timeout_secs, Some(15));
        assert_eq!(cfg.download_read_timeout_secs, Some(120));
        assert_eq!(cfg.resolver_connect_timeout_secs, Some(7));
        assert_eq!(cfg.resolver_read_timeout_secs, Some(45));
    }

    #[test]
    fn test_parse_config_rejects_invalid_timeout_value() {
        let err = parse_config_str("download_connect_timeout_secs = 0")
            .expect_err("invalid timeout expected");
        assert!(err.to_string().contains("download_connect_timeout_secs"));
    }

    #[test]
    fn test_parse_config_rejects_unknown_keys() {
        let err = parse_config_str("unknown_key = 123").expect_err("unknown key error expected");
        assert!(err.to_string().contains("Unknown configuration key"));
        assert!(err.to_string().contains("unknown_key"));
    }

    #[test]
    fn test_parse_config_db_options() {
        let cfg = parse_config_str(
            r#"
db_max_connections = 10
db_busy_timeout_ms = 3000
"#,
        )
        .expect("db options should parse");
        assert_eq!(cfg.db_max_connections, Some(10));
        assert_eq!(cfg.db_busy_timeout_ms, Some(3000));
    }

    #[test]
    fn test_parse_config_rejects_invalid_db_max_connections() {
        let err = parse_config_str("db_max_connections = 0").expect_err("0 is below range");
        assert!(err.to_string().contains("db_max_connections"));
    }

    #[test]
    fn test_parse_config_rejects_invalid_db_busy_timeout_ms() {
        let err = parse_config_str("db_busy_timeout_ms = 120001")
            .expect_err("value above 120000 should be rejected");
        assert!(err.to_string().contains("db_busy_timeout_ms"));
    }
}
