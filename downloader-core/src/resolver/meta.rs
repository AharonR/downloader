//! Shared HTML `<meta>` tag parsing for site-specific resolvers.
//!
//! Several resolvers (e.g. Oxford Academic, `ScienceDirect`) extract citation
//! metadata from `<meta name="..." content="...">` / `property="..."` tags in the
//! same way. These helpers centralize that parsing so each resolver only keeps
//! its site-specific extraction glue.

use std::sync::LazyLock;

use regex::Regex;

use super::utils::compile_static_regex;

static META_TAG_RE: LazyLock<Regex> =
    LazyLock::new(|| compile_static_regex(r"(?is)<meta\s+[^>]*>"));
static META_ATTR_RE: LazyLock<Regex> = LazyLock::new(|| {
    compile_static_regex(r#"([a-zA-Z_:][-a-zA-Z0-9_:.]*)\s*=\s*(?:"([^"]*)"|'([^']*)')"#)
});

/// A parsed `<meta>` tag reduced to its name/property and content value.
#[derive(Debug, Clone)]
pub(crate) struct MetaTag {
    pub(crate) name: String,
    pub(crate) content: String,
}

/// Parse all `<meta>` tags carrying both a name/property and a content value.
pub(crate) fn collect_meta_tags(html: &str) -> Vec<MetaTag> {
    let mut tags = Vec::new();

    for tag_match in META_TAG_RE.find_iter(html) {
        let mut tag_name: Option<String> = None;
        let mut content: Option<String> = None;

        for attr in META_ATTR_RE.captures_iter(tag_match.as_str()) {
            let key = attr
                .get(1)
                .map_or("", |m| m.as_str())
                .trim()
                .to_ascii_lowercase();
            let value = attr
                .get(2)
                .or_else(|| attr.get(3))
                .map_or("", |m| m.as_str())
                .trim()
                .to_string();

            if value.is_empty() {
                continue;
            }

            if key == "name" || key == "property" {
                tag_name = Some(value.to_ascii_lowercase());
            } else if key == "content" {
                content = Some(value);
            }
        }

        if let (Some(name), Some(content)) = (tag_name, content) {
            tags.push(MetaTag { name, content });
        }
    }

    tags
}

/// Return the first unescaped content value whose tag name matches one of `keys`.
pub(crate) fn first_meta_value(meta_tags: &[MetaTag], keys: &[&str]) -> Option<String> {
    meta_tags.iter().find_map(|tag| {
        keys.iter()
            .any(|key| tag.name.eq_ignore_ascii_case(key))
            .then(|| html_unescape_basic(&tag.content))
    })
}

/// Return all unique, non-empty unescaped content values matching one of `keys`.
pub(crate) fn all_meta_values(meta_tags: &[MetaTag], keys: &[&str]) -> Vec<String> {
    let mut values = Vec::new();
    for tag in meta_tags {
        if keys.iter().any(|key| tag.name.eq_ignore_ascii_case(key)) {
            let value = html_unescape_basic(&tag.content);
            if !value.is_empty() && !values.contains(&value) {
                values.push(value);
            }
        }
    }
    values
}

/// Decode the small set of HTML entities seen in citation metadata content.
pub(crate) fn html_unescape_basic(value: &str) -> String {
    value
        .replace("&amp;", "&")
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&ndash;", "\u{2013}")
        .replace("&mdash;", "\u{2014}")
        .replace("&nbsp;", "\u{00a0}")
        .replace("&#8211;", "\u{2013}")
        .replace("&#8212;", "\u{2014}")
        .replace("&#160;", "\u{00a0}")
        .trim()
        .to_string()
}
