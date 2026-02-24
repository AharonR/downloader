//! Shared HTTP client construction policy for resolvers.
//!
//! This module centralizes resolver networking defaults so site resolvers stay
//! consistent on timeout, user-agent, compression, proxy compatibility, and
//! cookie support.

use std::panic::{AssertUnwindSafe, catch_unwind};
use std::sync::Arc;
use std::sync::RwLock;
use std::time::Duration;

use reqwest::Client;
use reqwest::cookie::Jar;
use reqwest::{ClientBuilder, Proxy};
use tracing::warn;

use crate::user_agent;

use super::ResolveError;

const CONNECT_TIMEOUT_SECS: u64 = 10;
const READ_TIMEOUT_SECS: u64 = 30;

#[derive(Debug, Clone, Copy)]
struct ResolverHttpTimeouts {
    connect_timeout_secs: u64,
    read_timeout_secs: u64,
}

impl Default for ResolverHttpTimeouts {
    fn default() -> Self {
        Self {
            connect_timeout_secs: CONNECT_TIMEOUT_SECS,
            read_timeout_secs: READ_TIMEOUT_SECS,
        }
    }
}

static RESOLVER_HTTP_TIMEOUTS: RwLock<ResolverHttpTimeouts> = RwLock::new(ResolverHttpTimeouts {
    connect_timeout_secs: CONNECT_TIMEOUT_SECS,
    read_timeout_secs: READ_TIMEOUT_SECS,
});

/// Configures resolver HTTP timeouts used by resolver client builders.
///
/// Intended for CLI/runtime configuration before resolver construction.
pub fn configure_resolver_http_timeouts(connect_timeout_secs: u64, read_timeout_secs: u64) {
    if let Ok(mut guard) = RESOLVER_HTTP_TIMEOUTS.write() {
        *guard = ResolverHttpTimeouts {
            connect_timeout_secs,
            read_timeout_secs,
        };
    }
}

fn resolver_http_timeouts() -> ResolverHttpTimeouts {
    RESOLVER_HTTP_TIMEOUTS
        .read()
        .map(|guard| *guard)
        .unwrap_or_default()
}

/// Builds a single shared resolver user-agent string (no per-resolver name in header).
///
/// Use this for all resolvers so traffic is not trivially fingerprintable per site.
/// `resolver_name` is only used for logging/tracing, not in the UA string.
#[must_use]
pub fn standard_user_agent(_resolver_name: &str) -> String {
    user_agent::default_resolver_user_agent()
}

/// Builds a resolver HTTP client using shared project policy.
///
/// `resolver_name` is used only for error messages and logging (e.g. proxy panic
/// warning), not in the User-Agent header.
///
/// # Errors
///
/// Returns [`ResolveError`] when client construction fails.
pub fn build_resolver_http_client(
    resolver_name: &str,
    user_agent: impl Into<String>,
    cookie_jar: Option<Arc<Jar>>,
) -> Result<Client, ResolveError> {
    let user_agent = user_agent.into();

    let initial = try_build_client(&user_agent, cookie_jar.clone(), false);
    match initial {
        Ok(client) => Ok(client),
        Err(BuildClientFailure::Panic) => {
            // Some restricted macOS CI/sandbox environments panic when querying
            // system proxy settings. Fallback keeps env-proxy support while
            // bypassing system lookup so resolver constructors stay panic-free.
            warn!(
                resolver = resolver_name,
                "Resolver client hit system proxy panic; using env-proxy fallback builder"
            );
            match try_build_client(&user_agent, cookie_jar, true) {
                Ok(client) => Ok(client),
                Err(BuildClientFailure::Panic) => Err(ResolveError::resolution_failed(
                    resolver_name,
                    "HTTP client construction panicked while initializing resolver networking",
                )),
                Err(BuildClientFailure::Build(error)) => Err(ResolveError::resolution_failed(
                    resolver_name,
                    &format!("HTTP client construction failed: {error}"),
                )),
            }
        }
        Err(BuildClientFailure::Build(error)) => Err(ResolveError::resolution_failed(
            resolver_name,
            &format!("HTTP client construction failed: {error}"),
        )),
    }
}

enum BuildClientFailure {
    Panic,
    Build(reqwest::Error),
}

fn try_build_client(
    user_agent: &str,
    cookie_jar: Option<Arc<Jar>>,
    disable_system_proxy_lookup: bool,
) -> Result<Client, BuildClientFailure> {
    let user_agent = user_agent.to_string();
    catch_unwind(AssertUnwindSafe(move || {
        let mut builder = base_builder(user_agent, cookie_jar);
        if disable_system_proxy_lookup {
            builder = apply_env_proxy_fallback(builder.no_proxy());
        }
        builder.build().map_err(BuildClientFailure::Build)
    }))
    .map_err(|_| BuildClientFailure::Panic)?
}

fn base_builder(user_agent: String, cookie_jar: Option<Arc<Jar>>) -> ClientBuilder {
    let timeouts = resolver_http_timeouts();
    let mut builder = Client::builder()
        .connect_timeout(Duration::from_secs(timeouts.connect_timeout_secs))
        .timeout(Duration::from_secs(timeouts.read_timeout_secs))
        .user_agent(user_agent)
        .gzip(true);

    if let Some(jar) = cookie_jar {
        builder = builder.cookie_provider(jar);
    }

    builder
}

fn apply_env_proxy_fallback(mut builder: ClientBuilder) -> ClientBuilder {
    if let Some(proxy) = env_proxy_for_scheme("https")
        && let Ok(resolved) = Proxy::https(&proxy)
    {
        builder = builder.proxy(resolved);
    }
    if let Some(proxy) = env_proxy_for_scheme("http")
        && let Ok(resolved) = Proxy::http(&proxy)
    {
        builder = builder.proxy(resolved);
    }
    builder
}

fn env_proxy_for_scheme(scheme: &str) -> Option<String> {
    match scheme {
        "https" => find_first_proxy_var(&["HTTPS_PROXY", "https_proxy", "ALL_PROXY", "all_proxy"]),
        "http" => find_first_proxy_var(&["HTTP_PROXY", "http_proxy", "ALL_PROXY", "all_proxy"]),
        _ => None,
    }
}

fn find_first_proxy_var(names: &[&str]) -> Option<String> {
    names.iter().find_map(|name| {
        std::env::var(name)
            .ok()
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty())
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Non-Crossref resolver names that must all receive the same shared UA.
    /// When adding a new resolver that calls `standard_user_agent`, add its name here.
    /// Excludes "crossref" (until product decision) and "direct" (DirectResolver is a
    /// passthrough and does not use an HTTP client / standard_user_agent).
    const NON_CROSSREF_RESOLVER_NAMES: &[&str] = &["arxiv", "pubmed", "ieee", "springer", "sciencedirect"];

    #[test]
    fn test_standard_user_agent_single_shared_format() {
        let ua_first = standard_user_agent(NON_CROSSREF_RESOLVER_NAMES[0]);
        for name in NON_CROSSREF_RESOLVER_NAMES {
            let ua = standard_user_agent(name);
            assert_eq!(ua, ua_first, "all resolvers must share same UA (got different for {name})");
            assert!(ua.contains("downloader/"), "UA must contain downloader/");
            assert!(ua.contains("research-tool"), "UA must contain research-tool");
            assert!(ua.contains("github.com"), "UA must contain project URL");
            assert!(
                !ua.contains(name),
                "UA must not contain resolver name '{name}' (no per-resolver fingerprinting)"
            );
        }
    }
}
