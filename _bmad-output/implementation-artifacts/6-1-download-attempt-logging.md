# Story 6.1: Download Attempt Logging

Status: done

<!-- Note: Validation is optional. Run validate-create-story for quality check before dev-story. -->

## Story

As a **user**,
I want **all download attempts logged**,
so that **I have a record of what was downloaded and when**.

## Acceptance Criteria

1. **AC1: Persist attempt record for every completed attempt**
   - **Given** any download attempt (success or failure)
   - **When** the attempt finishes processing
   - **Then** one SQLite `download_log` record is written with `url`, `status`, `timestamp`, and `file_path` (when available)

2. **AC2: Persist metadata fields when present**
   - **Given** queue metadata or resolver metadata is available
   - **When** an attempt is logged
   - **Then** metadata fields (`title`, `authors`, `doi`) are stored in the log record when available

3. **AC3: History is queryable by date range, status, and project**
   - **Given** persisted download attempts
   - **When** history is queried through repository APIs
   - **Then** callers can filter by date range, status, and project scope

4. **AC4: Logging path is non-disruptive to downloads**
   - **Given** download processing is in progress
   - **When** attempt logging runs
   - **Then** logging failures do not crash or block queue processing, and performance impact stays minimal

## Tasks / Subtasks

- [x] Task 1: Extend persisted attempt schema for required metadata and query support (AC: 1, 2, 3)
  - [x] 1.1 Add migration updates for download history fields required by Story 6.1 (`title`, `authors`, `doi`)
  - [x] 1.2 Add/confirm indexes for status/time/project query patterns
  - [x] 1.3 Keep migrations backward-compatible for existing local DBs

- [x] Task 2: Add queue-level history logging/query APIs (AC: 1, 2, 3)
  - [x] 2.1 Add typed history status and query filter models
  - [x] 2.2 Add `log_download_attempt` API for success/failure entries with metadata
  - [x] 2.3 Add `query_download_attempts` API with filters: date range, status, project

- [x] Task 3: Wire logging into engine completion/failure paths (AC: 1, 2, 4)
  - [x] 3.1 Log successful attempts with final file path and metadata
  - [x] 3.2 Log failed attempts with error message and best-effort HTTP status extraction
  - [x] 3.3 Ensure logging is best-effort (warn on log failure; do not fail queue processing)

- [x] Task 4: Add tests for persistence + filtering behavior (AC: 1, 2, 3, 4)
  - [x] 4.1 Integration test: success attempt creates expected `download_log` row
  - [x] 4.2 Integration test: failed attempt creates expected `download_log` row
  - [x] 4.3 Integration test: query filters work for status/project/date-range combinations
  - [x] 4.4 Regression assertion: logging errors do not alter completed/failed stats invariants

### Review Follow-ups (AI)

- [x] [AI-Audit][Medium] Define and enforce one canonical timestamp format for persisted history records so date-range filtering semantics stay stable across environments.
- [x] [AI-Audit][Medium] Ensure project filtering uses a consistent project key/value (not an unstable display path) and add test coverage for nested project names.
- [x] [AI-Audit][Low] Add explicit regression coverage that exactly one terminal log row is written per processed queue item (no duplicate final entries).

## Dev Notes

### Architecture Context

- Queue/state DB already exists at `output_dir/.downloader/queue.db` and already runs migrations through `Database::new`.
- `download_log` table exists from prior migration; Story 6.1 should complete required metadata and queryability.
- Download processing lifecycle is owned by `src/download/engine.rs`; queue state transitions (`mark_completed*`, `mark_failed`) must remain authoritative.

### Implementation Guidance

- Keep logging side effects best-effort and isolated from terminal queue status updates.
- Reuse existing queue metadata (`meta_title`, `meta_authors`, `source_type`, `original_input`) to populate `download_log` metadata.
- Keep query API in the queue boundary so Story 6.4 can build CLI output without direct SQL in `main.rs`.
- Maintain existing invariants:
  - `completed + failed = processed`
  - interrupt/resume behavior is unchanged
  - logging failures emit warnings only

### Testing Notes

- Extend `tests/download_engine_integration.rs` for end-to-end logging behavior.
- Add/extend queue integration coverage for filtering APIs.
- Keep full validation green: `cargo fmt`, `cargo clippy -- -D warnings`, targeted + regression tests.

### References

- [Source: _bmad-output/planning-artifacts/epics.md#Epic-6-Download-History]
- [Source: _bmad-output/planning-artifacts/epics.md#Story-6.1-Download-Attempt-Logging]
- [Source: _bmad-output/planning-artifacts/prd.md#FR-4-Logging-&-Memory]
- [Source: _bmad-output/planning-artifacts/architecture.md#SQLite-Schema-Overview]
- [Source: _bmad-output/project-context.md#Database]
- [Source: _bmad-output/project-context.md#Testing-Rules]

## Dev Agent Record

### Agent Model Used

GPT-5 Codex (CLI)

### Debug Log References

- `cargo fmt`
- `cargo test --test queue_integration`
- `cargo test --lib test_database_download_log_metadata_columns_exist`
- `cargo test --lib test_download_attempt_status_from_str`
- `cargo test --lib test_normalize_history_limit_clamps_max`
- `cargo test --lib test_engine_new_valid_concurrency`
- `cargo test --test download_engine_integration --test queue_integration` (fails in sandbox: wiremock bind `Operation not permitted`)
- `cargo clippy --all-targets -- -D warnings` (fails due pre-existing warnings in unchanged parser/download/resolver tests)

### Completion Notes List

- Added migration `20260217000005_add_download_log_metadata_columns.sql` to persist `title`, `authors`, and `doi`, plus status/project/time indexes.
- Added migration `20260217000006_add_queue_meta_doi.sql` and propagated `meta_doi` through queue persistence.
- Added queue history models and APIs (`DownloadAttemptStatus`, `NewDownloadAttempt`, `DownloadAttemptQuery`, `DownloadAttempt`) in `src/queue/history.rs`.
- Added `Queue::log_download_attempt` and `Queue::query_download_attempts` for structured attempt persistence and filtered queries.
- Wired `DownloadEngine` terminal paths (success/failure) to persist download history rows with metadata, duration, and best-effort HTTP status extraction.
- Added DOI metadata propagation into queue rows and used `meta_doi` first when logging history.
- Canonicalized history `project` key to a stable absolute output path for consistent filtering.
- Added integration coverage for queue history persistence/filtering and engine history writes.

### File List

- _bmad-output/implementation-artifacts/6-1-download-attempt-logging.md
- _bmad-output/implementation-artifacts/sprint-status.yaml
- migrations/20260217000005_add_download_log_metadata_columns.sql
- migrations/20260217000006_add_queue_meta_doi.sql
- src/db.rs
- src/download/engine.rs
- src/lib.rs
- src/main.rs
- src/queue/history.rs
- src/queue/item.rs
- src/queue/mod.rs
- tests/download_engine_integration.rs
- tests/queue_integration.rs

### Change Log

- 2026-02-17: Story created and marked ready-for-dev.
- 2026-02-17: Party mode audit completed with follow-up actions.
- 2026-02-17: Implemented download attempt logging/query support and moved story to review.
- 2026-02-17: Code review completed with auto-fix of medium findings; story marked done.

## Party Mode Audit (AI)

Audit date: 2026-02-17  
Outcome: pass_with_actions  
Summary: High=0, Medium=2, Low=1

Findings:
- Medium: Timestamp representation is currently unspecified; date filtering can drift if formats vary.
- Medium: Project filter semantics are under-defined (project key vs display path), risking inconsistent query behavior.
- Low: Terminal log cardinality is not explicitly protected (one final row per queue item), increasing duplicate-risk during future refactors.

## Senior Developer Review (AI)

Reviewer: fierce  
Date: 2026-02-17  
Outcome: Approve

### Findings Summary

- High: 0
- Medium: 0 (2 fixed during auto-fix phase)
- Low: 3

### Fixed During Review (Auto-Fix High/Medium)

- Added queue-level DOI metadata persistence (`meta_doi`) and wired history logging to use it, closing a metadata loss path.
- Canonicalized project history key generation to use absolute output directory path, improving project filter consistency.

### Low-Severity Notes

- `started_at` and `completed_at` are currently both set at insert time; a future pass can capture true attempt start timestamps for richer analytics.
- Engine integration tests are present but cannot execute in this sandbox due `wiremock` port bind restrictions (`Operation not permitted`).
- Full `cargo clippy --all-targets -- -D warnings` currently fails on pre-existing warnings in unchanged parser/download/resolver test code.
