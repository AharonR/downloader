# Story 5.3: Metadata-Based File Naming

Status: done

<!-- Note: Validation is optional. Run validate-create-story for quality check before dev-story. -->

## Story

As a **user**,
I want **downloaded files named with author, year, and title**,
so that **I can identify files without opening them**.

## Acceptance Criteria

1. **AC1: Metadata filename pattern**
   - **Given** a downloaded paper with metadata
   - **When** the file is saved
   - **Then** filename follows pattern: `Author_Year_Title.ext`

2. **AC2: Title truncation**
   - **Given** long metadata titles
   - **When** filename is generated
   - **Then** title portion is truncated to max 60 characters

3. **AC3: Filesystem-safe sanitization**
   - **Given** metadata fields contain special characters
   - **When** filename is generated
   - **Then** filename is sanitized for filesystem safety

4. **AC4: Duplicate naming behavior**
   - **Given** duplicate metadata-derived names
   - **When** files are saved
   - **Then** numeric suffixes are applied (`Author_Year_Title_2.pdf`, etc.)

5. **AC5: Fallback naming without metadata**
   - **Given** metadata is missing
   - **When** file is saved
   - **Then** fallback format is used: `domain_timestamp.ext`

## Tasks / Subtasks

- [x] Task 1: Carry resolver metadata to download-time naming (AC: 1, 5)
  - [x] 1.1 Add queue fields for naming metadata/hints
  - [x] 1.2 Store resolver metadata during enqueue in `main.rs`
  - [x] 1.3 Preserve backward compatibility for non-metadata inputs

- [x] Task 2: Implement metadata filename generation (AC: 1, 2, 3, 5)
  - [x] 2.1 Add filename builder for `Author_Year_Title.ext`
  - [x] 2.2 Truncate title segment to 60 chars
  - [x] 2.3 Sanitize all filename components safely
  - [x] 2.4 Add `domain_timestamp.ext` fallback when metadata is incomplete

- [x] Task 3: Apply duplicate suffix policy (AC: 4)
  - [x] 3.1 Ensure duplicate metadata filenames produce `_2`, `_3`, ...
  - [x] 3.2 Keep existing non-metadata duplicate behavior stable

- [x] Task 4: Integrate naming with download client/engine (AC: 1-5)
  - [x] 4.1 Pass naming hint from queue item into download save path logic
  - [x] 4.2 Preserve resume behavior and existing URL/content-disposition handling when hint absent

- [x] Task 5: Add tests for naming behavior (AC: 1-5)
  - [x] 5.1 Unit tests for metadata/fallback filename builders
  - [x] 5.2 Unit/integration tests for duplicate suffix progression
  - [x] 5.3 Integration tests for queue metadata propagation into final saved filename

### Review Follow-ups (AI)

- [x] [AI-Audit][Medium] Define deterministic first-author extraction rules from resolver metadata (single author, family-only, consortium, and malformed cases).
- [x] [AI-Audit][Medium] Explicitly define extension selection precedence for metadata naming (URL extension vs content-type fallback when extension is missing).
- [x] [AI-Audit][Low] Add explicit tests confirming metadata duplicate suffixes begin at `_2` (while keeping legacy non-metadata behavior unchanged).

### Code Review Follow-ups (AI - 2026-02-17)

- [x] [AI-Review][Medium] Preserve resumable-download behavior by using queue `bytes_downloaded` hint when preferred filenames are present.
- [x] [AI-Review][Low] Validate metadata filename propagation and `_2` duplicate suffix behavior in integration tests.

## Dev Notes

### Architecture Context

- Existing resolver pipeline already returns optional metadata (`title`, `authors`, `year`) for some inputs (notably DOI/Crossref).
- Current download naming behavior is URL/content-disposition based; this story layers metadata-first naming with safe fallbacks.

### Implementation Guidance

- Keep naming logic centralized and deterministic.
- Avoid breaking resumable downloads and non-metadata flows.
- Use explicit queue persistence for naming hints rather than recomputing metadata later.
- Keep all naming sanitization cross-platform (Windows/macOS/Linux safe).

### Testing Notes

- Extend unit coverage in `src/download/client.rs` for metadata filename logic.
- Extend integration tests in `tests/download_engine_integration.rs` and/or `tests/download_integration.rs`.
- Keep existing filename tests green.

### References

- [Source: _bmad-output/planning-artifacts/epics.md#Story-5.3-Metadata-Based-File-Naming]
- [Source: _bmad-output/planning-artifacts/prd.md#FR-3.3]
- [Source: _bmad-output/project-context.md#Platform-Compatibility]

## Dev Agent Record

### Agent Model Used

GPT-5 Codex (CLI)

### Debug Log References

- `cargo fmt`
- `cargo clippy -- -D warnings`
- `cargo test --lib download::client::tests::`
- `cargo test --bin downloader`
- `cargo test --test cli_e2e test_binary_project_`
- `cargo test --test cli_e2e test_binary_without_project_keeps_default_output_layout`
- `cargo test --test download_engine_integration test_metadata_`

### Completion Notes List

- Added queue metadata persistence (`suggested_filename`, title/authors/year, saved path) with migration.
- Added metadata-driven filename builder: `Author_Year_Title.ext`.
- Added 60-char title truncation and filesystem-safe component sanitization.
- Added metadata-missing fallback naming: `domain_timestamp.ext`.
- Routed metadata filename hints through queue -> engine -> client download path.
- Applied metadata duplicate suffix behavior starting at `_2` while preserving legacy naming behavior for non-metadata paths.

### File List

- `_bmad-output/implementation-artifacts/5-3-metadata-based-file-naming.md`
- `_bmad-output/implementation-artifacts/sprint-status.yaml`
- `migrations/20260217000004_add_queue_metadata_fields.sql`
- `src/download/client.rs`
- `src/download/engine.rs`
- `src/download/mod.rs`
- `src/lib.rs`
- `src/main.rs`
- `src/queue/item.rs`
- `src/queue/mod.rs`
- `tests/download_engine_integration.rs`

### Change Log

- 2026-02-17: Story created and marked ready-for-dev.
- 2026-02-17: Implemented metadata-based file naming and moved story to review.
- 2026-02-17: Code review fixes applied; story marked done.

## Party Mode Audit (AI)

Audit date: 2026-02-17  
Outcome: pass_with_actions  
Summary: High=0, Medium=2, Low=1

Findings:
- Medium: Author extraction behavior needs deterministic edge-case handling to avoid unstable filenames.
- Medium: Extension source precedence should be explicit when metadata filename builder is used.
- Low: Duplicate suffix convention (`_2`) should be validated by direct test assertions.

## Senior Developer Review (AI)

Reviewer: fierce  
Date: 2026-02-17  
Outcome: Approve

### Findings Summary

- High: 0
- Medium: 1 (auto-fixed)
- Low: 2

### Validation Evidence

- `cargo fmt`
- `cargo clippy -- -D warnings`
- `cargo test --lib test_build_preferred_filename_`
- `cargo test --lib test_resolve_unique_path_with_metadata_suffix_starts_at_two`
- `cargo test --bin downloader`
- `cargo test --test cli_e2e test_binary_project_`
- `cargo test --test cli_e2e test_binary_without_project_keeps_default_output_layout`
- `cargo test --test download_engine_integration test_metadata_ -- --nocapture` (outside sandbox)
