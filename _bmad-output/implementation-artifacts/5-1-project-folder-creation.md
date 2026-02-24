# Story 5.1: Project Folder Creation

Status: done

<!-- Note: Validation is optional. Run validate-create-story for quality check before dev-story. -->

## Story

As a **user**,
I want **to organize downloads into a project folder**,
so that **my files are grouped by research topic**.

## Acceptance Criteria

1. **AC1: Project folder creation from CLI flag**
   - **Given** the `--project "Climate Research"` flag
   - **When** downloads complete
   - **Then** a folder `Climate-Research` is created (sanitized name)

2. **AC2: Downloads are placed in the project folder**
   - **Given** project mode is enabled
   - **When** files are downloaded
   - **Then** files are written under the resolved project folder

3. **AC3: Completion summary includes folder location**
   - **Given** a run with project mode
   - **When** processing finishes
   - **Then** summary output reports the project folder location

4. **AC4: Existing folders are reused**
   - **Given** a sanitized project folder already exists
   - **When** the downloader runs again with the same `--project` value
   - **Then** the existing folder is reused and not duplicated

5. **AC5: Default output location remains configurable**
   - **Given** a custom `--output-dir`
   - **When** project mode is enabled
   - **Then** the project folder is created under that output directory

## Tasks / Subtasks

- [x] Task 1: Add project flag and output path resolution (AC: 1, 5)
  - [x] 1.1 Add `--project` CLI argument in `src/cli.rs`
  - [x] 1.2 Implement project-name sanitization (`Climate Research` -> `Climate-Research`)
  - [x] 1.3 Resolve effective output path from `output_dir + sanitized_project`

- [x] Task 2: Route downloads to project folder (AC: 2, 4)
  - [x] 2.1 Ensure project folder is created when missing
  - [x] 2.2 Reuse existing folder if already present
  - [x] 2.3 Keep existing queue/state behavior intact under project folder

- [x] Task 3: Surface project folder in completion output (AC: 3)
  - [x] 3.1 Add completion-summary line for project folder location
  - [x] 3.2 Preserve existing summary behavior for non-project runs

- [x] Task 4: Add tests for project folder behavior (AC: 1-5)
  - [x] 4.1 CLI parsing tests for `--project`
  - [x] 4.2 Integration test for sanitized folder creation
  - [x] 4.3 Integration test for custom `--output-dir` + `--project`
  - [x] 4.4 Integration test for folder reuse behavior

### Review Follow-ups (AI)

- [x] [AI-Audit][Medium] Add explicit sanitization rules for filesystem-unsafe characters beyond whitespace (e.g., `:`, `?`, `*`, `<`, `>`), while preserving readable folder names.
- [x] [AI-Audit][Medium] Define and test project-path traversal guards (reject/normalize `..` and empty segments) before constructing output directories.
- [x] [AI-Audit][Low] Add one regression test verifying non-project runs keep existing output behavior unchanged.

### Code Review Follow-ups (AI - 2026-02-17)

- [x] [AI-Review][Medium] Normalize Windows reserved folder names (`CON`, `PRN`, `AUX`, etc.) to safe project folder names.
- [x] [AI-Review][Medium] Bound sanitized project folder length to reduce path-length risk and improve cross-platform reliability.
- [x] [AI-Review][Low] Normalize leading-dot project names to avoid accidental hidden-folder creation.

## Dev Notes

### Architecture Context

- Organization requirements are explicit in epic/PRD artifacts: project folders, naming, and indexing are part of the output system.
- Current binary flow already supports `--output-dir`; this story adds project scoping on top of it without breaking existing defaults.

### Implementation Guidance

- Prefer a small, testable helper for project-path derivation and sanitization.
- Keep path handling cross-platform by using `PathBuf` and avoiding string concatenation for filesystem paths.
- Preserve non-project behavior exactly (`output_dir` unchanged when `--project` is absent).
- Avoid introducing new dependencies unless absolutely required.

### Testing Notes

- Cover CLI argument parsing in `src/cli.rs` unit tests.
- Cover end-to-end behavior through binary integration tests in `tests/cli_e2e.rs`.
- Validate that repeated runs do not create duplicate sibling project directories.

### References

- [Source: _bmad-output/planning-artifacts/epics.md#Story-5.1-Project-Folder-Creation]
- [Source: _bmad-output/planning-artifacts/prd.md#FR-3.1]
- [Source: _bmad-output/planning-artifacts/prd.md#FR-5.2]
- [Source: _bmad-output/project-context.md#Platform-Compatibility]

## Dev Agent Record

### Agent Model Used

GPT-5 Codex (CLI)

### Debug Log References

- `cargo fmt`
- `cargo clippy -- -D warnings`
- `cargo test --bin downloader`
- `cargo test --test cli_e2e test_binary_project_`
- `cargo test --test cli_e2e test_binary_without_project_keeps_default_output_layout`

### Completion Notes List

- Added `--project` CLI flag and integrated project-scoped output path resolution.
- Added project-name sanitization that converts whitespace and filesystem-unsafe characters to a stable folder name.
- Added traversal/input guards for invalid project names (`.`, `..`, path separators).
- Updated completion summary output to include project folder location when project mode is enabled.
- Added CLI and e2e coverage for project creation, folder reuse, and non-project regression behavior.
- Code review auto-fixes added reserved-name normalization, folder-name length bounds, and leading-dot normalization.

### File List

- `_bmad-output/implementation-artifacts/5-1-project-folder-creation.md`
- `_bmad-output/implementation-artifacts/sprint-status.yaml`
- `src/cli.rs`
- `src/main.rs`
- `tests/cli_e2e.rs`

### Change Log

- 2026-02-17: Story created and marked ready-for-dev.
- 2026-02-17: Implemented project-folder creation flow and moved story to review.
- 2026-02-17: Code review auto-fixes applied; story marked done.

## Party Mode Audit (AI)

Audit date: 2026-02-17  
Outcome: pass_with_actions  
Summary: High=0, Medium=2, Low=1

Findings:
- Medium: Sanitization scope is currently underspecified for unsafe filesystem characters; this can produce platform-specific failures.
- Medium: Story should explicitly constrain path traversal inputs in project flag handling.
- Low: Add a guardrail regression test for non-project output-path behavior.

## Senior Developer Review (AI)

Reviewer: fierce  
Date: 2026-02-17  
Outcome: Approve

### Findings Summary

- High: 0
- Medium: 2 (auto-fixed)
- Low: 1 (addressed)

### Validation Evidence

- `cargo fmt`
- `cargo clippy -- -D warnings`
- `cargo test --bin downloader`
- `cargo test --test cli_e2e test_binary_project_`
- `cargo test --test cli_e2e test_binary_without_project_keeps_default_output_layout`
