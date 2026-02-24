# Story 5.2: Sub-Project Organization

Status: done

<!-- Note: Validation is optional. Run validate-create-story for quality check before dev-story. -->

## Story

As a **user**,
I want **to create nested project structures**,
so that **I can organize large research efforts**.

## Acceptance Criteria

1. **AC1: Nested folders from project path**
   - **Given** the `--project "Climate/Emissions/2024"` flag
   - **When** downloads complete
   - **Then** nested folders are created: `Climate/Emissions/2024/`

2. **AC2: Parent folders are created automatically**
   - **Given** parent folders do not yet exist
   - **When** nested project mode is used
   - **Then** missing parent folders are created

3. **AC3: Cross-platform separator behavior**
   - **Given** users pass `/` as separator on any OS
   - **When** project path is resolved
   - **Then** folder creation works correctly on all supported platforms

4. **AC4: Deep nesting support with reasonable bounds**
   - **Given** deeper project trees
   - **When** project path is processed
   - **Then** nesting works up to a defined safety limit

## Tasks / Subtasks

- [x] Task 1: Extend project path parser to support nested segments (AC: 1, 3)
  - [x] 1.1 Parse `--project` values by `/` separator
  - [x] 1.2 Retain segment sanitization rules from Story 5.1
  - [x] 1.3 Normalize path handling with `PathBuf` instead of string joins

- [x] Task 2: Enforce safe nested path constraints (AC: 2, 4)
  - [x] 2.1 Reject empty, traversal (`.`/`..`), or fully invalid segments
  - [x] 2.2 Add maximum nesting depth guard
  - [x] 2.3 Keep existing Windows reserved-name normalization per segment

- [x] Task 3: Wire nested output path into downloader flow (AC: 1-4)
  - [x] 3.1 Use nested path as effective output directory
  - [x] 3.2 Ensure parent directories are created recursively
  - [x] 3.3 Preserve non-project and single-level project behavior

- [x] Task 4: Add tests for nested project behavior (AC: 1-4)
  - [x] 4.1 Unit tests for nested path parsing and validation
  - [x] 4.2 E2E test for nested folder creation
  - [x] 4.3 E2E test for separator portability assumptions (`/`)
  - [x] 4.4 E2E test for depth-limit enforcement

### Review Follow-ups (AI)

- [x] [AI-Audit][Medium] Define explicit maximum nested depth constant and assert it in unit + e2e tests to prevent path abuse regressions.
- [x] [AI-Audit][Medium] Validate that each nested segment reuses Story 5.1 sanitization and reserved-name normalization independently.
- [x] [AI-Audit][Low] Add test coverage for repeated separators (`a//b///c`) to ensure empty segments are rejected cleanly.

## Dev Notes

### Architecture Context

- Story 5.1 introduced project-scoped output folders with sanitization and path-safety guards.
- Story 5.2 extends that logic to hierarchical project organization while preserving safety and portability.

### Implementation Guidance

- Keep path parsing logic centralized with Story 5.1 helpers; do not duplicate sanitization rules.
- Use recursive directory creation (`create_dir_all`) on the final nested path.
- Document and enforce a maximum segment depth to avoid abusive path construction.
- Keep error messages in What/Why/Fix format for invalid project path input.

### Testing Notes

- Add unit tests in `src/main.rs` for nested parsing and depth guards.
- Add e2e coverage in `tests/cli_e2e.rs` for nested folder creation and invalid-depth failures.
- Keep existing Story 5.1 project tests green.

### References

- [Source: _bmad-output/planning-artifacts/epics.md#Story-5.2-Sub-Project-Organization]
- [Source: _bmad-output/planning-artifacts/prd.md#FR-3.2]
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

- Added nested `--project` path support using `/` separators and `PathBuf` joining.
- Applied per-segment sanitization and Windows reserved-name normalization for nested project paths.
- Added max-depth guard (`MAX_PROJECT_SEGMENTS`) and explicit validation errors for empty/traversal segments.
- Added e2e coverage for nested folder creation, repeated separator rejection, and depth-limit enforcement.

### File List

- `_bmad-output/implementation-artifacts/5-2-sub-project-organization.md`
- `_bmad-output/implementation-artifacts/sprint-status.yaml`
- `src/main.rs`
- `tests/cli_e2e.rs`

### Change Log

- 2026-02-17: Story created and marked ready-for-dev.
- 2026-02-17: Implemented nested project-path support and moved story to review.
- 2026-02-17: Code review completed; story marked done.

## Party Mode Audit (AI)

Audit date: 2026-02-17  
Outcome: pass_with_actions  
Summary: High=0, Medium=2, Low=1

Findings:
- Medium: Depth constraints need a hard-coded, tested guard to avoid unbounded path expansion.
- Medium: Segment-level sanitization/normalization must be enforced per nested segment (not on the combined string only).
- Low: Repeated-separator input handling should be validated explicitly.

## Senior Developer Review (AI)

Reviewer: fierce  
Date: 2026-02-17  
Outcome: Approve

### Findings Summary

- High: 0
- Medium: 0
- Low: 3

### Low-Severity Notes

- Add a dedicated unit test for backslash normalization (`a\\b\\c` -> `a/b/c` behavior).
- Add one e2e assertion that already-existing nested parent directories are reused without side effects.
- Document rationale for `MAX_PROJECT_SEGMENTS` in user-facing help/docs.

### Validation Evidence

- `cargo fmt`
- `cargo clippy -- -D warnings`
- `cargo test --bin downloader`
- `cargo test --test cli_e2e test_binary_project_`
- `cargo test --test cli_e2e test_binary_without_project_keeps_default_output_layout`
