# Test Design: Optimization Review Refactor

**Date:** 2026-02-18  
**Scope:** Unit, regression, and integration tests for the optimization-review-fixes refactor (queue, parser URL, download filename, constants, project, search, failure, output, command handlers).

## 1. Test Strategy Overview

| Layer | Purpose | Where | Run with |
|-------|---------|--------|----------|
| **Unit** | Isolate behavior of extracted helpers and pure functions | `src/*/mod.rs` or `src/*/*.rs` `#[cfg(test)]` | `cargo test -p downloader` (lib) or `cargo test --bin downloader` (binary) |
| **Regression** | Lock in refactored behavior so future changes don’t break it | Same as unit + dedicated regression cases | `cargo test` |
| **Integration** | Exercise full flows across module boundaries (CLI → commands → project/search/failure/output) | `tests/*.rs` | `cargo test` |

## 2. Unit Tests

### 2.1 Output module (`src/output/mod.rs`)

- **truncate_log_field**: exact fit returns same string; truncation appends ellipsis; empty string; zero `max_chars`; single char.
- **terminal_width**: with `COLUMNS` set (valid/invalid/too small) and unset → default 80.

### 2.2 Project module (`src/project/mod.rs`)

- **sanitize_project_segment**: empty → error; "." / ".." → error; reserved name (e.g. CON) → normalized with `-project`; long segment → truncated to `MAX_PROJECT_FOLDER_CHARS`; normal segment → sanitized.
- **is_windows_reserved_name**: CON, PRN, AUX, NUL, COM1, LPT1 (case-insensitive) → true; other names → false.

### 2.3 Failure module (`src/failure/mod.rs`)

- **classify_failure**: [AUTH] → Auth; HTTP 404 → InputSource; timeout → Network; "network error" → Network; invalid URL/DOI/reference → InputSource; other → Other. Optional: 407 proxy branch.
- **extract_auth_domain**: valid `[AUTH] authentication required for example.com (HTTP ...` → Some("example.com"); no prefix or no (HTTP → None.
- **category_failure_descriptor**: each category returns descriptor with expected what/why/fix.

### 2.4 Search module (`src/search/mod.rs`)

- **normalize_search_text**: collapse whitespace, lowercase; empty after trim.
- **classify_search_match**: exact match → Exact, 1.0; substring → Substring, similarity in (0,1); fuzzy above threshold → Fuzzy; below threshold / empty → None.
- **compare_search_results**: ordering by match_kind (Exact > Substring > Fuzzy), then similarity desc, then recency/id.

### 2.5 Parser URL (`src/parser/url.rs`)

- Existing tests already cover `clean_url_trailing` (extensions, punctuation, brackets). Optional: direct tests for `has_file_extension`, `strip_trailing_punctuation`, `strip_unmatched_closing_brackets` if we want to pin the helpers in isolation.

### 2.6 Download filename (`src/download/filename.rs`)

- Already has unit tests for sanitize, content-disposition, resolve_unique_path, build_preferred_filename. No change required.

## 3. Regression Tests

### 3.1 Queue: `check_affected` and ItemNotFound

- **Scenario:** After refactor, update/delete methods use a shared `check_affected` and return `QueueError::ItemNotFound(id)` when no row is affected.
- **Test:** Create a temporary DB, enqueue one item, call `mark_completed_with_path(999, ...)` (non-existent id). Expect `Err(QueueError::ItemNotFound(999))`.
- **Where:** `src/queue/mod.rs` `#[cfg(test)]` (uses tempfile + tokio_test::block_on).

### 3.2 Parser URL: trailing cleanup behavior

- **Scenario:** URL trailing cleanup (punctuation + unmatched brackets) unchanged after split into helpers.
- **Tests:** Already present: `test_clean_url_trailing_strips_multiple_trailing_punctuation`, `test_clean_url_trailing_strips_unmatched_closing_brackets`, extension preservation. Keep as regression.

### 3.3 Commands: config show / auth / log / search

- **Scenario:** Command handlers moved to `src/commands/*`; dispatch from `main` unchanged.
- **Tests:** Integration tests (see below) run `downloader config show`, `auth clear`, `log`, `search` and assert on output. These double as regression tests for the extraction.

## 4. Integration Tests

### 4.1 Command-handler flows

- **config show:** Run `downloader config show` with no config file; assert stdout contains `config_path`, `output_dir`, `concurrency`, `rate_limit`, `verbosity`.
- **auth clear:** Run `downloader auth clear`; assert success (exit 0) and no panic.
- **log:** With a seeded history DB under a temp dir, run `downloader log --output-dir <dir> --limit 1`; assert at least one line of history-style output (date | status | ...).
- **search:** With a seeded history DB containing a known title/author, run `downloader search "<query>" --output-dir <dir> --limit 1`; assert stdout contains the expected match or “No search results” when appropriate.

### 4.2 Placement

- **Option A:** Add to existing `tests/cli_e2e.rs` (keeps all CLI e2e in one place).
- **Option B:** New file `tests/optimization_refactor_commands.rs` that only runs the above command flows and documents them as post-refactor integration tests.

This design uses **Option B** so the optimization-refactor integration tests are clearly scoped and nameable (e.g. `cargo test optimization_refactor`).

## 5. Running the Tests

```bash
# All tests
cargo test

# Lib unit + regression (queue, parser, download, project, failure, search in lib)
cargo test --lib

# Binary unit tests (main.rs + output, commands not in lib)
cargo test --bin downloader

# Optimization-refactor integration only
cargo test optimization_refactor

# By priority (if we add p0_/p1_ prefixes to new tests)
cargo test p0_
```

## 6. Summary

- **Unit:** Output (truncate_log_field, terminal_width), project (sanitize_project_segment, is_windows_reserved_name), failure (classify_failure, extract_auth_domain, category_failure_descriptor), search (normalize_search_text, classify_search_match, compare_search_results).
- **Regression:** Queue ItemNotFound with temp DB; existing parser URL tests; command-handler behavior via integration.
- **Integration:** New file `tests/optimization_refactor_commands.rs` for config show, auth clear, log, search with seeded DB where needed.
