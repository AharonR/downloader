//! Auth command handlers: capture and clear persisted cookies.

use std::fs;
use std::io::{self, IsTerminal, Read};
use std::path::Path;

use anyhow::{Result, anyhow, bail};
use downloader_core::{
    CapturedCookieFormat, clear_persisted_cookies, parse_captured_cookies, persisted_cookie_path,
    store_persisted_cookies, unique_domain_count,
};
use tracing::{info, warn};

pub fn run_auth_capture_command(save_cookies: bool) -> Result<()> {
    info!("Browser cookie capture");
    info!("1. Install a cookie export extension (e.g., 'Get cookies.txt LOCALLY').");
    info!("2. Log into the site you want to download from.");
    info!("3. Export cookies to Netscape format (or JSON).");
    info!("4. Paste the cookie file path or pipe cookie contents.");

    let raw_input = read_cookie_capture_input()?;
    let parsed = parse_captured_cookies(&raw_input)
        .map_err(|error| anyhow!("Cookie capture failed: {error}"))?;

    for warning in &parsed.warnings {
        warn!("{warning}");
    }

    let format_label = match parsed.format {
        CapturedCookieFormat::Netscape => "netscape",
        CapturedCookieFormat::Json => "json",
    };
    let domains = unique_domain_count(&parsed.cookies);

    info!(
        format = format_label,
        cookies = parsed.cookies.len(),
        domains,
        "Cookie capture validation complete"
    );

    if save_cookies {
        let persisted_path = store_persisted_cookies(&parsed.cookies)
            .map_err(|error| anyhow!("Failed to persist cookies securely: {error}"))?;
        info!(path = %persisted_path.display(), "Saved encrypted cookies");
    }

    info!("Cookies captured for {domains} domains");

    Ok(())
}

pub fn run_auth_clear_command() -> Result<()> {
    let removed = clear_persisted_cookies()
        .map_err(|error| anyhow!("Failed to clear persisted cookies: {error}"))?;

    if removed {
        let path = persisted_cookie_path()
            .map_err(|error| anyhow!("Failed to resolve cookie storage path: {error}"))?;
        info!(path = %path.display(), "Cleared persisted auth cookies");
    } else {
        info!("No persisted auth cookies found");
    }

    Ok(())
}

fn read_cookie_capture_input() -> Result<String> {
    if !io::stdin().is_terminal() {
        let mut buffer = String::new();
        io::stdin().read_to_string(&mut buffer)?;
        let trimmed = buffer.trim();
        if trimmed.is_empty() {
            bail!("No cookie data provided on stdin");
        }

        if !trimmed.contains('\n') && Path::new(trimmed).is_file() {
            let file_contents = fs::read_to_string(trimmed)
                .map_err(|error| anyhow!("Cannot read cookie file '{}': {}", trimmed, error))?;
            return Ok(file_contents);
        }

        return Ok(buffer);
    }

    info!("Paste cookie file path, then press Enter (or pipe cookie data via stdin):");

    let mut line = String::new();
    io::stdin().read_line(&mut line)?;
    let input = line.trim();
    if input.is_empty() {
        bail!("No cookie input provided");
    }

    if Path::new(input).is_file() {
        let file_contents = fs::read_to_string(input)
            .map_err(|error| anyhow!("Cannot read cookie file '{}': {}", input, error))?;
        Ok(file_contents)
    } else {
        Ok(input.to_string())
    }
}
