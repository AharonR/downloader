# Story 8.4: Query Past Downloads

Status: done

<!-- Note: Validation is optional. Run validate-create-story for quality check before dev-story. -->

## Story

As a **user**,
I want **to search my download history**,
so that **I can find papers I've downloaded before**.

## Acceptance Criteria

1. **AC1: `downloader search <query>` searches title/author/DOI history fields**
   - **Given** persisted history rows in `download_log`
   - **When** the user runs `downloader search <query>`
   - **Then** the query is evaluated against title, authors, and DOI fields
   - **And** matching is case-insensitive
   - **And** exact and substring matches rank ahead of fuzzy-only matches

2. **AC2: Search results include required fields**
   - **Given** one or more matching rows
   - **When** results are rendered
   - **Then** each row shows title (or filename fallback), date downloaded, and file path
   - **And** output remains deterministic and width-aware in terminal mode

3. **AC3: Project/date filters are supported**
   - **Given** optional search filters
   - **When** user supplies filter flags
   - **Then** results can be narrowed by project and date range (`--project`, `--since`, optional `--until`)
   - **And** filtering composes with global/project history scope rules already used by `downloader log`

4. **AC4: `--open` opens the selected result file**
   - **Given** matching results with existing file paths
   - **When** user passes `--open`
   - **Then** the command opens the highest-ranked matching file in the system default app
   - **And** if the selected file path is missing/unavailable, output provides a What/Why/Fix style message

5. **AC5: Fuzzy matching helps with typos**
   - **Given** a misspelled query that is similar to stored metadata
   - **When** no strong exact/substring match exists
   - **Then** fuzzy matching can still return relevant results
   - **And** fuzzy scoring threshold is deterministic and covered by tests

## Tasks / Subtasks

- [x] Task 1: Add `search` subcommand CLI surface (AC: 1, 3, 4)
  - [x] 1.1 Add `Command::Search(SearchArgs)` in `src/cli.rs`
  - [x] 1.2 Define `SearchArgs` with positional `query: String`
  - [x] 1.3 Add filter flags: `--project`, `--since`, `--until`, `--output-dir`, `--limit`
  - [x] 1.4 Add `--open` boolean flag with clear help text
  - [x] 1.5 Add CLI parse tests covering required/optional args and conflicts

- [x] Task 2: Add history search query plumbing (AC: 1, 2, 3)
  - [x] 2.1 Extend `src/queue/history.rs` with a dedicated search query model (do not overload log-status query paths)
  - [x] 2.2 Implement SQL retrieval for candidate rows with project/date bounds and stable ordering
  - [x] 2.3 Ensure query returns fields needed for rendering and opening (`id`, `title`, `authors`, `doi`, `file_path`, `started_at`, `url`)
  - [x] 2.4 Keep query path read-only and backward-compatible with existing `downloader log` behavior

- [x] Task 3: Implement deterministic ranking + fuzzy matching (AC: 1, 5)
  - [x] 3.1 Add a scoring helper that prioritizes exact and substring matches before fuzzy-only candidates
  - [x] 3.2 Use a string similarity metric for typo tolerance (e.g., `strsim::jaro_winkler` or `strsim::normalized_levenshtein`)
  - [x] 3.3 Apply deterministic tie-breakers (`score DESC`, then `started_at DESC`, then `id DESC`)
  - [x] 3.4 Document threshold behavior in code comments and tests

- [x] Task 4: Wire command execution + output formatting in main CLI flow (AC: 2, 3)
  - [x] 4.1 Route `Command::Search` in `run_downloader()` (similar to `Command::Log`)
  - [x] 4.2 Implement `run_search_command(args: &SearchArgs)` in `src/main.rs`
  - [x] 4.3 Reuse/discover history DB paths with existing scope logic (`discover_history_db_paths`, project scoping)
  - [x] 4.4 Render compact rows containing date/title-or-file/path with terminal-width truncation
  - [x] 4.5 Return clear empty-state message when no matches are found

- [x] Task 5: Implement safe cross-platform `--open` behavior (AC: 4)
  - [x] 5.1 Add a helper that opens local paths using OS-specific command invocation (`open` macOS, `xdg-open` Linux, `cmd /C start` Windows)
  - [x] 5.2 Open only the highest-ranked result for `--open` to avoid mass-launch side effects
  - [x] 5.3 Validate path existence before launching; provide actionable fallback when missing
  - [x] 5.4 Ensure path invocation avoids shell interpolation vulnerabilities

- [x] Task 6: Add comprehensive regression coverage (AC: 1-5)
  - [x] 6.1 `src/cli.rs`: parse tests for `search` command and flags
  - [x] 6.2 `tests/queue_integration.rs`: history search + filter + ranking behavior tests
  - [x] 6.3 `src/main.rs` tests: rendering and open-path decision logic tests
  - [x] 6.4 `tests/cli_e2e.rs`: command-level tests for no-results, filtered-results, and `--open` guard behavior
  - [x] 6.5 Add typo-tolerant search test cases proving fuzzy threshold behavior remains stable

### Review Follow-ups (AI)

- [x] [AI-Audit][High] Lock default search scope to openable history rows (`status = success` and non-null `file_path`) or explicitly add an include-failed mode; AC2/AC4 currently assume openable results but do not define row eligibility.
- [x] [AI-Audit][High] Define global ranking/open contract across multiple history DBs: compute one merged ranking before selecting `--open` target so per-DB pre-limits cannot hide the true top result.
- [x] [AI-Audit][Medium] Freeze fuzzy-matching contract in code and AC notes (metric, normalization, threshold constant, and deterministic tie-break behavior).
- [x] [AI-Audit][Medium] Add/search index strategy or bounded candidate-window rule for scalable metadata search (`title`, `authors`, `doi`, `started_at`) to avoid full-history scans.
- [x] [AI-Audit][Medium] Specify date filter contract (`--since`, `--until`) with inclusive bounds and timestamp format/timezone expectations; add boundary-condition tests.
- [x] [AI-Audit][Medium] Add a testable command-runner seam for `--open` and verify safe arg-based invocation (no shell interpolation, handles spaces/special characters in paths).

## Dev Notes

### Architecture Context

- Story 6.4 already established history query infrastructure and DB discovery for global/project scopes.
- Story 8.3 added parse confidence fields and `--uncertain`; Story 8.4 must not regress those log query semantics.
- Search should remain read-only against history state and should not modify queue/download rows.

### Implementation Guidance

- Prefer extending history query layer (`src/queue/history.rs`) with explicit search-specific types/functions to avoid tangled conditional SQL.
- Keep ranking logic in Rust where deterministic tie-breaking and fuzzy scoring are easier to test.
- Ensure search remains responsive for larger history sets:
  - narrow candidate rows at SQL layer first (project/date and presence of searchable fields)
  - apply fuzzy ranking on bounded candidate set
- For `--open`, only open one selected result (top-ranked) to prevent accidental app storms.
- Use existing What/Why/Fix messaging style for open failures to match Epic 7 UX conventions.

### File Structure Notes

**Likely modified files:**
- `src/cli.rs`
- `src/main.rs`
- `src/queue/history.rs`
- `tests/queue_integration.rs`
- `tests/cli_e2e.rs`

**Possible dependency update:**
- `Cargo.toml` (if adding a fuzzy-match crate such as `strsim`)

### Testing Requirements

- Verify exact match > substring match > fuzzy-only ordering.
- Verify fuzzy typo recovery (`query` misspellings still return intended rows when similarity threshold is met).
- Verify project/date filters compose correctly and do not leak rows from unrelated scopes.
- Verify `--open` behavior:
  - no matches -> no open invocation
  - match with missing path -> actionable guidance
  - match with valid path -> one open invocation against top-ranked row
- Verify existing `downloader log` tests remain green (no behavioral regression from shared query/path code).

### Previous Story Intelligence (from 8.3 and 6.4)

- Keep CLI filter semantics explicit and testable; avoid hidden behavior in notes only.
- Preserve deterministic output contracts (ordering, width truncation, clear empty states).
- Prefer branch-specific SQL over parameterized "OR short-circuit" when index usage matters.
- Maintain structured tracing and avoid free-form debug spam.

### Latest Technical Notes (validated 2026-02-18)

- `strsim` provides stable similarity metrics including `jaro_winkler` and `normalized_levenshtein` for typo-tolerant matching.
- Rust `std::process::Command` is the standard path for cross-platform child process execution and should be used with explicit args (no shell interpolation).
- Existing clap derive patterns in this repo support adding new top-level subcommands cleanly with typed args.

### References

- [Source: /Users/ar2463/Documents/GitHub/Downloader/_bmad-output/planning-artifacts/epics.md#Story-8.4-Query-Past-Downloads]
- [Source: /Users/ar2463/Documents/GitHub/Downloader/_bmad-output/planning-artifacts/prd.md#FR-4-Logging--Memory]
- [Source: /Users/ar2463/Documents/GitHub/Downloader/_bmad-output/implementation-artifacts/6-4-history-query-command.md]
- [Source: /Users/ar2463/Documents/GitHub/Downloader/_bmad-output/implementation-artifacts/8-3-parsing-confidence-tracking.md]
- [Source: /Users/ar2463/Documents/GitHub/Downloader/src/main.rs]
- [Source: /Users/ar2463/Documents/GitHub/Downloader/src/queue/history.rs]
- [strsim crate docs](https://docs.rs/strsim/latest/strsim/)
- [strsim normalized Levenshtein](https://docs.rs/strsim/latest/strsim/fn.normalized_levenshtein.html)
- [Rust `std::process::Command`](https://doc.rust-lang.org/std/process/struct.Command.html)

## Dev Agent Record

### Agent Model Used

gpt-5-codex

### Debug Log References

- `cargo fmt`
- `cargo clippy --bin downloader -- -D warnings`
- `cargo test --bin downloader`
- `cargo test --test queue_integration`
- `cargo test --test cli_e2e search`
- `cargo test` (sandbox-limited failures in existing network/system-dependent suites)

### Completion Notes List

- 2026-02-18: Story 8.4 created and marked `ready-for-dev`.
- 2026-02-18: Ultimate context engine analysis completed - comprehensive developer guide created.
- 2026-02-18: Implemented `downloader search <query>` with `--project`, `--since`, `--until`, `--output-dir`, `--limit`, and `--open`.
- 2026-02-18: Added history search query plumbing and openable-default filtering (`status='success'` with non-null `file_path`), plus deterministic SQL ordering.
- 2026-02-18: Added deterministic global ranking (exact > substring > fuzzy) using `strsim::normalized_levenshtein` with threshold `0.86`, tie-breakers (`started_at DESC`, `id DESC`), and matched-field rendering.
- 2026-02-18: Added safe cross-platform open invocation with explicit arg lists and a test seam (`open_path_with_runner`) to avoid shell interpolation.
- 2026-02-18: Added candidate-window cap (`10000` per DB) and messaging when cap is hit to bound fuzzy search workload in global mode.
- 2026-02-18: Added explicit inclusive date-boundary coverage for search queries in `tests/queue_integration.rs`.
- 2026-02-18: Story-targeted validations all passed (`clippy`, `--bin`, `queue_integration`, and search e2e); full `cargo test` remains blocked by pre-existing sandbox/network-dependent suites plus one existing parser tracing test outside Story 8.4 scope.
- 2026-02-18: Code review auto-fix pass completed for Story 8.4 with two medium-severity fixes (relative path resolution and date-range validation) plus new regression tests.
- 2026-02-18: Bug-test-writing stage validated regression coverage for review fixes via targeted unit and CLI E2E tests.

### File List

- Cargo.lock
- Cargo.toml
- _bmad-output/implementation-artifacts/8-4-query-past-downloads.md
- _bmad-output/implementation-artifacts/sprint-status.yaml
- src/cli.rs
- src/lib.rs
- src/main.rs
- src/queue/history.rs
- src/queue/mod.rs
- tests/cli_e2e.rs
- tests/queue_integration.rs

### Change Log

- 2026-02-18: Implemented Story 8.4 history search flow across CLI, history querying, ranking/open behavior, and regression tests.
- 2026-02-18: Senior code review completed with safe auto-fixes; story promoted to done.
- 2026-02-18: Regression-test writing stage completed for review findings (date-range and relative-path handling).

## Party Mode Audit (AI)

- **Audit Date:** 2026-02-18
- **Topic:** 8-4 Query Past Downloads
- **Outcome:** pass_with_actions
- **Findings:** 2 High · 4 Medium · 2 Low

### Findings by Perspective

**📋 Product/PM**
- **High (PM-1):** AC2/AC4 require file-path display and `--open`, but the story does not define whether failed/skipped rows are excluded by default. This creates product ambiguity and can produce non-openable top results.  
  Evidence: `/Users/ar2463/Documents/GitHub/Downloader/_bmad-output/implementation-artifacts/8-4-query-past-downloads.md` (AC2, AC4), `/Users/ar2463/Documents/GitHub/Downloader/_bmad-output/implementation-artifacts/6-4-history-query-command.md` (history includes failed rows by design).
- **Low (PM-2):** Search result UX does not specify whether the matched field (title/authors/doi) should be indicated, reducing explainability for fuzzy results.

**🏗️ Architect**
- **High (ARC-1):** Global mode likely reuses multi-DB query/merge logic from `log`; without an explicit global ranking contract, per-DB limits can cause incorrect top-ranked `--open` selection.  
  Evidence: `/Users/ar2463/Documents/GitHub/Downloader/src/main.rs:957` (per-DB query then merge pattern in `run_log_command`), story Task 4.3.
- **Medium (ARC-2):** Story does not include index/performance guidance for metadata search, risking expensive scans as history grows.
- **Low (ARC-3):** Output truncation strategy for long file paths is not explicitly constrained for preserving useful tail context (e.g., filename extension).

**🧪 QA/TEA**
- **Medium (QA-1):** Fuzzy matching is required but the exact metric/threshold contract is not specified; tests may become brittle across implementation choices.
- **Medium (QA-2):** Date range semantics are under-specified (format, timezone, bound inclusivity), creating edge-case inconsistency.

**💻 Developer**
- **Medium (DEV-1):** `--open` behavior needs a dedicated command-runner seam for testability and safe argument handling; current task wording does not guarantee this.

### Single Prioritized Action List

1. Define default row eligibility for search/open (openable success rows by default).
2. Define global ranking contract across multi-DB search before `--open` selection.
3. Freeze fuzzy matching metric + threshold + tie-break rules.
4. Add performance/index plan or bounded candidate strategy for history search.
5. Define date filter semantics and add boundary-condition tests.
6. Add a test seam for `--open` runner with safe arg-based execution validation.

## Senior Developer Review (AI)

- **Review Date:** 2026-02-18
- **Reviewer:** gpt-5-codex
- **Outcome:** changes_requested_with_safe_autofixes
- **Summary:** 0 High · 2 Medium · 1 Low found. All medium findings were auto-fixed in this review pass.

### Findings

1. **Medium:** Relative `file_path` values from history rows were used verbatim, so `downloader search --open` could fail when run from a different working directory than the original download run.  
   Evidence: `src/main.rs` (`run_search_command` candidate merge path before review fix).
2. **Medium:** `downloader search` accepted inverted date bounds (`--since` later than `--until`) without actionable feedback, resulting in confusing empty-result behavior.  
   Evidence: `src/main.rs` (`run_search_command` filter handling before review fix).
3. **Low (remaining):** Global search still caps candidate retrieval at 10,000 rows per history DB, so extremely old matches can be excluded from ranking/open selection in very large histories; current behavior is documented via CLI notice.

### Auto-Fixes Applied

1. Added `validate_search_date_range()` in `src/main.rs` and applied it at command entry to reject inverted bounds with What/Why/Fix guidance.
2. Added `resolve_search_candidate_file_path()` in `src/main.rs` to rebase relative stored paths against each history DB root before rendering/opening.
3. Added/updated regression coverage:
   - `src/main.rs` tests for date-range validation and relative-path resolution helpers
   - `tests/cli_e2e.rs` test for relative path rendering against history root

### Validation

- `cargo fmt`
- `cargo clippy --bin downloader -- -D warnings`
- `cargo test --bin downloader`
- `cargo test --test queue_integration`
- `cargo test --test cli_e2e search`
- `cargo test` (expected sandbox-limited failures in existing network/system-dependent suites, plus one existing parser tracing test outside Story 8.4 scope)

### Remaining Decisions

1. Decide whether to add an explicit `--exhaustive` search mode (or paging strategy) to remove the per-DB candidate cap tradeoff for very large histories.
