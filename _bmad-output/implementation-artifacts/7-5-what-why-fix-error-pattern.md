# Story 7.5: What/Why/Fix Error Pattern

Status: done

<!-- Note: Validation is optional. Run validate-create-story for quality check before dev-story. -->

## Story

As a **user**,
I want **error messages that tell me what to do**,
so that **I can resolve problems without searching**.

## Acceptance Criteria

1. **AC1: What/Why/Fix message format**
   - **Given** an error condition is shown to the user
   - **When** error output is rendered
   - **Then** it follows: What happened -> Why it might have happened -> How to fix

2. **AC2: Categorized error indicators**
   - **Given** known error classes
   - **When** an error is displayed
   - **Then** messages include category indicators using the specified icon set (🔐, ❌, 🌐, ⚠️)

3. **AC3: Actionable remediation guidance**
   - **Given** a failed operation
   - **When** guidance is printed
   - **Then** fix suggestions are concrete and specific to the failure class

4. **AC4: Summary grouping by category**
   - **Given** a run with one or more failures
   - **When** completion/failure summary is printed
   - **Then** the summary includes grouped counts by error category

## Tasks / Subtasks

- [x] Task 1: Standardize CLI-facing error presentation contract (AC: 1, 3)
  - [x] 1.1 Introduce/extend a single helper path that renders What/Why/Fix triplets
  - [x] 1.2 Ensure existing download/auth/parse failures route through the unified renderer
  - [x] 1.3 Keep language concise and directly actionable

- [x] Task 2: Add category indicators/icons and mapping rules (AC: 2)
  - [x] 2.1 Define deterministic mapping of failure classes to indicators (🔐, ❌, 🌐, ⚠️)
  - [x] 2.2 Apply mapping consistently in error lines and summary rows

- [x] Task 3: Add grouped error-category summary output (AC: 4)
  - [x] 3.1 Aggregate failure counts by category during completion reporting
  - [x] 3.2 Render grouped category totals in final output

- [x] Task 4: Add deterministic tests for error UX behavior (AC: 1-4)
  - [x] 4.1 Unit tests for category mapping and What/Why/Fix rendering
  - [x] 4.2 Integration/E2E tests for representative failures (auth, not found, network, parse)
  - [x] 4.3 Regression test for grouped summary category counts

### Review Follow-ups (AI)

- [x] [AI-Audit][Medium] Ensure category mapping is derived from stable error-type metadata (not brittle string-matching of free-form messages).
- [x] [AI-Audit][Medium] Validate grouped summary counts when mixed failure categories occur in the same run.
- [x] [AI-Audit][Low] Keep What/Why/Fix output compact enough for standard terminal widths without wrapping noise.

## Dev Notes

### Architecture Context

- CLI output assembly lives primarily in `src/main.rs`.
- Existing failure classification helpers (download/history summary) can be reused/extended.
- Error-domain typing exists in library modules and should stay source-of-truth where possible.

### Implementation Guidance

- Prefer additive output changes that do not break deterministic parsing/tests for existing stories.
- Reuse existing error-type metadata and suggestion extraction before introducing new heuristics.
- Keep message format stable and compact for both terminal users and logs.

### Testing Notes

- Favor deterministic no-network failure injection patterns in tests.
- Assert exact markers/labels and category totals where feasible.
- Validate both per-item errors and summary-level grouped counts.

### Project Structure Notes

- Expected touch points: `src/main.rs`, `tests/cli_e2e.rs`.
- Possible supporting updates: `src/download/error.rs`, `src/parser/error.rs`.

### References

- [Source: _bmad-output/planning-artifacts/epics.md#Story-7.5-What/Why/Fix-Error-Pattern]
- [Source: _bmad-output/planning-artifacts/prd.md#Non-Functional-Requirements]
- [Source: _bmad-output/planning-artifacts/ux-design-specification.md#Error-Messaging]
- [Source: _bmad-output/planning-artifacts/architecture.md#Requirements-Overview]
- [Source: _bmad-output/project-context.md#Framework-Specific-Rules]

## Dev Agent Record

### Agent Model Used

GPT-5 Codex (CLI)

### Debug Log References

- create-story via epic-auto-flow
- cargo fmt
- cargo test --locked --bin downloader classify_failure -- --nocapture
- cargo test --locked --bin downloader render_failure_summary_lines -- --nocapture
- cargo test --locked --bin downloader render_history_cli_row_ -- --nocapture
- cargo test --locked --bin downloader -- --nocapture
- cargo test --locked --test cli_e2e test_binary_log_ -- --nocapture
- code-review follow-up: stabilize grouped category descriptors for mixed auth failures
- cargo test --locked --bin downloader render_failure_summary_lines -- --nocapture
- cargo test --locked --bin downloader render_history_cli_row_ -- --nocapture

### Completion Notes List

- 2026-02-17: Story created and set to ready-for-dev.
- 2026-02-17: Implemented deterministic failure categorization with icon mapping (🔐, ❌, 🌐, ⚠️).
- 2026-02-17: Added What/Why/Fix rendering for failed history rows and grouped failure summaries.
- 2026-02-17: Added unit coverage for classification, category icons, and grouped summary rendering behavior.
- 2026-02-17: Senior code review completed with High/Medium issues auto-fixed; story accepted as done.

### File List

- _bmad-output/implementation-artifacts/7-5-what-why-fix-error-pattern.md
- _bmad-output/implementation-artifacts/sprint-status.yaml
- src/main.rs

### Change Log

- 2026-02-17: Story created and marked ready-for-dev.
- 2026-02-17: Party mode audit completed with follow-up actions.
- 2026-02-17: Implemented What/Why/Fix error pattern and moved story to review.
- 2026-02-17: Code review completed; story marked done.
- 2026-02-17: Follow-up code review fixes applied (stable category descriptors for grouped summaries).

## Party Mode Audit (AI)

Audit date: 2026-02-17  
Outcome: pass_with_actions  
Summary: High=0, Medium=2, Low=1

Findings:
- Medium: If category assignment relies on free-form text matching, error grouping can drift as wording evolves.
- Medium: Grouped summary output is prone to miscount regressions when a run contains multiple failure classes.
- Low: What/Why/Fix formatting can become overly verbose without explicit line-length discipline.

## Senior Developer Review (AI)

Reviewer: fierce  
Date: 2026-02-17  
Outcome: Approve

### Findings Summary

- High: 0
- Medium: 2 (fixed)
- Low: 1

### Fixed During Review (Auto-Fix High/Medium)

- **Medium (fixed):** Grouped category summaries reused first-seen descriptor text, causing inconsistent Auth guidance for mixed 401/407 failures.  
  Fix: switched grouped summary rendering to stable category-level descriptors.
- **Medium (fixed):** History-row What/Why/Fix rendering could miss auth categorization for legacy rows without `[AUTH]` prefix despite typed auth metadata.  
  Fix: added typed fallback descriptor mapping from `error_type` (with 407-specific proxy guidance).

### Low-Severity Notes

- Completion-summary categorization still derives from stored error text because queue rows currently persist only `last_error`; future schema support for typed failure categories would further harden mapping.
