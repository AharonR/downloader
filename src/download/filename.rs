//! Filename extraction, sanitization, and path resolution for downloads.
//!
//! This module provides utilities for deriving safe filenames from URLs,
//! Content-Disposition headers, and metadata, and for resolving unique paths.

use std::collections::HashMap;
use std::path::{Component, Path, PathBuf};

use url::Url;

/// Builds a preferred filename from resolver metadata, or a domain/timestamp fallback.
///
/// Pattern with complete metadata: `Author_Year_Title.ext`
/// Fallback pattern (missing metadata): `domain_timestamp.ext`
#[must_use]
pub fn build_preferred_filename<S>(url: &str, metadata: &HashMap<String, String, S>) -> String
where
    S: std::hash::BuildHasher,
{
    let extension = extension_from_url(url).unwrap_or_else(|| ".bin".to_string());
    let author = metadata
        .get("authors")
        .map(String::as_str)
        .and_then(extract_primary_author);
    let year = metadata.get("year").map(String::as_str).and_then(|v| {
        let cleaned = sanitize_filename_component(v);
        (!cleaned.is_empty()).then_some(cleaned)
    });
    let title = metadata.get("title").map(String::as_str).and_then(|v| {
        let cleaned = sanitize_filename_component(v);
        if cleaned.is_empty() {
            return None;
        }
        let truncated: String = cleaned.chars().take(60).collect();
        Some(truncated)
    });

    if let (Some(author), Some(year), Some(title)) = (author, year, title) {
        return format!("{author}_{year}_{title}{extension}");
    }

    let domain = Url::parse(url)
        .ok()
        .and_then(|u| u.host_str().map(std::string::ToString::to_string))
        .unwrap_or_else(|| "download".to_string());
    let domain = sanitize_filename_component(&domain.replace('.', "-"));
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    format!("{domain}_{timestamp}{extension}")
}

pub(crate) fn extension_from_url(url: &str) -> Option<String> {
    let parsed = Url::parse(url).ok()?;
    let last_segment = parsed.path_segments()?.next_back()?;
    let dot_index = last_segment.rfind('.')?;
    let ext = &last_segment[dot_index..];
    if ext.len() <= 1 || ext.len() > 12 {
        return None;
    }
    Some(ext.to_lowercase())
}

pub(crate) fn extract_primary_author(authors: &str) -> Option<String> {
    let first = authors.split(';').next().map_or("", str::trim);
    if first.is_empty() {
        return None;
    }
    let family = first
        .split(',')
        .next()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .unwrap_or(first);
    let normalized = sanitize_filename_component(family);
    (!normalized.is_empty()).then_some(normalized)
}

pub(crate) fn sanitize_filename_component(value: &str) -> String {
    let mut out = String::new();
    let mut prev_sep = false;
    for ch in value.chars() {
        let mapped = match ch {
            '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|' | '\'' => '_',
            c if c.is_whitespace() || c.is_control() => '_',
            c if c.is_alphanumeric() || matches!(c, '-' | '_' | '.') => c,
            _ => '_',
        };
        if mapped == '_' {
            if !prev_sep {
                out.push('_');
                prev_sep = true;
            }
        } else {
            out.push(mapped);
            prev_sep = false;
        }
    }
    out.trim_matches('_').to_string()
}

/// Guess file extension from Content-Type header.
pub(crate) fn extension_from_content_type(content_type: &str) -> &'static str {
    let mime = content_type
        .split(';')
        .next()
        .unwrap_or("")
        .trim()
        .to_lowercase();

    match mime.as_str() {
        "text/html" => ".html",
        "text/plain" => ".txt",
        "application/json" => ".json",
        "application/xml" | "text/xml" => ".xml",
        "application/pdf" => ".pdf",
        "image/jpeg" => ".jpg",
        "image/png" => ".png",
        "image/gif" => ".gif",
        "image/svg+xml" => ".svg",
        "application/zip" => ".zip",
        "application/gzip" => ".gz",
        "text/css" => ".css",
        "text/javascript" | "application/javascript" => ".js",
        "video/mp4" => ".mp4",
        "audio/mpeg" => ".mp3",
        _ => ".bin", // Fallback for unknown types
    }
}

/// Parses Content-Disposition header to extract filename.
///
/// Handles both:
/// - `attachment; filename="example.pdf"`
/// - `attachment; filename=example.pdf`
/// - `attachment; filename*=UTF-8''example.pdf` (RFC 5987)
pub(crate) fn parse_content_disposition(header: &str) -> Option<String> {
    // Try filename*= first (RFC 5987 encoded)
    if let Some(pos) = header.find("filename*=") {
        let start = pos + 10;
        let value = header[start..].trim();
        // Format: charset'language'encoded_value
        if let Some(quote_pos) = value.find("''") {
            let encoded = &value[quote_pos + 2..];
            // Take until ; or end
            let end = encoded.find(';').unwrap_or(encoded.len());
            let encoded_name = &encoded[..end].trim();
            // URL decode
            if let Ok(decoded) = urlencoding::decode(encoded_name) {
                return Some(decoded.into_owned());
            }
        }
    }

    // Try regular filename=
    if let Some(pos) = header.find("filename=") {
        let start = pos + 9;
        let value = header[start..].trim();

        // Handle quoted filename
        if let Some(stripped) = value.strip_prefix('"') {
            if let Some(end) = stripped.find('"') {
                return Some(stripped[..end].to_string());
            }
        } else {
            // Unquoted - take until ; or end
            let end = value.find(';').unwrap_or(value.len());
            let filename = value[..end].trim();
            if !filename.is_empty() {
                return Some(filename.to_string());
            }
        }
    }

    None
}

/// Sanitizes filename for filesystem safety.
///
/// Replaces characters that are invalid on common filesystems:
/// / \ : * ? " < > |
pub(crate) fn sanitize_filename(name: &str) -> String {
    let sanitized: String = name
        .chars()
        .map(|c| match c {
            '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|' => '_',
            // Also handle null and control characters
            c if c.is_control() => '_',
            c => c,
        })
        .collect();

    if sanitized.is_empty() {
        return "_".to_string();
    }

    if is_safe_filename_segment(&sanitized) {
        sanitized
    } else {
        sanitized
            .chars()
            .map(|c| if c == '.' { '_' } else { c })
            .collect()
    }
}

/// Resolves a unique file path, adding numeric suffix if file exists.
pub(crate) fn resolve_unique_path(dir: &Path, filename: &str) -> PathBuf {
    resolve_unique_path_with_suffix_start(dir, filename, 1)
}

/// Resolves a unique file path with configurable duplicate suffix start.
///
/// Example with `suffix_start = 2`: `file.pdf`, then `file_2.pdf`, `file_3.pdf`, ...
pub(crate) fn resolve_unique_path_with_suffix_start(
    dir: &Path,
    filename: &str,
    suffix_start: usize,
) -> PathBuf {
    let filename = {
        let sanitized = sanitize_filename(filename);
        // Ensure no path separators remain (defense in depth against path traversal)
        if sanitized.contains('/')
            || sanitized.contains('\\')
            || sanitized.trim_matches('_').is_empty()
        {
            "download.bin".to_string()
        } else {
            sanitized
        }
    };
    let base_path = dir.join(&filename);

    if !base_path.exists() {
        return base_path;
    }

    // Split filename into stem and extension
    let (stem, ext) = match filename.rfind('.') {
        Some(pos) => (&filename[..pos], &filename[pos..]),
        None => (filename.as_str(), ""),
    };

    // Try with numeric suffixes
    for i in suffix_start..1000 {
        let new_name = format!("{stem}_{i}{ext}");
        let new_path = dir.join(new_name);
        if !new_path.exists() {
            return new_path;
        }
    }

    // Fallback (extremely unlikely)
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    dir.join(format!("{stem}_{timestamp}{ext}"))
}

fn is_safe_filename_segment(name: &str) -> bool {
    !Path::new(name).components().any(|component| {
        matches!(
            component,
            Component::CurDir | Component::ParentDir | Component::RootDir | Component::Prefix(_)
        )
    })
}

/// Fallback filename derived from URL path segment, or `download_timestamp.bin`.
pub(crate) fn fallback_filename_from_url(url: &Url) -> String {
    if let Some(mut segments) = url.path_segments()
        && let Some(last) = segments.next_back()
        && !last.is_empty()
    {
        return sanitize_filename(last);
    }

    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    format!("download_{timestamp}.bin")
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use std::collections::HashMap;
    use std::path::Component;

    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_sanitize_filename_removes_invalid_chars() {
        assert_eq!(sanitize_filename("file/name.pdf"), "file_name.pdf");
        assert_eq!(sanitize_filename("file\\name.pdf"), "file_name.pdf");
        assert_eq!(sanitize_filename("file:name.pdf"), "file_name.pdf");
        assert_eq!(sanitize_filename("file*name.pdf"), "file_name.pdf");
        assert_eq!(sanitize_filename("file?name.pdf"), "file_name.pdf");
        assert_eq!(sanitize_filename("file\"name.pdf"), "file_name.pdf");
        assert_eq!(sanitize_filename("file<name>.pdf"), "file_name_.pdf");
        assert_eq!(sanitize_filename("file|name.pdf"), "file_name.pdf");
    }

    #[test]
    fn test_sanitize_filename_rewrites_dot_segments() {
        assert_eq!(sanitize_filename("."), "_");
        assert_eq!(sanitize_filename(".."), "__");
    }

    #[test]
    fn test_sanitize_filename_preserves_valid_chars() {
        assert_eq!(
            sanitize_filename("valid-file_name.pdf"),
            "valid-file_name.pdf"
        );
        assert_eq!(sanitize_filename("file (1).pdf"), "file (1).pdf");
        assert_eq!(sanitize_filename("日本語.pdf"), "日本語.pdf");
    }

    #[test]
    fn test_parse_content_disposition_quoted() {
        let header = r#"attachment; filename="example.pdf""#;
        assert_eq!(
            parse_content_disposition(header),
            Some("example.pdf".to_string())
        );
    }

    #[test]
    fn test_parse_content_disposition_unquoted() {
        let header = "attachment; filename=example.pdf";
        assert_eq!(
            parse_content_disposition(header),
            Some("example.pdf".to_string())
        );
    }

    #[test]
    fn test_parse_content_disposition_with_semicolon() {
        let header = r#"attachment; filename="example.pdf"; size=1234"#;
        assert_eq!(
            parse_content_disposition(header),
            Some("example.pdf".to_string())
        );
    }

    #[test]
    fn test_parse_content_disposition_rfc5987() {
        let header = "attachment; filename*=UTF-8''example%20file.pdf";
        assert_eq!(
            parse_content_disposition(header),
            Some("example file.pdf".to_string())
        );
    }

    #[test]
    fn test_parse_content_disposition_missing() {
        let header = "attachment";
        assert_eq!(parse_content_disposition(header), None);
    }

    #[test]
    fn test_resolve_unique_path_no_conflict() {
        let temp_dir = TempDir::new().unwrap();
        let path = resolve_unique_path(temp_dir.path(), "test.pdf");
        assert_eq!(path, temp_dir.path().join("test.pdf"));
    }

    #[test]
    fn test_resolve_unique_path_with_conflict() {
        let temp_dir = TempDir::new().unwrap();

        // Create existing file
        std::fs::write(temp_dir.path().join("test.pdf"), b"existing").unwrap();

        let path = resolve_unique_path(temp_dir.path(), "test.pdf");
        assert_eq!(path, temp_dir.path().join("test_1.pdf"));
    }

    #[test]
    fn test_resolve_unique_path_multiple_conflicts() {
        let temp_dir = TempDir::new().unwrap();

        // Create existing files
        std::fs::write(temp_dir.path().join("test.pdf"), b"1").unwrap();
        std::fs::write(temp_dir.path().join("test_1.pdf"), b"2").unwrap();
        std::fs::write(temp_dir.path().join("test_2.pdf"), b"3").unwrap();

        let path = resolve_unique_path(temp_dir.path(), "test.pdf");
        assert_eq!(path, temp_dir.path().join("test_3.pdf"));
    }

    #[test]
    fn test_resolve_unique_path_dot_segment_stays_under_output_dir() {
        let temp_dir = TempDir::new().unwrap();
        let path = resolve_unique_path(temp_dir.path(), "..");
        assert_eq!(path, temp_dir.path().join("download.bin"));
    }

    #[test]
    fn test_resolve_unique_path_protects_against_traversal() {
        let temp_dir = TempDir::new().unwrap();
        let base = temp_dir.path();

        // Path traversal attempts must be sanitized; resolved path must stay under base
        // and must not have any ParentDir path component (no literal .. in the path)
        for malicious in ["../../etc/passwd", "subdir/../../../etc/passwd", "a/\\b\\c"] {
            let path = resolve_unique_path(base, malicious);
            assert!(
                path.starts_with(base),
                "resolved path must be under output dir: got {}",
                path.display()
            );
            let has_parent_dir = path.components().any(|c| c == Component::ParentDir);
            assert!(
                !has_parent_dir,
                "resolved path must not have .. component: got {}",
                path.display()
            );
        }
    }

    #[test]
    fn test_build_preferred_filename_with_metadata_pattern() {
        let mut metadata = HashMap::new();
        metadata.insert("authors".to_string(), "Smith, John; Doe, Jane".to_string());
        metadata.insert("year".to_string(), "2024".to_string());
        metadata.insert("title".to_string(), "A Study on Climate Change".to_string());

        let filename = build_preferred_filename("https://example.com/paper.pdf", &metadata);
        assert_eq!(filename, "Smith_2024_A_Study_on_Climate_Change.pdf");
    }

    #[test]
    fn test_build_preferred_filename_truncates_title_to_sixty_chars() {
        let mut metadata = HashMap::new();
        metadata.insert("authors".to_string(), "Smith, John".to_string());
        metadata.insert("year".to_string(), "2024".to_string());
        metadata.insert("title".to_string(), "A".repeat(90));

        let filename = build_preferred_filename("https://example.com/paper.pdf", &metadata);
        let prefix = "Smith_2024_";
        assert!(filename.starts_with(prefix));
        assert!(filename.ends_with(".pdf"));

        let title_part = filename
            .trim_start_matches(prefix)
            .trim_end_matches(".pdf")
            .to_string();
        assert_eq!(title_part.chars().count(), 60);
    }

    #[test]
    fn test_build_preferred_filename_fallback_domain_timestamp() {
        let metadata = HashMap::new();
        let filename = build_preferred_filename("https://example.com/download", &metadata);
        assert!(filename.starts_with("example-com_"));
        assert!(filename.ends_with(".bin"));

        let timestamp = filename
            .trim_start_matches("example-com_")
            .trim_end_matches(".bin");
        assert!(
            timestamp.chars().all(|c| c.is_ascii_digit()),
            "expected numeric timestamp, got: {timestamp}"
        );
    }

    #[test]
    fn test_resolve_unique_path_with_metadata_suffix_starts_at_two() {
        let temp_dir = TempDir::new().unwrap();
        std::fs::write(temp_dir.path().join("Smith_2024_Title.pdf"), b"existing").unwrap();

        let path =
            resolve_unique_path_with_suffix_start(temp_dir.path(), "Smith_2024_Title.pdf", 2);
        assert_eq!(path, temp_dir.path().join("Smith_2024_Title_2.pdf"));
    }

    // --- extension_from_url ---

    #[test]
    fn test_extension_from_url_pdf() {
        assert_eq!(
            extension_from_url("https://example.com/paper.pdf"),
            Some(".pdf".to_string())
        );
    }

    #[test]
    fn test_extension_from_url_no_extension() {
        assert_eq!(extension_from_url("https://example.com/paper"), None);
    }

    #[test]
    fn test_extension_from_url_too_long_extension_rejected() {
        // Extensions longer than 12 chars are rejected
        assert_eq!(
            extension_from_url("https://example.com/file.toolongextension"),
            None
        );
    }

    #[test]
    fn test_extension_from_url_lowercases_extension() {
        assert_eq!(
            extension_from_url("https://example.com/paper.PDF"),
            Some(".pdf".to_string())
        );
    }

    #[test]
    fn test_extension_from_url_uses_last_segment() {
        assert_eq!(
            extension_from_url("https://example.com/dir/paper.html"),
            Some(".html".to_string())
        );
    }

    #[test]
    fn test_extension_from_url_dot_only_rejected() {
        // A dot with nothing after it has len == 1, rejected
        assert_eq!(extension_from_url("https://example.com/file."), None);
    }

    // --- extract_primary_author ---

    #[test]
    fn test_extract_primary_author_family_name_from_comma() {
        assert_eq!(
            extract_primary_author("Smith, John"),
            Some("Smith".to_string())
        );
    }

    #[test]
    fn test_extract_primary_author_multiple_authors_takes_first() {
        assert_eq!(
            extract_primary_author("Smith, John; Doe, Jane"),
            Some("Smith".to_string())
        );
    }

    #[test]
    fn test_extract_primary_author_no_comma_uses_whole_name() {
        assert_eq!(
            extract_primary_author("Einstein"),
            Some("Einstein".to_string())
        );
    }

    #[test]
    fn test_extract_primary_author_empty_string_returns_none() {
        assert_eq!(extract_primary_author(""), None);
    }

    #[test]
    fn test_extract_primary_author_whitespace_only_returns_none() {
        assert_eq!(extract_primary_author("   "), None);
    }

    #[test]
    fn test_extract_primary_author_special_chars_sanitized() {
        // Colon gets sanitized to underscore, but result must be non-empty
        let result = extract_primary_author("O'Brien, Pat");
        assert!(result.is_some());
        assert!(!result.unwrap().contains('\''));
    }

    // --- extension_from_content_type ---

    #[test]
    fn test_extension_from_content_type_pdf() {
        assert_eq!(extension_from_content_type("application/pdf"), ".pdf");
    }

    #[test]
    fn test_extension_from_content_type_html() {
        assert_eq!(extension_from_content_type("text/html"), ".html");
    }

    #[test]
    fn test_extension_from_content_type_plain_text() {
        assert_eq!(extension_from_content_type("text/plain"), ".txt");
    }

    #[test]
    fn test_extension_from_content_type_strips_parameters() {
        assert_eq!(
            extension_from_content_type("text/html; charset=utf-8"),
            ".html"
        );
    }

    #[test]
    fn test_extension_from_content_type_case_insensitive() {
        assert_eq!(extension_from_content_type("Application/PDF"), ".pdf");
    }

    #[test]
    fn test_extension_from_content_type_xml_variants() {
        assert_eq!(extension_from_content_type("application/xml"), ".xml");
        assert_eq!(extension_from_content_type("text/xml"), ".xml");
    }

    #[test]
    fn test_extension_from_content_type_unknown_falls_back_to_bin() {
        assert_eq!(
            extension_from_content_type("application/octet-stream"),
            ".bin"
        );
        assert_eq!(extension_from_content_type(""), ".bin");
    }

    #[test]
    fn test_extension_from_content_type_javascript() {
        assert_eq!(extension_from_content_type("text/javascript"), ".js");
        assert_eq!(extension_from_content_type("application/javascript"), ".js");
    }

    // --- fallback_filename_from_url ---

    #[test]
    fn test_fallback_filename_from_url_uses_last_path_segment() {
        let url = url::Url::parse("https://example.com/papers/thesis.pdf").unwrap();
        assert_eq!(fallback_filename_from_url(&url), "thesis.pdf");
    }

    #[test]
    fn test_fallback_filename_from_url_empty_path_returns_timestamp_fallback() {
        let url = url::Url::parse("https://example.com/").unwrap();
        let result = fallback_filename_from_url(&url);
        assert!(result.starts_with("download_"));
        assert!(result.ends_with(".bin"));
    }

    #[test]
    fn test_fallback_filename_from_url_sanitizes_invalid_chars() {
        // Colons in the filename component get sanitized
        let url = url::Url::parse("https://example.com/file%3Aname.pdf").unwrap();
        let result = fallback_filename_from_url(&url);
        assert!(!result.contains(':'));
    }
}
