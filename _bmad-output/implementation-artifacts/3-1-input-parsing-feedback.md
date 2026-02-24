# Story 3.1: Input Parsing Feedback

Status: done

<!-- Note: Validation is optional. Run validate-create-story for quality check before dev-story. -->

## Story

As a **user**,
I want **immediate feedback on what was parsed from my input**,
so that **I trust the tool understood me correctly**.

## Acceptance Criteria

1. **AC1: Parsing Summary by Type**
   - **Given** input containing various item types
   - **When** parsing completes
   - **Then** a summary is displayed: "Parsed X items: Y URLs, Z DOIs, W references"

2. **AC2: Uncertain Parse Flagging**
   - **Given** parses with uncertain reference quality
   - **When** summary is displayed
   - **Then** uncertain parses are flagged: "(N references need verification)"

3. **AC3: Summary Before Downloads**
   - **Given** items were parsed
   - **When** execution continues to download/resolution
   - **Then** the parsing summary appears before downloading begins

4. **AC4: Terminal Width Respect**
   - **Given** a narrow terminal
   - **When** summary output is rendered
   - **Then** output is truncated safely to fit terminal width constraints

## Tasks / Subtasks

- [x] **Task 1: Add parse-feedback summary formatter in CLI flow** (AC: 1, 3)
  - [x] Add helper to build deterministic summary text from `ParseResult` type counts
  - [x] Emit summary immediately after parse and before queue/download work

- [x] **Task 2: Add uncertain-reference signal** (AC: 2)
  - [x] Compute uncertain references using existing reference confidence analysis
  - [x] Append "(N references need verification)" when uncertain count > 0

- [x] **Task 3: Add terminal-width-safe rendering** (AC: 4)
  - [x] Detect width from `COLUMNS` when available with sane fallback
  - [x] Clamp summary string with ellipsis when width is exceeded

- [x] **Task 4: Tests and regression safety** (AC: 1-4)
  - [x] Add unit tests for summary formatting and width truncation behavior
  - [x] Validate CLI E2E suite remains green

### Review Follow-ups (AI)

- [x] [AI-Audit][Medium] Ensure parse feedback is emitted before resolver/queue setup and before any download status output to prevent UX ambiguity. [src/main.rs]
- [x] [AI-Audit][Medium] Add explicit truncation helper tests for very small widths and exact-width boundary behavior to prevent terminal overflow regressions. [src/main.rs]

## Dev Notes

### Story Context and Intent

- Epic 3 shifts UX from parsing capability to confidence and clarity during batch processing.
- Parser already reports type counts (`ParseTypeCounts`) and skipped items; this story makes feedback explicit and user-facing in CLI output order.

### Technical Requirements

- Reuse parser outputs and avoid duplicate parsing logic.
- Keep logging/output pipeline deterministic and non-panicking.
- Avoid adding dependencies for terminal sizing; use lightweight width detection with fallback.

### Architecture Compliance

- Keep changes localized to CLI orchestration (`src/main.rs`) and tests.
- Do not move parsing logic into download modules.
- Preserve existing error-handling conventions (`anyhow` in binary, structured library behavior).

### Testing Requirements

- Unit tests for summary formatting and truncation helper behavior.
- Run `cargo fmt --check`.
- Run `cargo clippy -- -D warnings`.
- Run targeted tests for CLI behavior and story scope.

### Project Context Reference

- Follow naming/import conventions from `_bmad-output/project-context.md`.
- No `unwrap`/`expect` in runtime paths.

### References

- [Source: _bmad-output/planning-artifacts/epics.md#Story-3.1-Input-Parsing-Feedback]
- [Source: _bmad-output/planning-artifacts/ux-design-specification.md]
- [Source: src/parser/input.rs]
- [Source: src/parser/reference.rs]

## Dev Agent Record

### Agent Model Used

GPT-5 Codex (CLI)

### Debug Log References

- create-story executed via epic-auto-flow invoke.

### Completion Notes List

- Story scaffold generated with AC-aligned tasks and implementation guardrails.
- Added parse-feedback summary output with deterministic per-type counts.
- Added uncertain-reference signal using reference confidence analysis.
- Added terminal-width-safe truncation with ellipsis fallback.
- Added unit tests for summary formatting and truncation edge cases.
- Validated with `cargo fmt --check`, `cargo clippy -- -D warnings`, `cargo test --bin downloader`, and `cargo test --test cli_e2e`.
- Code-review auto-fix applied: uncertain parse suffix now triggers on low confidence references only.
- Code-review auto-fix applied: parse summary now emits only when parsed item count is non-zero.

### File List

- `_bmad-output/implementation-artifacts/3-1-input-parsing-feedback.md`
- `src/main.rs`
- `tests/cli_e2e.rs`
- `_bmad-output/implementation-artifacts/sprint-status.yaml`

### Change Log

- 2026-02-15: Story created and marked ready-for-dev.
- 2026-02-15: Implemented parse feedback summary UX and moved story to review.
- 2026-02-15: Code-review auto-fix pass completed and story marked done.

## Party Mode Audit (AI)

Audit date: 2026-02-15  
Outcome: pass_with_actions  
Summary: High=0, Medium=2, Low=1

Findings:
- Medium: Parse feedback could drift later in flow if placed after resolver/queue setup; should be emitted immediately post-parse for trust-first UX.
- Medium: Width-safe formatting needs explicit edge-case tests (tiny terminal widths, exact fit boundaries) to avoid regressions.
- Low: Story wording and output examples should keep item labels consistent (`URLs`, `DOIs`, `references`, `BibTeX`) for clarity.

## Senior Developer Review (AI)

Reviewer: fierce  
Date: 2026-02-15  
Outcome: Approve

### Findings Resolved

- Medium: Uncertain parse signal was overly aggressive for medium-confidence references; adjusted to low-confidence-only verification warnings.
- Medium: Parse summary logged even for empty parse results; moved summary logging to non-empty parse path.
- Medium: Missing regression guard for medium-confidence suffix behavior; added unit test coverage.

### Validation Evidence

- `cargo fmt --check`
- `cargo clippy -- -D warnings`
- `cargo test --bin downloader`
- `cargo test --test cli_e2e`
