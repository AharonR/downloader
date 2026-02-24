# Story 3.3: In-Place Status Updates

Status: done

<!-- Note: Validation is optional. Run validate-create-story for quality check before dev-story. -->

## Story

As a **user**,
I want **progress updates without terminal scroll spam**,
so that **I can see status clearly**.

## Acceptance Criteria

1. **AC1: In-Place Updates**
   - **Given** multiple downloads completing
   - **When** status changes
   - **Then** status line updates in-place

2. **AC2: No Success-Line Spam**
   - **Given** completed items
   - **When** running in default mode
   - **Then** completed items do not produce persistent individual lines

3. **AC3: Persistent Error Visibility**
   - **Given** failed items
   - **When** failures occur
   - **Then** errors produce persistent output lines

4. **AC4: Terminal Capability Fallback**
   - **Given** non-interactive terminals
   - **When** in-place status is unsupported
   - **Then** output gracefully falls back without spinner rendering

## Tasks / Subtasks

- [x] Ensure interactive progress is rendered via spinner message updates (AC: 1)
- [x] Downgrade per-item success logs from info to debug to avoid scroll spam (AC: 2)
- [x] Preserve warning/error output lines for failed items (AC: 3)
- [x] Add TTY detection guard for spinner mode (AC: 4)

### Review Follow-ups (AI)

- [x] [AI-Audit][Medium] Keep default output focused on failures and summary; avoid per-item success lines. [src/download/engine.rs]
- [x] [AI-Audit][Medium] Confirm non-TTY operation remains functional without spinner assumptions. [src/main.rs]

## Dev Notes

- Success log in engine task completion was changed from `info!` to `debug!`.
- Failure path remains `warn!`, preserving persistent error visibility.
- Spinner loop runs only under TTY; non-interactive environments skip spinner entirely.

### References

- [Source: _bmad-output/planning-artifacts/epics.md#Story-3.3-In-Place-Status-Updates]
- [Source: src/main.rs]
- [Source: src/download/engine.rs]

## Dev Agent Record

### Agent Model Used

GPT-5 Codex (CLI)

### Completion Notes List

- Removed default per-item success-line noise.
- Kept failure lines persistent and actionable.
- Added terminal capability gated rendering.

### File List

- `src/download/engine.rs`
- `src/main.rs`
- `_bmad-output/implementation-artifacts/3-3-in-place-status-updates.md`

### Change Log

- 2026-02-15: Story created, implemented, reviewed, and marked done.

## Party Mode Audit (AI)

Audit date: 2026-02-15  
Outcome: pass_with_actions  
Summary: High=0, Medium=2, Low=0

Findings:
- Medium: Avoid emitting one line per successful item in default path.
- Medium: Ensure non-TTY path does not panic or emit broken control sequences.

## Senior Developer Review (AI)

Reviewer: fierce  
Date: 2026-02-15  
Outcome: Approve

### Findings Resolved

- Success item completion logs moved to debug-level.
- TTY detection guards spinner setup and teardown paths.

### Validation Evidence

- `cargo fmt --all`
- `cargo clippy -- -D warnings`
- `cargo test --bin downloader`
- `cargo test --test cli_e2e`
