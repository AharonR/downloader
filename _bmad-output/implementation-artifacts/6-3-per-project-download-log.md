# Story 6.3: Per-Project Download Log

Status: done

<!-- Note: Validation is optional. Run validate-create-story for quality check before dev-story. -->

## Story

As a **user**,
I want **a `download.log` file in each project folder**,
so that **I can see the history for that specific project**.

## Acceptance Criteria

1. **AC1: Create/appended project log file**
   - **Given** a project-scoped run
   - **When** downloads complete
   - **Then** `download.log` exists in the project folder and new entries are appended

2. **AC2: Human-readable log format**
   - **Given** `download.log` output
   - **When** a user opens it
   - **Then** entries are plain-text readable (not JSON)

3. **AC3: Required entry fields**
   - **Given** each logged attempt entry
   - **When** it is written
   - **Then** it includes timestamp, status, filename, and source

4. **AC4: Clear failure markers**
   - **Given** failed download attempts in a run
   - **When** entries are written
   - **Then** failure entries are clearly marked and include error reason

5. **AC5: Reference-not-duplicate principle**
   - **Given** SQLite remains the source of truth for full attempt data
   - **When** writing `download.log`
   - **Then** entries reference persisted history rows instead of duplicating all SQLite fields

## Tasks / Subtasks

- [x] Task 1: Add project log append flow in CLI post-processing path (AC: 1, 2)
  - [x] 1.1 Capture run boundary so only new terminal attempts are written
  - [x] 1.2 Add `append_project_download_log` call for project-scoped runs
  - [x] 1.3 Preserve append mode and avoid rewrites of existing file content

- [x] Task 2: Build history-backed log rendering with clear fields (AC: 2, 3, 4, 5)
  - [x] 2.1 Query attempt rows from SQLite for this run/project boundary
  - [x] 2.2 Render human-readable entries with timestamp, status, filename, source
  - [x] 2.3 Mark failures with explicit failure reason text
  - [x] 2.4 Include SQLite row references in output to avoid full data duplication

- [x] Task 3: Add tests for project log generation behavior (AC: 1-5)
  - [x] 3.1 Unit/integration test: `download.log` created and appended
  - [x] 3.2 Test: entries include required fields and readable formatting
  - [x] 3.3 Test: failure entries are clearly marked with reason + history reference
  - [x] 3.4 Test: no entries are written when no new attempts were logged

### Review Follow-ups (AI)

- [x] [AI-Audit][Medium] Add explicit run-boundary filtering (e.g., history row ID watermark) to prevent duplicate `download.log` entries across repeated runs.
- [x] [AI-Audit][Medium] Ensure filename/source fallback behavior is deterministic when metadata is missing (`file_path`, `original_input`).
- [x] [AI-Audit][Low] Keep each `download.log` line width bounded enough for terminal/editor readability without losing row-reference context.

## Dev Notes

### Architecture Context

- Queue/history persistence remains in `output_dir/.downloader/queue.db`.
- Story 6.1/6.2 established attempt-level logging in `download_log`; Story 6.3 should reuse that source instead of copying queue internals.
- Project index behavior from Epic 5 (`index.md`) already appends per-run project summaries and can be followed as a structural pattern.

### Implementation Guidance

- Keep `download.log` writer best-effort and non-blocking relative to successful queue processing.
- Use history row boundaries (e.g., latest ID before/after run) to avoid re-writing old entries.
- Keep output deterministic and easy to scan in plain text.

### Testing Notes

- Extend current main/unit tests around project post-processing helpers.
- Verify both success and failure rows with source + filename fallbacks.
- Keep existing project index and summary behavior unchanged.

### References

- [Source: _bmad-output/planning-artifacts/epics.md#Story-6.3-Per-Project-Download-Log]
- [Source: _bmad-output/planning-artifacts/prd.md#FR-4-Logging-&-Memory]
- [Source: _bmad-output/implementation-artifacts/5-4-project-index-generation.md]
- [Source: _bmad-output/implementation-artifacts/6-1-download-attempt-logging.md]
- [Source: _bmad-output/implementation-artifacts/6-2-failure-logging-with-details.md]

## Dev Agent Record

### Agent Model Used

GPT-5 Codex (CLI)

### Debug Log References

- `cargo fmt`
- `cargo test --test queue_integration`
- `cargo test --bin downloader project_download_log`

### Completion Notes List

- Added queue history run-boundary support via `DownloadAttemptQuery.after_id` and `Queue::latest_download_attempt_id`.
- Added project-scoped post-run `download.log` appender that writes human-readable entries with timestamp/status/file/source.
- Added failure entry formatting with explicit reason text and SQLite row references (`history#<id>`).
- Added tests validating required `download.log` fields and duplicate-prevention across append runs.

### File List

- _bmad-output/implementation-artifacts/6-3-per-project-download-log.md
- _bmad-output/implementation-artifacts/sprint-status.yaml
- src/main.rs
- src/queue/history.rs
- tests/queue_integration.rs

### Change Log

- 2026-02-17: Story created and marked ready-for-dev.
- 2026-02-17: Party mode audit completed with follow-up actions.
- 2026-02-17: Implemented per-project download.log generation and moved story to review.
- 2026-02-17: Code review completed; story marked done.

## Party Mode Audit (AI)

Audit date: 2026-02-17  
Outcome: pass_with_actions  
Summary: High=0, Medium=2, Low=1

Findings:
- Medium: Without a run-boundary watermark, append mode can duplicate historical entries on repeat runs.
- Medium: Missing field fallback order (`filename`, `source`) is under-defined and can produce inconsistent log rows.
- Low: Long sources/error strings can reduce readability unless line formatting/truncation is explicit.

## Senior Developer Review (AI)

Reviewer: fierce  
Date: 2026-02-17  
Outcome: Approve

### Findings Summary

- High: 0
- Medium: 0 (addressed during implementation)
- Low: 1

### Fixed During Review (Auto-Fix High/Medium)

- Verified run-boundary watermarking (`after_id`) is applied before appending `download.log`, preventing duplicate entries across reruns.
- Verified readable field fallback and row-reference formatting for both success and failure entries.

### Low-Severity Notes

- Full `cargo clippy --all-targets -- -D warnings` remains red due extensive pre-existing lint debt in unchanged parser/auth/resolver test modules.
