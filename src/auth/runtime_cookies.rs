use std::collections::HashSet;
use std::io;
use std::sync::Arc;

use anyhow::{Result, anyhow};
use reqwest::cookie::Jar;
use tracing::{info, warn};

use super::{
    load_cookies_into_jar, load_persisted_cookies, parse_netscape_cookies, store_persisted_cookies,
    unique_domain_count,
};

/// Loads runtime cookies from --cookies input or encrypted persisted storage.
///
/// This keeps runtime orchestration code focused on flow control while auth/cookie
/// parsing and persistence behavior stays in the auth module.
///
/// # Errors
///
/// Returns an error when the provided cookie source file cannot be read, when
/// cookie parsing fails, or when secure cookie persistence fails.
pub fn load_runtime_cookie_jar(
    cookie_source: Option<&str>,
    save_cookies: bool,
) -> Result<Option<Arc<Jar>>> {
    if let Some(cookie_source) = cookie_source {
        let reader: Box<dyn io::BufRead> = if cookie_source == "-" {
            Box::new(io::BufReader::new(io::stdin()))
        } else {
            let file = std::fs::File::open(cookie_source)
                .map_err(|e| anyhow!("Cannot open cookie file '{cookie_source}': {e}"))?;
            Box::new(io::BufReader::new(file))
        };

        let parse_result = parse_netscape_cookies(reader)
            .map_err(|e| anyhow!("Failed to parse cookie file: {e}"))?;

        for (line_num, reason) in &parse_result.warnings {
            warn!(line = line_num, reason = %reason, "Skipping malformed cookie line");
        }

        let domains: HashSet<&str> = parse_result
            .cookies
            .iter()
            .map(|cookie| cookie.domain.as_str())
            .collect();
        info!(
            count = parse_result.cookies.len(),
            domains = domains.len(),
            "Loaded cookies"
        );

        if save_cookies {
            let persisted_path = store_persisted_cookies(&parse_result.cookies)
                .map_err(|error| anyhow!("Failed to persist cookies securely: {error}"))?;
            info!(
                path = %persisted_path.display(),
                "Persisted cookies to encrypted store"
            );
        }

        return Ok(Some(load_cookies_into_jar(&parse_result.cookies)));
    }

    match load_persisted_cookies() {
        Ok(Some(cookies)) => {
            info!(
                cookies = cookies.len(),
                domains = unique_domain_count(&cookies),
                "Loaded encrypted persisted cookies"
            );
            Ok(Some(load_cookies_into_jar(&cookies)))
        }
        Ok(None) => Ok(None),
        Err(error) => {
            warn!(
                error = %error,
                "Failed to load persisted cookies; continuing without stored auth cookies"
            );
            Ok(None)
        }
    }
}
