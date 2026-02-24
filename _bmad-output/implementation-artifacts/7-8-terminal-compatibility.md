# Story 7.8: Terminal Compatibility

Status: done

<!-- Note: Validation is optional. Run validate-create-story for quality check before dev-story. -->

## Story

As a **user**,
I want **the tool to work in any terminal**,
so that **output is readable everywhere**.

## Acceptance Criteria

1. **AC1: Terminal width-aware output**
   - **Given** varying terminal widths
   - **When** output is rendered
   - **Then** text is width-aware/truncated appropriately

2. **AC2: `NO_COLOR` support**
   - **Given** `NO_COLOR` env var is set
   - **When** tool output is rendered
   - **Then** ANSI color/styling is disabled

3. **AC3: `--no-color` CLI override**
   - **Given** `--no-color` flag
   - **When** tool runs
   - **Then** ANSI color/styling is disabled

4. **AC4: Dumb-terminal plain text**
   - **Given** `TERM=dumb`
   - **When** tool output is rendered
   - **Then** output remains plain text without ANSI escape sequences

5. **AC5: Spinner fallback behavior**
   - **Given** non-interactive or dumb terminal environments
   - **When** downloads run
   - **Then** spinner UI is disabled/falls back to plain text behavior

## Tasks / Subtasks

- [x] Task 1: Add no-color CLI surface and runtime color controls (AC: 2, 3, 4)
  - [x] 1.1 Add `--no-color` flag in CLI args
  - [x] 1.2 Implement deterministic color-disable policy from flag/env/terminal type
  - [x] 1.3 Apply color policy to tracing output initialization

- [x] Task 2: Harden terminal-environment helpers (AC: 1, 4, 5)
  - [x] 2.1 Add terminal-mode helpers for `TERM=dumb` and `NO_COLOR`
  - [x] 2.2 Add spinner gating helper for interactive vs quiet vs dumb behavior
  - [x] 2.3 Preserve existing width-aware truncation behavior for user-facing lines

- [x] Task 3: Add deterministic tests for compatibility behavior (AC: 1-5)
  - [x] 3.1 CLI parser test for `--no-color`
  - [x] 3.2 Unit tests for color/spinner gating helpers
  - [x] 3.3 E2E tests for `NO_COLOR` and `TERM=dumb` no-ANSI behavior

### Review Follow-ups (AI)

- [x] [AI-Audit][Medium] Ensure color-disable behavior is centralized (single decision point) to avoid drift between env flag and CLI flag handling.
- [x] [AI-Audit][Medium] Ensure spinner behavior is explicitly disabled for dumb terminals in addition to non-interactive stdout/stderr.
- [x] [AI-Audit][Low] Maintain deterministic tests that assert no ANSI escape sequences under `NO_COLOR` and `TERM=dumb`.

## Dev Notes

### Architecture Context

- Runtime output initialization and spinner behavior are in `src/main.rs`.
- CLI argument surface is in `src/cli.rs`.
- Existing width truncation helpers (`terminal_width`, `truncate_to_width`) already support AC1.

### Implementation Guidance

- Use a single helper path for color-disable decisions.
- Keep spinner policy deterministic and testable.
- Avoid introducing terminal behavior that depends on non-deterministic TTY assumptions in tests.

### Testing Notes

- Validate helper logic with pure unit tests.
- Validate no-ANSI behavior in E2E via string checks (`\x1b[` absent).
- Keep tests robust under sandbox constraints.

### Project Structure Notes

- Touch points: `src/main.rs`, `src/cli.rs`, `tests/cli_e2e.rs`.

### References

- [Source: _bmad-output/planning-artifacts/epics.md#Story-7.8-Terminal-Compatibility]
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
- cargo test --locked --bin downloader test_cli_no_color_flag_sets_no_color -- --nocapture
- cargo test --locked --bin downloader should_disable_color -- --nocapture
- cargo test --locked --bin downloader should_use_spinner -- --nocapture
- cargo test --locked --test cli_e2e no_color_env_disables_ansi_sequences -- --nocapture
- cargo test --locked --test cli_e2e dumb_terminal_disables_ansi_sequences -- --nocapture
- cargo test --locked --test cli_e2e test_binary_no_color_flag_accepted -- --nocapture
- cargo test --locked --bin downloader -- --nocapture
- cargo test --locked --test cli_e2e -- --nocapture

### Completion Notes List

- 2026-02-17: Story created and set to ready-for-dev.
- 2026-02-17: Added `--no-color` CLI flag and centralized color-disable decision helpers.
- 2026-02-17: Integrated `NO_COLOR`/`TERM=dumb` handling into tracing output configuration.
- 2026-02-17: Added explicit spinner gating helper for quiet/non-interactive/dumb terminal fallback behavior.
- 2026-02-17: Added unit and E2E coverage for no-color and dumb-terminal compatibility rules.
- 2026-02-17: Senior code review completed with High/Medium issues resolved; story accepted as done.

### File List

- _bmad-output/implementation-artifacts/7-8-terminal-compatibility.md
- _bmad-output/implementation-artifacts/sprint-status.yaml
- src/main.rs
- src/cli.rs
- tests/cli_e2e.rs

### Change Log

- 2026-02-17: Story created and marked ready-for-dev.
- 2026-02-17: Party mode audit completed with follow-up actions.
- 2026-02-17: Implemented terminal compatibility controls and moved story to review.
- 2026-02-17: Code review completed; story marked done.

## Party Mode Audit (AI)

Audit date: 2026-02-17  
Outcome: pass_with_actions  
Summary: High=0, Medium=2, Low=1

Findings:
- Medium: Color-disable behavior can become inconsistent if `NO_COLOR`, `--no-color`, and terminal heuristics are evaluated in separate paths.
- Medium: Dumb-terminal compatibility requires explicit spinner suppression beyond generic non-interactive detection.
- Low: ANSI absence should be validated by deterministic test assertions in E2E output.

## Senior Developer Review (AI)

Reviewer: fierce  
Date: 2026-02-17  
Outcome: Approve

### Findings Summary

- High: 0
- Medium: 2 (fixed)
- Low: 1

### Fixed During Review (Auto-Fix High/Medium)

- **Medium (fixed):** Color-disable logic was not centralized before this story and risked drift across env/flag/terminal paths.  
  Fix: introduced `should_disable_color(...)` and `is_no_color_requested(...)` with single-path usage at tracing init.
- **Medium (fixed):** Spinner fallback did not explicitly account for `TERM=dumb`.  
  Fix: introduced `should_use_spinner(...)` helper and disabled spinner when terminal is dumb.

### Low-Severity Notes

- Current no-ANSI E2E checks validate absence of escape codes; if future UI introduces richer formatting, consider adding explicit snapshot coverage for plain-text fallback lines.
