# Story 7.6: Verbosity Levels

Status: done

<!-- Note: Validation is optional. Run validate-create-story for quality check before dev-story. -->

## Story

As a **user**,
I want **control over output verbosity**,
so that **I can get more or less detail as needed**.

## Acceptance Criteria

1. **AC1: Default verbosity baseline**
   - **Given** no verbosity flags
   - **When** the tool runs
   - **Then** output shows high-level status and completion summary only

2. **AC2: Verbose mode detail**
   - **Given** `--verbose` or `-v`
   - **When** the tool runs
   - **Then** additional per-item/progress detail is shown

3. **AC3: Quiet mode script-friendly output**
   - **Given** `--quiet` or `-q`
   - **When** the tool runs
   - **Then** output is reduced to minimal summary-oriented information

4. **AC4: Debug mode full tracing**
   - **Given** `--debug`
   - **When** the tool runs
   - **Then** full tracing/debug output is enabled

5. **AC5: Mutually exclusive verbosity levels**
   - **Given** verbosity options are provided
   - **When** conflicting levels are requested
   - **Then** CLI validation enforces mutual exclusivity

## Tasks / Subtasks

- [x] Task 1: Add explicit debug verbosity surface and mapping (AC: 4)
  - [x] 1.1 Add `--debug` CLI flag in download-mode arguments
  - [x] 1.2 Map `--debug` to highest tracing verbosity deterministically

- [x] Task 2: Enforce mutually exclusive verbosity controls (AC: 5)
  - [x] 2.1 Define clap conflicts for `--quiet`, `--verbose`, and `--debug`
  - [x] 2.2 Validate conflict behavior in parser tests

- [x] Task 3: Standardize runtime verbosity behavior (AC: 1, 2, 3, 4)
  - [x] 3.1 Keep default output concise and useful
  - [x] 3.2 Ensure `-v`/`--verbose` increases diagnostic detail without debug noise
  - [x] 3.3 Ensure `-q` suppresses non-essential output but preserves critical/final signals
  - [x] 3.4 Ensure `--debug` enables maximum tracing detail

- [x] Task 4: Add deterministic tests for verbosity behavior (AC: 1-5)
  - [x] 4.1 CLI parser tests for `--debug` and flag conflicts
  - [x] 4.2 E2E tests validating output-level differences across default/verbose/quiet/debug
  - [x] 4.3 Regression checks for config-driven verbosity interaction with CLI overrides

### Review Follow-ups (AI)

- [x] [AI-Audit][Medium] Ensure `--debug` is available on the main download command path without breaking existing `-v` count semantics.
- [x] [AI-Audit][Medium] Enforce mutual exclusivity for all verbosity controls at clap-parse time to prevent ambiguous runtime behavior.
- [x] [AI-Audit][Low] Add explicit regression checks for config-driven verbosity defaults when CLI flags are absent.

## Dev Notes

### Architecture Context

- CLI argument definitions are in `src/cli.rs`; runtime log-level initialization is in `src/main.rs`.
- Existing log-level mapping currently derives from `verbose` count and `quiet` flag.
- Config-file defaults can influence verbosity and must preserve CLI precedence.

### Implementation Guidance

- Prefer explicit enum/state mapping for verbosity decisions to avoid drift.
- Keep output deterministic for tests and scripting use cases.
- Avoid regressions in command subcommands (`auth`, `log`, `config`) when verbosity flags are set.

### Testing Notes

- Assert parser-level conflicts for mutually exclusive flags.
- Validate effective tracing level behavior using deterministic output assertions where possible.
- Cover interactions with config defaults and command-line overrides.

### Project Structure Notes

- Expected touch points: `src/cli.rs`, `src/main.rs`, `tests/cli_e2e.rs`.
- Supporting updates may include tests in `src/main.rs` and `src/cli.rs`.

### References

- [Source: _bmad-output/planning-artifacts/epics.md#Story-7.6-Verbosity-Levels]
- [Source: _bmad-output/planning-artifacts/prd.md#FR-5-CLI-Interface]
- [Source: _bmad-output/planning-artifacts/ux-design-specification.md#Verbosity-and-Output-Density]
- [Source: _bmad-output/planning-artifacts/architecture.md#Requirements-Overview]
- [Source: _bmad-output/project-context.md#Framework-Specific-Rules]

## Dev Agent Record

### Agent Model Used

GPT-5 Codex (CLI)

### Debug Log References

- create-story via epic-auto-flow
- cargo fmt
- cargo test --locked --bin downloader test_cli_debug -- --nocapture
- cargo test --locked --bin downloader resolve_default_log_level -- --nocapture
- cargo test --locked --bin downloader apply_config_verbosity_ -- --nocapture
- cargo test --locked --test cli_e2e debug_parsed_args_line -- --nocapture
- cargo test --locked --test cli_e2e test_binary_config_show_ -- --nocapture
- cargo test --locked --bin downloader test_cli_ -- --nocapture
- cargo test --locked --bin downloader -- --nocapture
- cargo test --locked --test cli_e2e -- --nocapture
- code-review follow-up: enforce CLI verbosity-flag precedence over inherited RUST_LOG when verbosity flags are explicitly set
- cargo test --locked --bin downloader should_force_cli_log_level -- --nocapture
- cargo test --locked --bin downloader resolve_default_log_level -- --nocapture
- cargo test --locked --test cli_e2e debug_parsed_args_line -- --nocapture

### Completion Notes List

- 2026-02-17: Story created and set to ready-for-dev.
- 2026-02-17: Added `--debug` CLI flag and enforced mutual exclusivity between `--quiet`, `--verbose`, and `--debug`.
- 2026-02-17: Refactored runtime log-level selection into deterministic `resolve_default_log_level`.
- 2026-02-17: Extended config/default-merge logic to respect explicit CLI `--debug` and config-driven debug verbosity.
- 2026-02-17: Added parser, unit, and E2E coverage for debug behavior and verbosity precedence.
- 2026-02-17: Senior code review completed with High/Medium issues fixed; story accepted as done.

### File List

- _bmad-output/implementation-artifacts/7-6-verbosity-levels.md
- _bmad-output/implementation-artifacts/sprint-status.yaml
- src/cli.rs
- src/main.rs
- tests/cli_e2e.rs

### Change Log

- 2026-02-17: Story created and marked ready-for-dev.
- 2026-02-17: Party mode audit completed with follow-up actions.
- 2026-02-17: Implemented verbosity-level controls and moved story to review.
- 2026-02-17: Code review completed; story marked done.
- 2026-02-17: Follow-up code review fix applied (CLI verbosity flags now override inherited `RUST_LOG`).

## Party Mode Audit (AI)

Audit date: 2026-02-17  
Outcome: pass_with_actions  
Summary: High=0, Medium=2, Low=1

Findings:
- Medium: `--debug` introduction can conflict with current `verbose` count behavior unless mapping precedence is explicit.
- Medium: Missing parser-level conflicts among verbosity flags can create inconsistent runtime log-level decisions.
- Low: Config-default verbosity should be revalidated to ensure CLI-overrides precedence remains deterministic.

## Senior Developer Review (AI)

Reviewer: fierce  
Date: 2026-02-17  
Outcome: Approve

### Findings Summary

- High: 0
- Medium: 2 (fixed)
- Low: 1

### Fixed During Review (Auto-Fix High/Medium)

- **Medium (fixed):** Explicit `--debug` could be masked by inherited `RUST_LOG` (for example `RUST_LOG=warn`), violating expected verbosity-flag behavior.  
  Fix: added `should_force_cli_log_level(...)` and updated tracing init so explicit verbosity flags force CLI-selected level.
- **Medium (fixed):** No direct coverage existed for CLI-precedence override behavior under inherited `RUST_LOG`.  
  Fix: added unit and E2E tests validating debug behavior remains active when `RUST_LOG=warn`.

### Low-Severity Notes

- Verbosity flags currently apply to download mode; if future UX requires parity for subcommands (`auth`, `log`, `config`), consider promoting verbosity flags to a shared/global argument surface.
