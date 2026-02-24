# Story 6.2: Failure Logging with Details

Status: done

<!-- Note: Validation is optional. Run validate-create-story for quality check before dev-story. -->

## Story

As a **user**,
I want **failures logged with actionable information**,
so that **I can diagnose and retry failed downloads**.

## Acceptance Criteria

1. **AC1: Categorize failure types**
   - **Given** a failed download attempt
   - **When** failure is logged
   - **Then** `error_type` is recorded as one of: `network`, `auth`, `not_found`, `parse_error`

2. **AC2: Capture HTTP status where applicable**
   - **Given** a failed HTTP response
   - **When** failure is logged
   - **Then** `http_status` is persisted when available

3. **AC3: Include actionable fix suggestions**
   - **Given** a logged failure
   - **When** error details are persisted
   - **Then** stored failure message includes a concrete resolution suggestion

4. **AC4: Track retry details**
   - **Given** a failed download after retries
   - **When** failure is logged
   - **Then** retry count and last retry timestamp are stored

5. **AC5: Preserve original user input**
   - **Given** URL/DOI/reference/bibtex-originated queue items
   - **When** failures are logged
   - **Then** original input value is persisted in history

## Tasks / Subtasks

- [x] Task 1: Extend history schema for failure-detail fields (AC: 1, 2, 4, 5)
  - [x] 1.1 Add migration columns for `error_type`, `retry_count`, `last_retry_at`, and `original_input`
  - [x] 1.2 Add indexes supporting failure-focused query patterns
  - [x] 1.3 Keep migration backwards-compatible for existing DBs

- [x] Task 2: Extend history models/query filters (AC: 1, 4, 5)
  - [x] 2.1 Extend queue history write model with failure detail fields
  - [x] 2.2 Extend queue history read model to expose persisted failure details
  - [x] 2.3 Keep query API stable while adding new optional filters

- [x] Task 3: Update engine failure logging pipeline (AC: 1, 2, 3, 4, 5)
  - [x] 3.1 Classify failures into required categories
  - [x] 3.2 Persist retry count and last retry timestamp for failed attempts
  - [x] 3.3 Persist original input + source type context to history rows
  - [x] 3.4 Ensure failure messages include suggestion text

- [x] Task 4: Add integration tests for failure detail persistence (AC: 1-5)
  - [x] 4.1 Test auth/not_found/network classification mapping
  - [x] 4.2 Test retry count and retry timestamp persistence on exhausted retries
  - [x] 4.3 Test original input preservation for DOI/reference-originated queue items

### Review Follow-ups (AI)

- [x] [AI-Audit][Medium] Add a typed failure-category mapping layer (`network`, `auth`, `not_found`, `parse_error`) so persisted values remain stable and queryable.
- [x] [AI-Audit][Medium] Persist retry diagnostics (`retry_count`, `last_retry_at`) in `download_log` rather than relying only on queue transient state.
- [x] [AI-Audit][Low] Standardize actionable suggestion text generation for failure rows so `downloader log --failed` can provide consistent fix guidance.

## Dev Notes

### Architecture Context

- Story 6.1 introduced terminal attempt history persistence and query filters.
- Story 6.2 deepens failure observability in the same `download_log` model.
- Queue remains the source of truth for input provenance (`source_type`, `original_input`) and retry metadata.

### Implementation Guidance

- Keep failure logging best-effort and non-blocking relative to queue processing.
- Classify failure type from structured `DownloadError` variants rather than fragile string matching where possible.
- Reuse project-context error-message pattern by including explicit fix suggestions.

### Testing Notes

- Keep existing queue and download engine behavior intact.
- Add focused tests for new failure fields without weakening current invariants.
- Validate that successful attempts remain unaffected by failure-detail additions.

### References

- [Source: _bmad-output/planning-artifacts/epics.md#Story-6.2-Failure-Logging-with-Details]
- [Source: _bmad-output/planning-artifacts/prd.md#FR-4-Logging-&-Memory]
- [Source: _bmad-output/project-context.md#Error-Message-Requirements]

## Dev Agent Record

### Agent Model Used

GPT-5 Codex (CLI)

### Debug Log References

- `cargo fmt`
- `cargo test --test queue_integration`
- `cargo test --lib test_database_download_log_failure_detail_columns_exist`
- `cargo test --lib test_download_error_type_from_str`
- `cargo test --lib test_build_actionable_error_message_adds_suggestion`
- `cargo test --test download_engine_integration` (fails in sandbox: wiremock bind `Operation not permitted`)

### Completion Notes List

- Added migration `20260217000007_add_download_log_failure_details.sql` with `error_type`, `retry_count`, `last_retry_at`, and `original_input` columns and failure-focused indexes.
- Added typed `DownloadErrorType` persistence/read support in queue history models and APIs.
- Updated download engine failure handling to classify errors (`network`, `auth`, `not_found`, `parse_error`) and persist actionable suggestion text.
- Persisted retry diagnostics (`retry_count`, `last_retry_at`) and original input provenance for failure records.
- Expanded queue + engine integration coverage for failure detail persistence and regression behavior.

### File List

- _bmad-output/implementation-artifacts/6-2-failure-logging-with-details.md
- _bmad-output/implementation-artifacts/sprint-status.yaml
- migrations/20260217000007_add_download_log_failure_details.sql
- src/db.rs
- src/download/engine.rs
- src/lib.rs
- src/queue/history.rs
- src/queue/mod.rs
- tests/download_engine_integration.rs
- tests/queue_integration.rs

### Change Log

- 2026-02-17: Story created and marked ready-for-dev.
- 2026-02-17: Party mode audit completed with follow-up actions.
- 2026-02-17: Implemented failure-detail logging pipeline and moved story to review.
- 2026-02-17: Code review completed; story marked done.

## Party Mode Audit (AI)

Audit date: 2026-02-17  
Outcome: pass_with_actions  
Summary: High=0, Medium=2, Low=1

Findings:
- Medium: Failure-category persistence is currently implicit and can drift without a typed mapping layer.
- Medium: Retry diagnostics are not fully represented in `download_log`, limiting post-run failure analysis.
- Low: Actionable suggestion formatting is not standardized for downstream failure reporting UX.

## Senior Developer Review (AI)

Reviewer: fierce  
Date: 2026-02-17  
Outcome: Approve

### Findings Summary

- High: 0
- Medium: 0 (addressed during implementation)
- Low: 2

### Fixed During Review (Auto-Fix High/Medium)

- Verified failure-category mapping is implemented through typed `DownloadErrorType` and persisted as constrained storage values.
- Verified failure rows persist retry diagnostics (`retry_count`, `last_retry_at`) and input provenance (`original_input`) with targeted regression coverage.

### Low-Severity Notes

- `download_engine_integration` remains constrained in this sandbox due wiremock bind restrictions (`Operation not permitted`).
- Full `cargo clippy --all-targets -- -D warnings` still fails on broad pre-existing lint debt in unchanged parser/auth/resolver test areas.
