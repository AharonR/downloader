# Story 5.4: Project Index Generation

Status: done

<!-- Note: Validation is optional. Run validate-create-story for quality check before dev-story. -->

## Story

As a **user**,
I want **an `index.md` file listing all downloads**,
so that **I can see what's in each project at a glance**.

## Acceptance Criteria

1. **AC1: Create/update project index**
   - **Given** a project folder with downloads
   - **When** the batch completes
   - **Then** `index.md` is created/updated in the project folder

2. **AC2: Include required file metadata**
   - **Given** downloaded files in a project run
   - **When** index entries are written
   - **Then** each row includes filename, title, authors, and source URL

3. **AC3: Group entries by download session**
   - **Given** multiple project download runs
   - **When** index updates are appended
   - **Then** entries are grouped by session with a timestamp label

4. **AC4: Markdown table renders cleanly**
   - **Given** project index output
   - **When** opened in Markdown editors/GitHub
   - **Then** sections and tables render cleanly with safe escaping

5. **AC5: Preserve historical index content**
   - **Given** an existing `index.md`
   - **When** a new project batch completes
   - **Then** existing entries remain and new session entries are appended

## Tasks / Subtasks

- [x] Task 1: Capture session scope for newly completed project downloads (AC: 1, 3, 5)
  - [x] 1.1 Record completed queue item IDs before processing begins
  - [x] 1.2 Determine newly completed items at end of successful run
  - [x] 1.3 Skip index updates when no new project completions exist

- [x] Task 2: Build project index markdown sections (AC: 2, 3, 4)
  - [x] 2.1 Render session heading with timestamp label
  - [x] 2.2 Render markdown table with required columns
  - [x] 2.3 Escape markdown-sensitive cell content for stable rendering

- [x] Task 3: Persist index in append mode (AC: 1, 5)
  - [x] 3.1 Create `index.md` with base header when absent
  - [x] 3.2 Preserve existing content when file already exists
  - [x] 3.3 Append new session content without mutating previous sections

- [x] Task 4: Wire index generation to project execution path (AC: 1, 3, 5)
  - [x] 4.1 Execute index update only for `--project` runs
  - [x] 4.2 Run index update only after successful, non-interrupted processing
  - [x] 4.3 Log index update summary (path and entry count)

- [x] Task 5: Add tests for index generation behavior (AC: 1-5)
  - [x] 5.1 Unit/integration test for index creation with metadata rows
  - [x] 5.2 Test append behavior preserving existing content
  - [x] 5.3 Test markdown-safe escaping in rendered rows

### Review Follow-ups (AI)

- [x] [AI-Audit][Medium] Add explicit coverage proving only files completed in the current run are appended to `index.md` (exclude prior completed queue entries).
- [x] [AI-Audit][Medium] Strengthen markdown-cell escaping to cover additional formatting breakage cases (for example backticks) and add regression tests.
- [x] [AI-Audit][Low] Add assertion coverage for session heading format to keep grouping output consistent across runs.

## Dev Notes

### Architecture Context

- Story 5.1 and 5.2 establish project-scoped output directories.
- Story 5.3 propagates resolver metadata and persisted saved-path information into queue items.
- This story adds a project-scoped markdown index generated after completed download sessions.

### Implementation Guidance

- Keep index generation isolated in dedicated helper functions in `src/main.rs`.
- Use queue status snapshots to append only entries completed in the current run.
- Avoid changing non-project behavior.
- Keep markdown output deterministic and readable.

### Testing Notes

- Extend tests near `src/main.rs` for index generation and append behavior.
- Ensure existing downloader tests remain green.
- Validate rendering safety for special characters in table cell content.

### References

- [Source: _bmad-output/planning-artifacts/epics.md#Story-5.4-Project-Index-Generation]
- [Source: _bmad-output/planning-artifacts/prd.md#FR-3.4]
- [Source: _bmad-output/project-context.md#Platform-Compatibility]

## Dev Agent Record

### Agent Model Used

GPT-5 Codex (CLI)

### Debug Log References

- `cargo fmt && cargo clippy --bin downloader -- -D warnings`
- `cargo test --bin downloader`
- `cargo test --test cli_e2e test_binary_project_`
- `cargo test --test cli_e2e test_binary_without_project_keeps_default_output_layout`

### Completion Notes List

- Added project index generation flow that appends an `index.md` section per successful project run.
- Captured pre-run completed queue IDs and append only newly completed items after processing.
- Added markdown index rendering with session grouping and table columns: filename, title, authors, source URL.
- Kept append mode behavior by preserving existing `index.md` content and appending new session blocks.
- Strengthened markdown-cell escaping to handle pipes, backticks, and line breaks.
- Added regression coverage for append behavior, session heading format, and completed-before filtering.

### File List

- `_bmad-output/implementation-artifacts/5-4-project-index-generation.md`
- `_bmad-output/implementation-artifacts/sprint-status.yaml`
- `src/main.rs`

### Change Log

- 2026-02-17: Story created and marked ready-for-dev.
- 2026-02-17: Party mode audit completed with follow-up actions.
- 2026-02-17: Implemented project index generation and moved story to review.
- 2026-02-17: Code review completed; story marked done.
## Party Mode Audit (AI)

Audit date: 2026-02-17  
Outcome: pass_with_actions  
Summary: High=0, Medium=2, Low=1

Findings:
- Medium: Session delta behavior should be explicitly tested so historical completed queue items are never re-indexed.
- Medium: Current escaping scope is narrow; unescaped markdown syntax (notably backticks) can degrade table rendering.
- Low: Session heading format is implied but not explicitly validated in tests.

## Senior Developer Review (AI)

Reviewer: fierce  
Date: 2026-02-17  
Outcome: Approve

### Findings Summary

- High: 0
- Medium: 0
- Low: 2

### Low-Severity Notes

- Consider shifting session labels from raw unix seconds to a human-readable timestamp format for easier browsing.
- Consider one binary/e2e assertion that verifies `index.md` generation on a real project run path.

### Validation Evidence

- `cargo fmt && cargo clippy --bin downloader -- -D warnings`
- `cargo test --bin downloader`
- `cargo test --test cli_e2e test_binary_project_`
- `cargo test --test cli_e2e test_binary_without_project_keeps_default_output_layout`
