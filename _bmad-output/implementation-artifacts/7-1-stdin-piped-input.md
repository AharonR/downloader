# Story 7.1: Stdin Piped Input

Status: done

<!-- Note: Validation is optional. Run validate-create-story for quality check before dev-story. -->

## Story

As a **user**,
I want **to pipe input from other commands**,
so that **I can integrate with shell workflows**.

## Acceptance Criteria

1. **AC1: Read piped stdin fully before processing**
   - **Given** piped input like `cat refs.txt | downloader`
   - **When** the tool reads stdin
   - **Then** input is read completely before processing begins

2. **AC2: Detect interactive vs piped stdin correctly**
   - **Given** stdin state
   - **When** the CLI evaluates input sources
   - **Then** stdin detection uses terminal checks (`isatty` / `IsTerminal`)

3. **AC3: Combine stdin with positional/file arguments**
   - **Given** positional URL arguments and piped stdin
   - **When** the command runs
   - **Then** both sources are accepted in one run

4. **AC4: Handle empty stdin with helpful guidance**
   - **Given** piped stdin that is empty (or whitespace only)
   - **When** no usable input items are found
   - **Then** output is a helpful guidance message, not an opaque error

## Tasks / Subtasks

- [x] Task 1: Refactor input collection path to support multi-source aggregation (AC: 1, 2, 3)
  - [x] 1.1 Keep terminal detection explicit via `std::io::IsTerminal` for stdin mode branching
  - [x] 1.2 Aggregate positional URL arguments and piped stdin into one combined input payload
  - [x] 1.3 Preserve existing cookie-stdin conflict guardrails (`--cookies -` behavior)

- [x] Task 2: Add explicit empty-stdin guidance flow (AC: 4)
  - [x] 2.1 Detect piped-but-empty stdin input deterministically
  - [x] 2.2 Emit clear next-step guidance message and exit successfully when no queue items are created

- [x] Task 3: Add/adjust tests for stdin behavior guarantees (AC: 1, 2, 3, 4)
  - [x] 3.1 Add CLI E2E coverage for combined positional + stdin input behavior
  - [x] 3.2 Add CLI E2E coverage for empty-stdin guidance message
  - [x] 3.3 Keep existing stdin invalid-input tests passing as regression guardrails

### Review Follow-ups (AI)

- [x] [AI-Audit][Medium] Define and implement deterministic merge semantics when positional URLs and piped stdin are both present, including ordering and duplicate handling behavior.
- [x] [AI-Audit][Medium] Ensure empty-stdin guidance does not incorrectly trigger when resumable queue state exists, and verify the user-facing message is explicit about next steps.

## Dev Notes

### Architecture Context

- CLI argument modeling is in `src/cli.rs`; command runtime behavior is in `src/main.rs`.
- Existing input pipeline resolves and parses input before queue enqueueing, then runs download engine.
- Existing story implementations rely on deterministic non-network E2E patterns in `tests/cli_e2e.rs`.

### Implementation Guidance

- Keep logic in `main` side-effect free until input source resolution is complete.
- Do not introduce new dependencies for stdin handling; use current std + clap patterns.
- Ensure combined-source behavior is deterministic and does not regress existing resume/auth flows.

### Testing Notes

- Prefer parser-rejected tokens for deterministic tests (avoid network dependency).
- Verify success exit behavior for empty stdin plus clear human-readable message content.
- Ensure tests cover precedence interaction among: positional URLs, piped stdin, and `--cookies -`.

### Project Structure Notes

- Expected code touch points: `src/main.rs`, `tests/cli_e2e.rs`.
- Optional touch point only if needed: `src/cli.rs` (help text clarification).

### References

- [Source: _bmad-output/planning-artifacts/epics.md#Story-7.1-Stdin-Piped-Input]
- [Source: _bmad-output/planning-artifacts/prd.md#FR-5-CLI-Interface]
- [Source: _bmad-output/planning-artifacts/architecture.md#Requirements-Overview]
- [Source: _bmad-output/project-context.md#Framework-Specific-Rules]

## Dev Agent Record

### Agent Model Used

GPT-5 Codex (CLI)

### Debug Log References

- create-story via epic-auto-flow
- cargo fmt
- cargo test --test cli_e2e test_binary_combines_positional_and_stdin_inputs -- --nocapture
- cargo test --test cli_e2e test_binary_empty_stdin -- --nocapture
- cargo test --test cli_e2e stdin -- --nocapture
- cargo test --bin downloader test_validate_cookie_stdin_conflict -- --nocapture
- cargo clippy --bin downloader --test cli_e2e -- -D warnings (blocked by pre-existing `clippy::too_many_arguments` in `src/queue/history.rs`)

### Completion Notes List

- 2026-02-17: Story created with implementation guardrails and set to ready-for-dev.
- 2026-02-17: Updated input collection flow to combine positional URL arguments and piped stdin in one deterministic parse payload.
- 2026-02-17: Added explicit empty-stdin guidance path and avoided false guidance when prior queue state exists.
- 2026-02-17: Added E2E coverage for combined input behavior and empty-stdin guidance paths.
- 2026-02-17: Senior code review completed with no remaining High/Medium issues; story accepted as done.

### File List

- _bmad-output/implementation-artifacts/7-1-stdin-piped-input.md
- _bmad-output/implementation-artifacts/sprint-status.yaml
- src/main.rs
- tests/cli_e2e.rs

### Change Log

- 2026-02-17: Story created and marked ready-for-dev.
- 2026-02-17: Party mode audit completed with follow-up actions.
- 2026-02-17: Implemented stdin aggregation and empty-stdin guidance; story moved to review.
- 2026-02-17: Code review completed; story marked done.

## Party Mode Audit (AI)

Audit date: 2026-02-17  
Outcome: pass_with_actions  
Summary: High=0, Medium=2, Low=1

Findings:
- Medium: Combined stdin + positional argument behavior is not yet explicit about merge ordering and duplicate handling, which can cause inconsistent queue expectations.
- Medium: Empty-stdin UX requirements can regress resume behavior if guidance handling is not conditioned on existing queue state.
- Low: Stdin-path coverage should explicitly guard cookie-stdin conflict semantics during mixed-input execution.

## Senior Developer Review (AI)

Reviewer: fierce  
Date: 2026-02-17  
Outcome: Approve

### Findings Summary

- High: 0
- Medium: 0
- Low: 3

### Fixed During Review (Auto-Fix High/Medium)

- Verified AC1/AC2: stdin is read via `read_to_string` and terminal detection remains explicit through `IsTerminal`.
- Verified AC3: positional URL arguments and piped stdin are merged into one deterministic parse payload.
- Verified AC4: empty-stdin guidance is emitted only when no prior queue state exists.

### Low-Severity Notes

- Add a dedicated regression for whitespace-only stdin combined with positional URLs to lock in current merge behavior.
- Consider extracting input-source merge logic into a small helper to simplify future maintenance and unit testing.
- Document mixed-input CLI behavior in user-facing README examples once Epic 7 CLI polish stories are complete.
