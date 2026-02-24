# Story 3.4: Completion Summary

Status: done

<!-- Note: Validation is optional. Run validate-create-story for quality check before dev-story. -->

## Story

As a **user**,
I want **a clear summary when all downloads finish**,
so that **I know what succeeded and what needs attention**.

## Acceptance Criteria

1. **AC1: Success Count Summary**
   - **Given** batch completion
   - **When** summary is displayed
   - **Then** success count is shown (`✓ X/Y downloaded successfully`)

2. **AC2: Output Location Display**
   - **Given** downloads are complete
   - **When** summary is shown
   - **Then** output location is shown

3. **AC3: Grouped Failures with Next Steps**
   - **Given** failed downloads
   - **When** summary is shown
   - **Then** failures are grouped by type with actionable next step

4. **AC4: Visually Distinct Summary Block**
   - **Given** completion output
   - **When** displayed
   - **Then** summary is rendered in a distinct box/separator block

## Tasks / Subtasks

- [x] Add explicit completion summary renderer (AC: 1, 2, 4)
- [x] Add failure grouping taxonomy (auth/not found/timeout/network/other) (AC: 3)
- [x] Add next-step hints per failure group (AC: 3)

### Review Follow-ups (AI)

- [x] [AI-Audit][Medium] Ensure failure grouping uses queue terminal state, not transient retry errors. [src/main.rs]
- [x] [AI-Audit][Medium] Ensure summary block remains visible even when failures are empty. [src/main.rs]

## Dev Notes

- Added `print_completion_summary()` in CLI to render a boxed summary section.
- Summary includes success ratio, output path, and grouped failures with next-step guidance.
- Failure groups derive from persisted queue `last_error` text.

### References

- [Source: _bmad-output/planning-artifacts/epics.md#Story-3.4-Completion-Summary]
- [Source: src/main.rs]

## Dev Agent Record

### Agent Model Used

GPT-5 Codex (CLI)

### Completion Notes List

- Added completion box with clear success ratio and output directory.
- Added grouped failure summary and actionable fix guidance.

### File List

- `src/main.rs`
- `_bmad-output/implementation-artifacts/3-4-completion-summary.md`

### Change Log

- 2026-02-15: Story created, implemented, reviewed, and marked done.

## Party Mode Audit (AI)

Audit date: 2026-02-15  
Outcome: pass_with_actions  
Summary: High=0, Medium=2, Low=0

Findings:
- Medium: Keep completion summary visually separated from trace logs.
- Medium: Group failures by user-actionable categories instead of raw status codes only.

## Senior Developer Review (AI)

Reviewer: fierce  
Date: 2026-02-15  
Outcome: Approve

### Findings Resolved

- Added separator box around summary output.
- Added grouped failure classification with explicit next-step text.

### Validation Evidence

- `cargo fmt --all`
- `cargo clippy -- -D warnings`
- `cargo test --bin downloader`
- `cargo test --test cli_e2e`
