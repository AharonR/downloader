# Story 3.2: Progress Spinner Display

Status: done

<!-- Note: Validation is optional. Run validate-create-story for quality check before dev-story. -->

## Story

As a **user**,
I want **to see active progress during downloads**,
so that **I know the tool is working**.

## Acceptance Criteria

1. **AC1: Animated Activity Indicator**
   - **Given** downloads in progress
   - **When** the terminal is displayed
   - **Then** an animated spinner shows activity

2. **AC2: Item Counter Display**
   - **Given** active download processing
   - **When** progress is rendered
   - **Then** current item count is shown as `[N/Total]`

3. **AC3: Current Domain Visibility**
   - **Given** a currently active download item
   - **When** status updates
   - **Then** current domain is shown as `Downloading from <domain>...`

4. **AC4: Indicatif Rendering**
   - **Given** CLI progress output
   - **When** spinner is displayed
   - **Then** `indicatif` library is used for rendering

## Tasks / Subtasks

- [x] Add terminal spinner rendering using `indicatif` (AC: 1, 4)
- [x] Add queue-polled progress message with current count `[N/Total]` (AC: 2)
- [x] Add current-domain extraction from in-progress queue items (AC: 3)
- [x] Gate spinner to TTY mode and disable in quiet mode (AC: 1-4)

### Review Follow-ups (AI)

- [x] [AI-Audit][Medium] Confirm spinner message includes both count and domain from live queue state. [src/main.rs]
- [x] [AI-Audit][Medium] Ensure `indicatif` is the rendering path for interactive terminals. [src/main.rs]

## Dev Notes

- Implemented spinner loop via `spawn_spinner()` with `ProgressBar::new_spinner()` and steady tick.
- Spinner message format now includes `[current/total]` and domain extracted from the active queue URL host.
- Spinner runs only when stderr is TTY and `--quiet` is not set.

### References

- [Source: _bmad-output/planning-artifacts/epics.md#Story-3.2-Progress-Spinner-Display]
- [Source: src/main.rs]

## Dev Agent Record

### Agent Model Used

GPT-5 Codex (CLI)

### Completion Notes List

- Added `indicatif` spinner-driven progress updates.
- Added count + domain status message format from queue state.
- Kept spinner non-intrusive and TTY-aware.

### File List

- `src/main.rs`
- `_bmad-output/implementation-artifacts/3-2-progress-spinner-display.md`

### Change Log

- 2026-02-15: Story created, implemented, reviewed, and marked done.

## Party Mode Audit (AI)

Audit date: 2026-02-15  
Outcome: pass_with_actions  
Summary: High=0, Medium=2, Low=0

Findings:
- Medium: Ensure progress count reflects queue terminal states and not only in-progress count.
- Medium: Ensure domain rendering uses active in-progress item and degrades safely when unavailable.

## Senior Developer Review (AI)

Reviewer: fierce  
Date: 2026-02-15  
Outcome: Approve

### Findings Resolved

- Progress line now computes count from completed+failed with in-progress increment.
- Domain extraction now safely falls back to `queue` label when parsing fails.

### Validation Evidence

- `cargo fmt --all`
- `cargo clippy -- -D warnings`
- `cargo test --bin downloader`
- `cargo test --test cli_e2e`
