# Story 3.5: Graceful Interrupt Handling

Status: done

<!-- Note: Validation is optional. Run validate-create-story for quality check before dev-story. -->

## Story

As a **user**,
I want **Ctrl+C to stop cleanly without losing progress**,
so that **I can safely interrupt long batches**.

## Acceptance Criteria

1. **AC1: Graceful Ctrl+C Behavior**
   - **Given** batch download in progress
   - **When** user presses Ctrl+C
   - **Then** current downloads are allowed to complete or timed out after 5s

2. **AC2: Queue State Preserved**
   - **Given** interrupted run
   - **When** application exits
   - **Then** queue state is preserved in SQLite for resume

3. **AC3: Partial Summary**
   - **Given** interruption occurred
   - **When** run exits
   - **Then** partial completion summary is displayed

4. **AC4: Exit Code + Interrupt Message**
   - **Given** interruption occurred
   - **When** process exits
   - **Then** exit code is non-zero and message includes `Interrupted. X/Y completed. Run again to resume.`

## Tasks / Subtasks

- [x] Add Ctrl+C signal handling in CLI (`tokio::signal::ctrl_c`) (AC: 1, 4)
- [x] Add interrupt-aware queue processing path with 5s drain timeout (AC: 1)
- [x] Move queue database to persistent SQLite file under output state dir (AC: 2)
- [x] Reset stale in-progress rows on startup for crash/interrupt recovery (AC: 2)
- [x] Emit interrupt summary + message and return non-zero (AC: 3, 4)

### Review Follow-ups (AI)

- [x] [AI-Audit][Medium] Ensure interrupted tasks do not claim new queue work after signal flag is set. [src/download/engine.rs]
- [x] [AI-Audit][Medium] Ensure startup recovery resets stale `in_progress` items. [src/main.rs]

## Dev Notes

- Added `process_queue_interruptible()` to engine with shared atomic interrupt flag.
- Engine stops dequeuing on interrupt and waits up to 5 seconds for active tasks, then aborts remaining.
- Queue database moved from in-memory to `${output_dir}/.downloader/queue.db` for persistence.
- Startup now calls `queue.reset_in_progress()` to recover stale in-progress rows.

### References

- [Source: _bmad-output/planning-artifacts/epics.md#Story-3.5-Graceful-Interrupt-Handling]
- [Source: src/main.rs]
- [Source: src/download/engine.rs]

## Dev Agent Record

### Agent Model Used

GPT-5 Codex (CLI)

### Completion Notes List

- Added signal handling and interrupt-aware queue processing.
- Added persistent queue DB and restart recovery path.
- Added non-zero interruption exit behavior with explicit resume guidance.

### File List

- `src/main.rs`
- `src/download/engine.rs`
- `_bmad-output/implementation-artifacts/3-5-graceful-interrupt-handling.md`

### Change Log

- 2026-02-15: Story created, implemented, reviewed, and marked done.

## Party Mode Audit (AI)

Audit date: 2026-02-15  
Outcome: pass_with_actions  
Summary: High=0, Medium=2, Low=0

Findings:
- Medium: Ensure queue state survives process restart, not only same-process retries.
- Medium: Ensure interrupt path drains in-flight tasks with explicit timeout cap.

## Senior Developer Review (AI)

Reviewer: fierce  
Date: 2026-02-15  
Outcome: Approve

### Findings Resolved

- Added persistent SQLite queue at `.downloader/queue.db`.
- Added interrupt drain timeout and non-zero interrupt completion path.

### Validation Evidence

- `cargo fmt --all`
- `cargo clippy -- -D warnings`
- `cargo test --bin downloader`
- `cargo test --test queue_integration`
