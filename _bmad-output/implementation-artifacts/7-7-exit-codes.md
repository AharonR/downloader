# Story 7.7: Exit Codes

Status: done

<!-- Note: Validation is optional. Run validate-create-story for quality check before dev-story. -->

## Story

As a **user**,
I want **standard exit codes**,
so that **I can use the tool in scripts**.

## Acceptance Criteria

1. **AC1: Success exit code**
   - **Given** a successful run
   - **When** process exits
   - **Then** exit code is `0`

2. **AC2: Partial-success exit code**
   - **Given** a run where some items succeed and some fail
   - **When** process exits
   - **Then** exit code is `1`

3. **AC3: Failure/fatal exit code**
   - **Given** complete failure (none succeeded) or fatal runtime error
   - **When** process exits
   - **Then** exit code is `2`

4. **AC4: Help documentation**
   - **Given** `downloader --help`
   - **When** help output is displayed
   - **Then** exit code semantics are documented

## Tasks / Subtasks

- [x] Task 1: Introduce explicit process-exit classification path (AC: 1, 2, 3)
  - [x] 1.1 Add runtime exit-outcome enum and final exit-code mapping
  - [x] 1.2 Ensure fatal errors map to exit code `2`
  - [x] 1.3 Ensure mixed success/failure runs map to exit code `1`

- [x] Task 2: Wire exit-outcome decisions into download flow (AC: 1, 2, 3)
  - [x] 2.1 Derive outcome from completed/failed counts for completed queue runs
  - [x] 2.2 Preserve success exit (`0`) for no-input guidance and no-work success paths
  - [x] 2.3 Keep interrupt/fatal paths non-zero as complete failure (`2`)

- [x] Task 3: Document exit codes in CLI help (AC: 4)
  - [x] 3.1 Add explicit exit-code section to clap help text
  - [x] 3.2 Add parser/help regression coverage

- [x] Task 4: Add deterministic test coverage (AC: 1-4)
  - [x] 4.1 Unit tests for exit-outcome classification logic
  - [x] 4.2 E2E tests for success (`0`) and complete-failure (`2`) paths
  - [x] 4.3 E2E/help tests for exit-code documentation visibility

### Review Follow-ups (AI)

- [x] [AI-Audit][Medium] Ensure fatal runtime errors and complete-failure outcomes share a single deterministic non-zero exit class (`2`).
- [x] [AI-Audit][Medium] Add deterministic logic coverage for `0/1/2` mapping independent of network behavior.
- [x] [AI-Audit][Low] Document exit codes directly in `--help` output for script users.

## Dev Notes

### Architecture Context

- Entry-point lifecycle is in `src/main.rs`; clap help text is in `src/cli.rs`.
- Download batch stats (`completed`, `failed`) are already available after queue processing.
- Existing test suites include both unit (`src/main.rs`, `src/cli.rs`) and E2E (`tests/cli_e2e.rs`).

### Implementation Guidance

- Keep exit-outcome logic explicit and unit-testable.
- Avoid introducing network-dependent logic for core exit-code tests.
- Preserve current successful no-input UX behavior with exit code `0`.

### Testing Notes

- Validate unit-level `0/1/2` mapping deterministically.
- Validate E2E success and failure exit codes via deterministic non-network paths.
- Verify help text includes exit-code docs.

### Project Structure Notes

- Touch points: `src/main.rs`, `src/cli.rs`, `tests/cli_e2e.rs`.

### References

- [Source: _bmad-output/planning-artifacts/epics.md#Story-7.7-Exit-Codes]
- [Source: _bmad-output/planning-artifacts/prd.md#FR-5-CLI-Interface]
- [Source: _bmad-output/planning-artifacts/ux-design-specification.md#Error-Messaging]
- [Source: _bmad-output/planning-artifacts/architecture.md#Requirements-Overview]
- [Source: _bmad-output/project-context.md#Framework-Specific-Rules]

## Dev Agent Record

### Agent Model Used

GPT-5 Codex (CLI)

### Debug Log References

- create-story via epic-auto-flow
- cargo fmt
- cargo test --locked --bin downloader determine_exit_outcome -- --nocapture
- cargo test --locked --bin downloader test_cli_help_includes_exit_code_documentation -- --nocapture
- cargo test --locked --test cli_e2e exit_code_ -- --nocapture
- cargo test --locked --test cli_e2e test_binary_help_displays_exit_codes -- --nocapture
- cargo test --locked --bin downloader -- --nocapture
- cargo test --locked --test cli_e2e -- --nocapture

### Completion Notes List

- 2026-02-17: Story created and set to ready-for-dev.
- 2026-02-17: Added explicit process exit mapping: success=`0`, partial=`1`, failure/fatal=`2`.
- 2026-02-17: Updated CLI help text to document exit-code semantics.
- 2026-02-17: Added deterministic unit and E2E coverage for exit code behavior and help docs.
- 2026-02-17: Senior code review completed with High/Medium issues resolved; story accepted as done.

### File List

- _bmad-output/implementation-artifacts/7-7-exit-codes.md
- _bmad-output/implementation-artifacts/sprint-status.yaml
- src/main.rs
- src/cli.rs
- tests/cli_e2e.rs

### Change Log

- 2026-02-17: Story created and marked ready-for-dev.
- 2026-02-17: Party mode audit completed with follow-up actions.
- 2026-02-17: Implemented exit-code behavior and moved story to review.
- 2026-02-17: Code review completed; story marked done.

## Party Mode Audit (AI)

Audit date: 2026-02-17  
Outcome: pass_with_actions  
Summary: High=0, Medium=2, Low=1

Findings:
- Medium: Exit-code mapping can drift if outcome classification remains implicit in control flow.
- Medium: Partial-success behavior should be validated with deterministic logic tests, not network-only E2E.
- Low: Exit-code semantics should be discoverable via `--help`.

## Senior Developer Review (AI)

Reviewer: fierce  
Date: 2026-02-17  
Outcome: Approve

### Findings Summary

- High: 0
- Medium: 2 (fixed)
- Low: 1

### Fixed During Review (Auto-Fix High/Medium)

- **Medium (fixed):** Process-level exit behavior relied on generic `Result<()>` semantics, which could not express the required `2` code for fatal/complete failures.  
  Fix: introduced explicit process-exit outcome enum and top-level `ExitCode` mapping.
- **Medium (fixed):** No deterministic coverage existed for the `0/1/2` mapping independent of network conditions.  
  Fix: added unit-level outcome tests and deterministic E2E checks for success/failure code paths.

### Low-Severity Notes

- Partial-success (`1`) remains validated via deterministic unit mapping in this environment; full network-backed partial E2E remains optional where sandbox policy permits local listener sockets.
