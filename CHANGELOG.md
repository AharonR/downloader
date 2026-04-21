# Changelog

All notable changes to this project will be documented in this file.

## [Unreleased]

### Added

- **Durable per-project registry coordination** — `downloader-core/src/project_registry.rs`
  now acquires a fail-fast advisory lock (`downloaded-registry.v1.lock`) for the
  lifetime of `DownloadedRegistry`, preventing concurrent same-project snapshot
  clobbering in multi-process runs.
- **Structured app completion warnings** — `downloader-app` `DownloadSummary` now
  includes `warnings` entries, including `registry_persist_failed` with path,
  error, impact, and operator fix guidance.
- **Targeted regression coverage for project isolation and dedup skip accounting**
  in `downloader-app/src-tauri/src/commands.rs`:
  - cross-project active URL no longer blocks enqueue in the current project
  - same-project `duplicate_active` skips persist as `DownloadAttemptStatus::Skipped`
    history rows with reason code.
- **Sidecar corruption recovery tests** in `downloader-core/src/sidecar/mod.rs`
  validating quarantine + regeneration and quarantine-name uniqueness.

### Changed

- **Windows atomic replacement path hardened** — `downloader-core/src/atomic_write.rs`
  now uses `ReplaceFileW` with bounded retries for transient share/access lock
  errors and safe fallback when destination is missing.
- **Registry save semantics in app/CLI** — post-download `save_if_dirty` failures
  are surfaced as warnings rather than hard-failing otherwise completed runs.
- **Project-scoped dedup gating correctness** — removed unscoped active-URL short
  circuit in app resolve/enqueue flow so scoped duplicate checks and skip history
  are authoritative.

### Fixed

- **Cross-project queue interference in app dedup flow** — URLs active in another
  project no longer cause silent skip in the current project.
- **Sidecar quarantine collision risk** — corrupt sidecar quarantine filenames now
  include high-entropy suffixes (`nanos`, process id, sequence) and retry on
  `AlreadyExists` collisions.

## [Phase 1 — Wedge Hardening] — 2026-03-09/10

### Added

- **RIS bibliography input** — `parser/ris`: new `parse_ris_content()` parses
  `.ris` files (RIS tagged format) into `ParsedItem`s for the download pipeline.
  Handles tags `TY`, `DO`, `UR`, `TI`, `AU`, `PY`, `ER`. DOI is preferred over
  URL when both appear in the same entry; DOI normalized to bare form via
  existing `extract_dois`. Returns `RisParseResult` with entries, resolved items,
  skipped count, and total found. 15 unit tests.

- **BibTeX bibliography input** — `--bibliography` / `-B` flag added to
  `DownloadArgs`. Accepts one or more `.bib` or `.ris` files; multiple files
  can be supplied by repeating the flag. Parsed entries merge with any plain
  URL/DOI inputs before queue insertion. Dry-run mode prints a bibliography
  summary instead of downloading.

- **Corpus export to BibTeX/RIS** — new `export` module in `downloader-core`
  and `downloader export` CLI subcommand. Scans a corpus directory for `.json`
  sidecar files, deserializes as Schema.org `ScholarlyArticle`, and renders
  a bibliography in BibTeX (`.bib`) or RIS (`.ris`) format.
  - `scan_corpus()` — directory scanner; silently skips non-sidecar `.json` files
  - `generate_bibtex()` — produces `@article` blocks; citation key is
    `{lastname}{year}` with fallbacks (year-only → lastname-only → path stem);
    ampersands in titles escaped as `\&`; authors joined with ` and `
  - `generate_ris()` — produces standard `TY/TI/AU/PY/DO/UR/ER` records;
    one `AU` line per author
  - Output: file path or `-` for stdout; format defaulting to BibTeX

  ```bash
  downloader export ./corpus --output bibliography.bib
  downloader export ./corpus --format ris --output refs.ris
  downloader export ./corpus --format bibtex --output -
  ```

- **Oxford Academic resolver** — `resolver/oxford`: site-specific resolver for
  `academic.oup.com` URLs and `10.1093/*` DOIs. Encodes OUP-specific URL
  patterns, PDF endpoint construction, and auth-aware failure reporting.

- **ToS acknowledgment** — first-run prompt informing the user of their
  responsibility to comply with publisher Terms of Service. Persisted as
  `tos_acknowledged = true` in the CLI config file after acknowledgment.
  Non-interactive (non-tty) mode logs a warning instead of blocking.

- **Responsible Use documentation** — README "Responsible Use" section covering
  robots.txt compliance, per-domain rate limiting, no paywall/DRM bypass policy,
  user responsibility for ToS compliance, institutional proxy guidance, and
  conservative rate limit recommendations per major publisher domain (3–5 s for
  Elsevier/Springer/IEEE/Wiley/ACM; 1–2 s for arXiv/PubMed).

- **Planning artifacts v2** — 10-expert adversarial audit of March 8 strategy
  documents surfaced 16 high + 14 medium severity gaps. Applied v2 revisions to
  product brief, strategic roadmap, and market research; added five companion
  documents: audit record, GTM acquisition playbook, legal risk assessment,
  competitive velocity tracker, Zotero batch benchmark.

### Changed

- **Test count** — 570 → 702 lib tests (+132: RIS parser ×15, export modules
  ×31, CLI export ×6, integration plumbing). All pass. Clippy clean.

## [Epic 11] — 2026-03-08 — Backlog Cleanup

### Added

- **YouTube Shorts support** — `resolver/youtube`: `extract_video_id` now handles
  `/shorts/<id>` URLs for `www.youtube.com` and `youtube.com`
  (e.g. `https://www.youtube.com/shorts/dQw4w9WgXcQ`). Full test coverage added.
- **CompletionSummary expand/collapse all** — When there are more than 5 failed
  downloads, an "Expand all / Collapse all" toggle button appears in the desktop
  app completion summary, synced with the per-item show/hide state.
- **cargo deny** — `deny.toml` created at project root; `cargo deny check` added
  as a named CI step in `phase-rollout-gates.yml`. Accepted advisories match
  `.cargo/audit.toml` (RUSTSEC-2023-0071, RUSTSEC-2025-0119).
- **Critical regression tests as named CI step** — `cargo test --test critical`
  now runs as a distinct named step in CI, separate from the full test suite.
- **Architecture doc** — `_bmad-output/as-built-architecture.md` created with
  module map and key data flows.
- **Desktop app smoke test checklist** — `downloader-app/SMOKE_TEST.md` created
  with manual smoke test checklist for the Tauri app.

### Changed

- **Session label format** — `project.rs` `make_session_label()` changed from
  `unix-{secs}-{seq}` to `YYYY-MM-DD_HHhMMmSSs` (e.g. `2026-03-08_14h05m30s`).
  No colons — the new format is safe as a filename component on Windows.
- **ProjectSelector keyboard nav** — Added decision comment explaining why native
  `<datalist>` keyboard navigation (arrow+Enter) works without custom handlers.

### Fixed

- **URL backslash-strip guard** — `parser/url: strip_backslash_escapes` now only
  strips backslashes when the string starts with `http://` or `https://`,
  preventing over-eager stripping of Windows paths passed as inputs.
- **Crossref malformed date warning** — `resolver/crossref:
  extract_year_from_date_parts` now emits `tracing::warn!` for malformed or
  unexpected date array structures instead of silently returning `None`.

### Documentation

- **Error Message Convention** — `_bmad-output/project-context.md` now has a
  dedicated `## Error Message Convention` section formalizing the What/Why/Fix
  pattern for user-facing errors and diagnostic `warn!` messages.
- **`parse_confidence` doc comments** — Both `QueueItem` and `QueueMetadata` in
  `queue/item.rs` have expanded doc comments on `parse_confidence` and
  `parse_confidence_factors` fields documenting valid values, storage contract,
  and usage scope.
- **CI socket test env var** — Added comment in `phase-rollout-gates.yml`
  explaining why `DOWNLOADER_REQUIRE_SOCKET_TESTS=1` is required.
- **`AppState` concurrency contract** — `commands.rs` `AppState` doc comment now
  explains why `Arc<Mutex>` cancel flag is safe (one active download per window).
- **README** — Added YouTube resolver to Supported Resolvers table, mixed
  stdin + positional input examples, and CLI/GUI config alignment note.


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
