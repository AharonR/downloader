# Changelog

All notable changes to this project will be documented in this file.

## [Unreleased]

### Added
- **Input pipeline implementation** — The core functionality to read URLs from stdin or command-line arguments, parse them, enqueue them, and download them. This was the critical missing piece that made the binary functional.
- **Positional URL arguments** — Users can now pass URLs directly as arguments: `downloader https://example.com/file.pdf`
- **Stdin support** — Pipe URLs via stdin: `echo "https://example.com/file.pdf" | downloader`
- **Graceful no-input handling** — When no URLs are provided (neither stdin nor args), the tool exits cleanly with helpful usage information
- **README.md** — User documentation with quick start examples, CLI options table, and build instructions
- **E2E test coverage** — Added tests for stdin with no valid URLs and stdin with invalid domains

### Fixed
- **Wikipedia-style URLs with parentheses** — Fixed regex pattern that was stripping closing parentheses from URLs like `https://en.wikipedia.org/wiki/URL_(disambiguation)`. Changed from `[^\s<>"'\)\]]+` to `[^\s<>"'\]]+` to allow `)` characters while still relying on the `clean_url_trailing()` function to handle unmatched trailing parens correctly.
- **37 clippy lint errors** across multiple categories:
  - **doc_markdown** (21 fixes) — Added backticks around code identifiers in doc comments:
    - `src/db.rs`: `SQLite`, `SQLITE_BUSY`
    - `src/parser/error.rs`: `InvalidUrl`, `UrlTooLong`
    - `src/parser/url.rs`: `MAX_URL_LENGTH`
    - `src/download/retry.rs`: `RateLimited`, `max_attempts`, `MAX_JITTER`
    - `src/download/rate_limiter.rs`: `DashMap`
  - **expect_used** (2 fixes) — Added `#[allow(clippy::expect_used)]` attributes for legitimate panics on static initialization:
    - `src/download/client.rs`: HTTP client configuration (compile-time constant)
    - `src/parser/url.rs`: URL regex pattern (compile-time constant)
  - **missing_panics_doc** (1 fix) — Added `# Panics` doc section to `HttpClient::new()` documenting the configuration panic condition
  - **match_same_arms** (1 fix) — Merged duplicate match arms in `src/download/retry.rs`: `DownloadError::Io { .. } | DownloadError::InvalidUrl { .. } => FailureType::Permanent`
  - **manual_range_contains** (1 fix) — Replaced `ext_len >= 1 && ext_len <= 5` with `(1..=5).contains(&ext_len)` in `src/parser/url.rs`
  - **cast_possible_truncation** (3 fixes) — Added allow attributes for safe bounded casts:
    - `src/download/retry.rs`: `calculate_delay()` and `calculate_jitter()` — capped at 32s/500ms
    - `src/download/rate_limiter.rs`: `add_cumulative_delay()` — small duration values
  - **cast_precision_loss** (2 fixes) — Added allow attributes for `u32 as f64` casts in backoff calculations (acceptable precision loss for timing)
  - **cast_lossless** (2 fixes) — Replaced `as f64` with `f64::from()` for lossless conversions:
    - `src/download/retry.rs`: `self.backoff_multiplier as f64` → `f64::from(self.backoff_multiplier)`
    - `src/download/retry.rs`: `(attempt - 1) as f64` → `f64::from(attempt - 1)`
  - **cast_sign_loss** (1 fix) — Added allow attribute for verified non-negative `seconds as u64` cast in `src/download/rate_limiter.rs`
  - **unused_imports** (1 fix) — Moved `InputType` import from main code to test module in `src/parser/url.rs`
  - **redundant_closure_for_method_calls** (1 fix) — Replaced `.map(|s| s.to_string())` with `.map(std::string::ToString::to_string)` in `src/download/client.rs`
  - **double_ended_iterator_last** (1 fix) — Replaced `segments.last()` with `segments.next_back()` in `src/download/client.rs`
  - **manual_strip** (1 fix) — Replaced `starts_with('"')` + `[1..]` with `strip_prefix('"')` pattern in `src/download/client.rs`
  - **unchecked_time_subtraction** (1 fix) — Replaced `self.default_delay - elapsed` with `self.default_delay.saturating_sub(elapsed)` in `src/download/rate_limiter.rs`
  - **redundant_closure** (1 fix) — Replaced `.map(|h| h.to_lowercase())` with `.map(str::to_lowercase)` in `src/download/rate_limiter.rs`
  - **single_match_else** (1 fix) — Converted `match datetime.duration_since(now)` to `if let Ok(duration) = datetime.duration_since(now)` in `src/download/rate_limiter.rs`
  - **unused_self** (1 fix) — Converted `calculate_jitter()` from instance method to associated function in `src/download/retry.rs` — it never used `self`
  - **deprecated** (6 warnings) — Added `#![allow(deprecated)]` to `tests/cli_e2e.rs` for `Command::cargo_bin` usage
- **UrlTooLong error handling** — Fixed bug in `src/parser/mod.rs` where `UrlTooLong` errors weren't being added to the skipped list (only `InvalidUrl` errors were). Added missing match arm to capture both error types.
- **Unused variable warnings** — Fixed 3 test warnings in `src/download/retry.rs` after converting `calculate_jitter()` to an associated function

### Changed
- **CLI positional URL parsing** — Removed trailing var-arg behavior so flags are parsed correctly in any position relative to URLs (e.g. `downloader https://example.com/file.pdf -q`). Use `--` to pass URL literals that begin with `-`.
- **RetryPolicy::calculate_jitter signature** — Converted from instance method to associated function `Self::calculate_jitter()` since it never accessed instance state. Updated all call sites including tests.
- **Code formatting** — Fixed 3 formatting inconsistencies:
  - Allow attribute formatting in `src/download/retry.rs` and `src/download/rate_limiter.rs`
  - Import ordering in `src/main.rs`
  - Line wrapping in `src/parser/url.rs`

## Testing
All changes were verified against the project's quality gates:
- `cargo fmt --check` — passing
- `cargo clippy -- -D warnings` — 0 errors
- `cargo test` — 276 tests passing (165 lib + 25 bin + 8 integration + 18 engine + 9 download + 11 parser + 24 queue + 16 doc), 1 ignored
- End-to-end verification: `echo "https://httpbin.org/bytes/1024" | cargo run -- -q` successfully downloaded test file
