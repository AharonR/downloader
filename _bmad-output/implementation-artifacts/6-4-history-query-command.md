# Story 6.4: History Query Command

Status: done

<!-- Note: Validation is optional. Run validate-create-story for quality check before dev-story. -->

## Story

As a **user**,
I want **to query my download history**,
so that **I can find past downloads and check their status**.

## Acceptance Criteria

1. **AC1: Add `downloader log` command with default recent view**
   - **Given** history exists
   - **When** `downloader log` runs without filters
   - **Then** it lists recent attempts with a default limit of 50 rows

2. **AC2: Support required filters**
   - **Given** optional filters
   - **When** `downloader log` is run with flags
   - **Then** supported filters include `--project`, `--status`, `--since`, and `--domain`

3. **AC3: Show required fields in output**
   - **Given** query results
   - **When** they are rendered
   - **Then** each row includes date, status, title/filename, and source

4. **AC4: Failed-only mode with fix guidance**
   - **Given** `downloader log --failed`
   - **When** output is shown
   - **Then** only failed attempts are listed with actionable fix suggestions

5. **AC5: Respect terminal readability constraints**
   - **Given** long metadata/source strings
   - **When** rows are rendered
   - **Then** output remains readable within terminal width constraints

## Tasks / Subtasks

- [x] Task 1: Add `log` subcommand CLI surface (AC: 1, 2)
  - [x] 1.1 Add clap command model for `downloader log`
  - [x] 1.2 Add filter flags: `--project`, `--status`, `--since`, `--domain`, `--failed`
  - [x] 1.3 Add default `--limit` behavior (50) with sensible max bounds

- [x] Task 2: Extend history query plumbing for command filters (AC: 1, 2, 4)
  - [x] 2.1 Add domain filter support in queue history query model/SQL
  - [x] 2.2 Map CLI filter args into `DownloadAttemptQuery`
  - [x] 2.3 Keep backward compatibility for existing history query call sites

- [x] Task 3: Implement `downloader log` command execution + output formatting (AC: 1, 3, 4, 5)
  - [x] 3.1 Open project-scoped queue DB for query command execution
  - [x] 3.2 Render date/status/title-or-file/source columns for each row
  - [x] 3.3 Add failed-only rendering path with fix suggestion output
  - [x] 3.4 Apply width-aware truncation/formatting to keep output readable

- [x] Task 4: Add tests for CLI parsing and query output behavior (AC: 1-5)
  - [x] 4.1 CLI parse tests for new `log` command flags and conflicts
  - [x] 4.2 Queue query filter tests for `domain` + failed/status filters
  - [x] 4.3 Output formatting tests for width truncation and failed suggestion display

### Review Follow-ups (AI)

- [x] [AI-Audit][Medium] Make domain filtering deterministic and case-insensitive using normalized host matching, not brittle free-text containment.
- [x] [AI-Audit][Medium] Ensure failed-only output always surfaces a clear fix suggestion even when legacy rows lack a `Suggestion:` suffix.
- [x] [AI-Audit][Low] Add stable width-aware truncation tests to prevent regressions in narrow terminals.

## Dev Notes

### Architecture Context

- History persistence and filtering APIs are available in `src/queue/history.rs`.
- Project folders keep state under `.downloader/queue.db`; command mode should resolve the same structure as download mode.
- Story 6.2 standardized actionable failure suggestion text for failed rows.

### Implementation Guidance

- Keep `downloader log` side-effect free (read-only command path).
- Reuse queue-level typed statuses where possible instead of raw string matching.
- Keep formatting deterministic and robust with/without title/file metadata.

### Testing Notes

- Add command parse tests in `src/cli.rs`.
- Add targeted unit tests in `src/main.rs` for output formatting/suggestion extraction.
- Reuse existing queue integration harness for SQL filter assertions.

### References

- [Source: _bmad-output/planning-artifacts/epics.md#Story-6.4-History-Query-Command]
- [Source: _bmad-output/planning-artifacts/prd.md#FR-4-Logging-&-Memory]
- [Source: _bmad-output/implementation-artifacts/6-1-download-attempt-logging.md]
- [Source: _bmad-output/implementation-artifacts/6-2-failure-logging-with-details.md]
- [Source: _bmad-output/implementation-artifacts/6-3-per-project-download-log.md]

## Dev Agent Record

### Agent Model Used

GPT-5 Codex (CLI)

### Debug Log References

- `cargo fmt`
- `cargo test --bin downloader test_cli_log`
- `cargo test --bin downloader history_cli_row`
- `cargo test --bin downloader map_history_status`
- `cargo test --bin downloader`
- `cargo test --test queue_integration`
- `cargo test --test cli_e2e test_binary_log_command_without_history_reports_empty_state`
- `cargo clippy --bin downloader -- -D warnings`

### Completion Notes List

- Added `downloader log` subcommand with filters: `--project`, `--status`, `--since`, `--domain`, `--failed`, and configurable `--limit` (default 50).
- Added domain-filter support to history queries using normalized host matching (exact/subdomain, case-insensitive).
- Implemented read-only `run_log_command` execution path and width-aware row rendering for history output.
- Implemented failed-only output mode with resilient suggestion extraction/fallback for legacy rows.
- Added CLI parse coverage, queue history domain-filter coverage, main output rendering coverage, and CLI E2E empty-history coverage.

### File List

- _bmad-output/implementation-artifacts/6-4-history-query-command.md
- _bmad-output/implementation-artifacts/sprint-status.yaml
- src/cli.rs
- src/main.rs
- src/queue/history.rs
- tests/cli_e2e.rs
- tests/queue_integration.rs

### Change Log

- 2026-02-17: Story created and marked ready-for-dev.
- 2026-02-17: Party mode audit completed with follow-up actions.
- 2026-02-17: Implemented history query command and moved story to review.
- 2026-02-17: Code review completed; story marked done.

## Party Mode Audit (AI)

Audit date: 2026-02-17  
Outcome: pass_with_actions  
Summary: High=0, Medium=2, Low=1

Findings:
- Medium: Domain filter behavior is ambiguous without explicit host-normalization semantics.
- Medium: Failed-only output risks inconsistent guidance if suggestion extraction is not standardized for older rows.
- Low: Terminal width truncation can regress silently without direct coverage.

## Senior Developer Review (AI)

Reviewer: fierce  
Date: 2026-02-17  
Outcome: Approve

### Findings Summary

- High: 0
- Medium: 0 (addressed during implementation)
- Low: 1

### Fixed During Review (Auto-Fix High/Medium)

- Verified domain filtering now uses normalized host matching (exact/subdomain, case-insensitive) rather than raw substring-only behavior.
- Verified failed-only output always renders fix guidance via embedded `Suggestion:` extraction or deterministic fallback mapping.

### Low-Severity Notes

- Full `cargo clippy --all-targets -- -D warnings` remains blocked by broad pre-existing lint debt in unchanged library test modules.
