# Story 7.4: No-Input Help Display

Status: done

<!-- Note: Validation is optional. Run validate-create-story for quality check before dev-story. -->

## Story

As a **user**,
I want **helpful guidance when I run the tool with no input**,
so that **I can learn how to use it**.

## Acceptance Criteria

1. **AC1: Friendly no-input quick-start guidance**
   - **Given** `downloader` is run with no args and no stdin
   - **When** startup input checks complete
   - **Then** the command exits successfully with a friendly quick-start message (not an error)

2. **AC2: Show common examples**
   - **Given** the no-input path is triggered
   - **When** guidance is printed
   - **Then** the output includes practical examples for stdin piping and direct invocation

3. **AC3: Keep guidance readable in 80-char terminals**
   - **Given** a narrow terminal (80 columns)
   - **When** no-input guidance is rendered
   - **Then** output lines are readable and do not require horizontal scrolling

4. **AC4: Preserve full help behavior**
   - **Given** `downloader --help`
   - **When** user requests full help
   - **Then** clap help output remains intact, while no-input path remains a quick-start message

## Tasks / Subtasks

- [x] Task 1: Align no-input runtime guidance with quick-start UX intent (AC: 1, 2)
  - [x] 1.1 Audit current `no input` and `empty stdin` messaging paths in `src/main.rs`
  - [x] 1.2 Ensure no-input path is explicitly friendly and action-oriented
  - [x] 1.3 Include common usage examples for both piped stdin and direct URL arguments

- [x] Task 2: Enforce 80-char readability for no-input guidance (AC: 3)
  - [x] 2.1 Route no-input message rendering through terminal-width-aware formatting
  - [x] 2.2 Keep deterministic behavior when terminal width is unavailable

- [x] Task 3: Preserve CLI help/quick-start separation (AC: 4)
  - [x] 3.1 Confirm quick-start guidance is only used on no-input runtime path
  - [x] 3.2 Confirm `--help` still delegates to clap-generated full help text

- [x] Task 4: Add deterministic coverage for no-input UX behavior (AC: 1-4)
  - [x] 4.1 E2E test for no-args/no-stdin quick-start messaging and success exit
  - [x] 4.2 E2E test for example lines in no-input output
  - [x] 4.3 Unit/integration check for 80-char-friendly formatting in no-input path
  - [x] 4.4 E2E regression that `--help` remains full help output

### Review Follow-ups (AI)

- [x] [AI-Audit][Medium] Ensure no-input quick-start messaging does not suppress existing resume behavior when queue state already exists.
- [x] [AI-Audit][Medium] Enforce deterministic 80-column readability for no-input guidance lines, including example output and suggestions.
- [x] [AI-Audit][Low] Add regression coverage confirming no-input quick-start remains distinct from clap `--help` output.

## Dev Notes

### Architecture Context

- Runtime orchestration and no-input handling live in `src/main.rs`.
- Clap help/version behavior should continue to short-circuit before runtime logic.
- Existing terminal-width utilities (`terminal_width`, `truncate_to_width`) can be reused.

### Implementation Guidance

- Keep no-input and empty-stdin outcomes non-fatal (`exit 0`) and instructional.
- Prefer stable message strings for deterministic tests and scriptability.
- Avoid introducing new dependencies; use existing std/clap/tracing patterns.

### Testing Notes

- Use binary E2E tests in `tests/cli_e2e.rs` for user-facing output assertions.
- Keep tests network-independent using invalid/non-URL tokens where needed.
- Validate readability assumptions under `COLUMNS=80` without relying on interactive TTY.

### Project Structure Notes

- Expected touch points: `src/main.rs`, `tests/cli_e2e.rs`.
- Optional touch point: `README.md` examples (only if messaging changes materially).

### References

- [Source: _bmad-output/planning-artifacts/epics.md#Story-7.4-No-Input-Help-Display]
- [Source: _bmad-output/planning-artifacts/prd.md#FR-5-CLI-Interface]
- [Source: _bmad-output/planning-artifacts/ux-design-specification.md#Command-Line-Interface]
- [Source: _bmad-output/planning-artifacts/architecture.md#Requirements-Overview]
- [Source: _bmad-output/project-context.md#Framework-Specific-Rules]

## Dev Agent Record

### Agent Model Used

GPT-5 Codex (CLI)

### Debug Log References

- create-story via epic-auto-flow
- cargo fmt
- cargo test --locked --test cli_e2e test_binary_empty_stdin_ -- --nocapture
- cargo test --locked --test cli_e2e quick_start_lines_fit_80_columns -- --nocapture
- cargo test --locked --test cli_e2e test_binary_help_displays_usage -- --nocapture
- cargo test --locked --bin downloader test_truncate_to_width -- --nocapture
- code-review follow-up: extracted deterministic quick-start guidance line builder for direct unit validation
- cargo test --locked --bin downloader quick_start_guidance_lines -- --nocapture
- cargo test --locked --test cli_e2e test_binary_empty_stdin_shows_quick_start_examples -- --nocapture
- cargo test --locked --test cli_e2e quick_start_lines_fit_80_columns -- --nocapture

### Completion Notes List

- 2026-02-17: Story created and set to ready-for-dev.
- 2026-02-17: Added quick-start guidance rendering helper with terminal-width-capped output for no-input and empty-stdin paths.
- 2026-02-17: Added explicit direct-argument and stdin-pipe examples to no-input guidance.
- 2026-02-17: Added deterministic E2E coverage for quick-start example output and 80-column readability.
- 2026-02-17: Senior code review completed with High/Medium issues resolved; story accepted as done.
- 2026-02-17: Added unit-level coverage for non-empty-stdin quick-start content via deterministic line builder.
- 2026-02-17: Adversarial follow-up review added regressions for `--help` precedence with piped stdin, tiny `COLUMNS` fallback behavior, and no-input branch width-cap coverage.

### File List

- _bmad-output/implementation-artifacts/7-4-no-input-help-display.md
- _bmad-output/implementation-artifacts/sprint-status.yaml
- src/main.rs
- tests/cli_e2e.rs

### Change Log

- 2026-02-17: Story created and marked ready-for-dev.
- 2026-02-17: Party mode audit completed with follow-up actions.
- 2026-02-17: Implemented no-input quick-start output polish and moved story to review.
- 2026-02-17: Code review completed; story marked done.
- 2026-02-17: Follow-up code review fix applied (unit-testable quick-start line builder).
- 2026-02-17: Adversarial code-review follow-up added targeted regression coverage for help precedence and quick-start width edge cases.

## Party Mode Audit (AI)

Audit date: 2026-02-17  
Outcome: pass_with_actions  
Summary: High=0, Medium=2, Low=1

Findings:
- Medium: No-input guidance can unintentionally override resume-aware behavior if queue-state checks are bypassed.
- Medium: 80-column readability is easy to regress when adding extra examples unless line-formatting is explicitly constrained.
- Low: Quick-start/no-input messaging can drift toward help-text duplication without dedicated regression checks.

## Senior Developer Review (AI)

Reviewer: fierce  
Date: 2026-02-17  
Outcome: Approve

### Findings Summary

- High: 0
- Medium: 1 (fixed)
- Low: 2

### Fixed During Review (Auto-Fix High/Medium)

- **Medium (fixed):** Coverage did not directly validate the non-empty-stdin quick-start content branch (the true no-input headline path).  
  Fix: extracted deterministic `quick_start_guidance_lines(...)` and added unit tests that assert headline/examples and width constraints.

### Low-Severity Notes

- Interactive TTY-only no-stdin behavior is still validated indirectly in E2E due test harness stdin limitations.
- Guidance currently truncates to width; if future UX prefers preserving full commands, switch to word-wrapping instead of truncation.

## Senior Developer Review (AI) - Adversarial Follow-up

Reviewer: fierce  
Date: 2026-02-17  
Outcome: Approve

### Findings Summary

- High: 0
- Medium: 3 (fixed)
- Low: 2 (open)

### Fixed During Review (Auto-Fix High/Medium)

- **Medium (fixed):** AC4 coverage missed an explicit regression proving clap `--help` output still wins when stdin is piped.  
  Fix: added `test_binary_help_with_stdin_bypasses_quick_start_guidance` in `tests/cli_e2e.rs`.
- **Medium (fixed):** No regression verified terminal-width fallback when `COLUMNS` is too small/invalid for readable quick-start output.  
  Fix: added `test_binary_quick_start_small_columns_env_falls_back_to_default_width` in `tests/cli_e2e.rs`.
- **Medium (fixed):** Width-cap unit coverage only validated the empty-stdin headline branch and did not directly assert the no-input branch.  
  Fix: added `test_quick_start_guidance_lines_no_input_branch_respect_width_cap` in `src/main.rs`.

### Remaining Decision Items

- **Low (decision):** Keep truncation-based formatting for narrow terminals or switch to word-wrapping to preserve full example commands.
- **Low (decision):** Keep current unit-level no-input branch validation, or introduce PTY-based E2E coverage for true interactive no-stdin behavior.
