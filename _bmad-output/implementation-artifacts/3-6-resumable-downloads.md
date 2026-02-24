# Story 3.6: Resumable Downloads

Status: done

<!-- Note: Validation is optional. Run validate-create-story for quality check before dev-story. -->

## Story

As a **user**,
I want **large file downloads to resume if interrupted**,
so that **I don't re-download gigabytes on failure**.

## Acceptance Criteria

1. **AC1: HTTP Range Resume**
   - **Given** partially downloaded file exists
   - **When** download resumes
   - **Then** Range request is attempted for remaining bytes

2. **AC2: Server Capability Detection**
   - **Given** resume candidate exists
   - **When** server does not support ranges
   - **Then** downloader restarts from beginning

3. **AC3: Queue Progress Persistence**
   - **Given** resumable processing
   - **When** queue state is updated
   - **Then** `bytes_downloaded` and `content_length` are persisted in queue schema

4. **AC4: Integrity Verification After Resume**
   - **Given** range resume succeeded
   - **When** download completes
   - **Then** final byte size is verified against expected content length when available

5. **AC5: Resume Attempt Logging**
   - **Given** resumed-capable item
   - **When** download executes
   - **Then** resume attempts are logged

## Tasks / Subtasks

- [x] Add queue schema migration for `bytes_downloaded` and `content_length` (AC: 3)
- [x] Add queue API for progress metadata updates (AC: 3)
- [x] Add HTTP client resume flow with HEAD probe + Range GET (AC: 1, 2)
- [x] Add integrity mismatch error and final-size verification after 206 responses (AC: 4)
- [x] Add engine logging and persistence of resume metadata (AC: 3, 5)

### Review Follow-ups (AI)

- [x] [AI-Audit][Medium] Ensure non-range servers safely fall back to full download path. [src/download/client.rs]
- [x] [AI-Audit][Medium] Ensure queue item model and migration stay aligned for new progress columns. [migrations/20260215000003_add_resume_columns_to_queue.sql, src/queue/item.rs]

## Dev Notes

- Added migration `20260215000003_add_resume_columns_to_queue.sql`.
- Added `Queue::update_progress()` and queue item fields for persisted partial state.
- Added `HttpClient::download_to_file_with_metadata()` with range-capable resume behavior.
- Added `DownloadError::Integrity` and classify-as-permanent behavior.
- Engine now records resume attempts and persists bytes/content-length metadata.

### References

- [Source: _bmad-output/planning-artifacts/epics.md#Story-3.6-Resumable-Downloads]
- [Source: migrations/20260215000003_add_resume_columns_to_queue.sql]
- [Source: src/download/client.rs]
- [Source: src/download/engine.rs]
- [Source: src/queue/mod.rs]
- [Source: src/queue/item.rs]

## Dev Agent Record

### Agent Model Used

GPT-5 Codex (CLI)

### Completion Notes List

- Added resume-aware HTTP download path with range support detection.
- Added persisted progress metadata to queue schema and queue model.
- Added integrity check error for resumed downloads.

### File List

- `migrations/20260215000003_add_resume_columns_to_queue.sql`
- `src/download/client.rs`
- `src/download/error.rs`
- `src/download/engine.rs`
- `src/download/mod.rs`
- `src/download/retry.rs`
- `src/lib.rs`
- `src/queue/mod.rs`
- `src/queue/item.rs`
- `_bmad-output/implementation-artifacts/3-6-resumable-downloads.md`

### Change Log

- 2026-02-15: Story created, implemented, reviewed, and marked done.

## Party Mode Audit (AI)

Audit date: 2026-02-15  
Outcome: pass_with_actions  
Summary: High=0, Medium=2, Low=1

Findings:
- Medium: Ensure resume metadata is persisted in queue rows and not only in memory.
- Medium: Ensure integrity mismatch yields explicit permanent failure classification.
- Low: Add additional resume integration tests in unrestricted environment.

## Senior Developer Review (AI)

Reviewer: fierce  
Date: 2026-02-15  
Outcome: Approve

### Findings Resolved

- Added queue migration and item model updates for progress state.
- Added permanent classification for integrity mismatches.
- Added resume-attempt logging path in engine success handling.

### Validation Evidence

- `cargo fmt --all`
- `cargo clippy -- -D warnings`
- `cargo test --bin downloader`
- `cargo test --test queue_integration`
