# Story 7.2: Dry Run Mode

Status: done

<!-- Note: Validation is optional. Run validate-create-story for quality check before dev-story. -->

## Story

As a **user**,
I want **to preview what would be downloaded**,
so that **I can verify parsing before committing**.

## Acceptance Criteria

1. **AC1: Expose dry-run CLI flag**
   - **Given** the `--dry-run` or `-n` flag
   - **When** I run the command
   - **Then** dry-run mode is activated deterministically

2. **AC2: Parse and display candidate input**
   - **Given** dry-run mode with input
   - **When** parsing completes
   - **Then** parsed input is displayed for preview

3. **AC3: Show resolved URLs**
   - **Given** DOI/reference/URL inputs
   - **When** resolution is attempted in dry-run mode
   - **Then** resolved downloadable URLs are shown (including successful DOI/reference resolution paths)

4. **AC4: Ensure no download side effects**
   - **Given** dry-run mode
   - **When** the command finishes
   - **Then** no files are downloaded
   - **And** no database records are created

5. **AC5: Explicit dry-run completion messaging**
   - **Given** dry-run execution
   - **When** output is rendered
   - **Then** output clearly states: `Dry run - no files downloaded`

## Tasks / Subtasks

- [x] Task 1: Add dry-run CLI surface and command-state plumbing (AC: 1)
  - [x] 1.1 Add `--dry-run` / `-n` flag in `src/cli.rs` download arguments
  - [x] 1.2 Thread parsed flag through the main command path in `src/main.rs`

- [x] Task 2: Implement dry-run execution path before queue/download lifecycle (AC: 2, 3, 4, 5)
  - [x] 2.1 Parse inputs and preserve current parse summary/skipped diagnostics behavior
  - [x] 2.2 Resolve each parsed item through existing resolver registry and show resolved URLs
  - [x] 2.3 Bypass queue/database initialization and download engine execution in dry-run mode
  - [x] 2.4 Emit explicit completion line: `Dry run - no files downloaded`

- [x] Task 3: Add deterministic tests for dry-run guarantees (AC: 1-5)
  - [x] 3.1 CLI parser tests for `--dry-run` and `-n`
  - [x] 3.2 CLI E2E test confirms dry-run output includes explicit completion message
  - [x] 3.3 CLI E2E test confirms no `.downloader/queue.db` is created in dry-run mode
  - [x] 3.4 CLI E2E test confirms resolved URL preview for direct URL input

### Review Follow-ups (AI)

- [x] [AI-Audit][Medium] Ensure dry-run mode short-circuits before any queue/database initialization path, including resume-state branches.
- [x] [AI-Audit][Medium] Standardize dry-run preview output format so parsed-item and resolved-URL lines are deterministic for CLI scripting/tests.
- [x] [AI-Audit][Low] Add regression check that dry-run honors cookie-stdin conflict validation consistently with normal mode.

## Dev Notes

### Architecture Context

- Current runtime path in `src/main.rs` parses input, initializes queue DB, enqueues resolved URLs, then downloads.
- Resolver pipeline already exists via `ResolverRegistry` (`ScienceDirect`, `Crossref`, `Direct`) and should be reused for preview fidelity.
- Output behavior already uses both `tracing` logs and CLI text output in command paths.

### Implementation Guidance

- Keep dry-run behavior as an early execution branch to guarantee no queue/download side effects.
- Reuse current parsing and resolver logic paths to avoid divergence between preview and real execution.
- Preserve existing cookie-stdin conflict and auth namespace guardrails.

### Testing Notes

- Prefer direct URL inputs for deterministic resolved-url preview tests.
- Avoid external network dependency in tests; do not require Crossref availability for baseline dry-run coverage.
- Assert absence of queue DB artifact under the provided output directory to prove side-effect constraints.

### Project Structure Notes

- Expected touch points: `src/cli.rs`, `src/main.rs`, `tests/cli_e2e.rs`.
- Optional supporting tests: `src/cli.rs` unit tests.

### References

- [Source: _bmad-output/planning-artifacts/epics.md#Story-7.2-Dry-Run-Mode]
- [Source: _bmad-output/planning-artifacts/prd.md#FR-5-CLI-Interface]
- [Source: _bmad-output/planning-artifacts/architecture.md#Requirements-Overview]
- [Source: _bmad-output/project-context.md#Framework-Specific-Rules]

## Dev Agent Record

### Agent Model Used

GPT-5 Codex (CLI)

### Debug Log References

- create-story via epic-auto-flow
- cargo fmt
- cargo test --bin downloader test_cli_dry_run -- --nocapture
- cargo test --test cli_e2e dry_run -- --nocapture
- cargo clippy --bin downloader --test cli_e2e -- -D warnings (blocked by pre-existing `clippy::too_many_arguments` in `src/queue/history.rs`)
- code-review follow-up: flattened unresolved dry-run preview lines to single-line output
- code-review follow-up: added project-scoped dry-run no-DB regression test

### Completion Notes List

- 2026-02-17: Story created and set to ready-for-dev.
- 2026-02-17: Added `--dry-run` / `-n` CLI flag and threaded dry-run mode through download command args.
- 2026-02-17: Implemented dry-run preview path that parses input, resolves URLs, and exits before queue/database/download lifecycle.
- 2026-02-17: Added deterministic dry-run tests for explicit completion output, resolved URL preview, no queue DB creation, and cookie/stdin conflict guardrail.
- 2026-02-17: Senior code review completed with no remaining High/Medium issues; story accepted as done.
- 2026-02-17: Follow-up review auto-fixed single-line dry-run unresolved formatting and expanded no-DB coverage for project-scoped dry-runs.

### File List

- _bmad-output/implementation-artifacts/7-2-dry-run-mode.md
- _bmad-output/implementation-artifacts/sprint-status.yaml
- src/cli.rs
- src/main.rs
- tests/cli_e2e.rs

### Change Log

- 2026-02-17: Story created and marked ready-for-dev.
- 2026-02-17: Party mode audit completed with follow-up actions.
- 2026-02-17: Implemented dry-run mode and moved story to review.
- 2026-02-17: Code review completed; story marked done.
- 2026-02-17: Follow-up code review fixes applied (output determinism + project dry-run no-DB regression).

## Party Mode Audit (AI)

Audit date: 2026-02-17  
Outcome: pass_with_actions  
Summary: High=0, Medium=2, Low=1

Findings:
- Medium: Dry-run must bypass queue/database initialization in all branches, including existing resume-aware state checks.
- Medium: Preview output needs deterministic formatting to avoid brittle tests and unclear scripting behavior.
- Low: Cookie-stdin guardrail behavior should remain explicitly validated in dry-run mode.

## Senior Developer Review (AI)

Reviewer: fierce  
Date: 2026-02-17  
Outcome: Approve

### Findings Summary

- High: 0
- Medium: 0
- Low: 2

### Fixed During Review (Auto-Fix High/Medium)

- Verified dry-run short-circuits before output-directory creation, queue DB initialization, enqueueing, and download execution.
- Verified dry-run output includes deterministic preview lines plus explicit completion text: `Dry run - no files downloaded`.
- Verified regression coverage for no-DB side effect and cookie/stdin conflict behavior.

### Low-Severity Notes

- Consider extracting resolver-preview formatting into a small helper for easier future UX refinement.
- If future stories require machine parsing, consider a structured preview mode (JSON) in addition to human-readable dry-run output.

## Senior Developer Review (AI) - Follow-up

Reviewer: fierce  
Date: 2026-02-17  
Outcome: Approve

### Findings Summary

- High: 0
- Medium: 1 (fixed)
- Low: 2 (1 fixed, 1 open)

### Fixed During Review (Auto-Fix High/Medium)

- **Medium (fixed):** Dry-run unresolved preview output embedded multi-line resolver error text, making preview lines non-deterministic for scripts.  
  Fix: unresolved error text is normalized to single-line whitespace in preview output (`preview_single_line`).

### Additional Safe Fixes

- **Low (fixed):** Added regression test proving `--dry-run --project` does not create `.downloader/queue.db`.

### Remaining Decision Items

- **Low (decision):** Whether to introduce a structured dry-run preview output mode (e.g., JSON) for machine consumers versus keeping human-readable text only.
