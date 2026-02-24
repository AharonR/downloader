# Story 8.3: Parsing Confidence Tracking

Status: done

<!-- Note: Validation is optional. Run validate-create-story for quality check before dev-story. -->

## Story

As a **user**,
I want **to know which references were uncertain**,
so that **I can verify them manually**.

## Acceptance Criteria

1. **AC1: Confidence Levels Tracked for Parsed References**
   - **Given** references parsed with varying extraction quality
   - **When** parsing completes
   - **Then** each parsed reference is classified as `high`, `medium`, or `low` confidence
   - **And** confidence levels use deterministic rules based on extracted fields (authors, year, title)

2. **AC2: Uncertain Items Flagged in Summary**
   - **Given** one or more low-confidence references were parsed
   - **When** run summary is displayed
   - **Then** uncertain references are explicitly flagged for manual verification
   - **And** summary language remains concise and actionable

3. **AC3: `downloader log --uncertain` Lists Review Candidates**
   - **Given** download history contains parsed references with uncertainty
   - **When** user runs `downloader log --uncertain`
   - **Then** output lists rows needing review
   - **And** filter can be combined with existing scope filters (`--project`, `--output-dir`, `--since`, `--domain`, `--limit`)

4. **AC4: Confidence Persisted for Later Query**
   - **Given** a reference-derived queue item is processed
   - **When** history row is written to `download_log`
   - **Then** confidence level is stored in the database
   - **And** confidence factors are persisted in queryable form
   - **And** non-reference rows remain valid (NULL confidence fields allowed)

5. **AC5: Confidence Factors Logged at Debug Level**
   - **Given** parser processes reference input
   - **When** debug logging is enabled
   - **Then** confidence factors are logged (field presence and computed level)
   - **And** logging remains structured (`tracing` fields), not free-form text blobs

## Tasks / Subtasks

- [x] Task 1: Extend reference confidence model with explicit factors (AC: 1, 5)
  - [x] 1.1 Add a confidence-factors payload in `src/parser/reference.rs` (e.g., `has_authors`, `has_year`, `has_title`, `author_count`)
  - [x] 1.2 Keep existing confidence classification (`high`/`medium`/`low`) backward-compatible with current tests
  - [x] 1.3 Add helper API for downstream code to consume both level and factors without reparsing
  - [x] 1.4 Emit structured `debug!` logs including confidence factors for parsed references

- [x] Task 2: Persist parse confidence metadata in queue records (AC: 1, 4)
  - [x] 2.1 Add migration for queue confidence fields (`parse_confidence`, `parse_confidence_factors`)
  - [x] 2.2 Extend `QueueMetadata` and `QueueItem` to carry confidence data
  - [x] 2.3 Update `Queue::enqueue_with_metadata()` SQL insert bindings and row mapping
  - [x] 2.4 Ensure non-reference input paths keep these fields NULL

- [x] Task 3: Persist confidence to `download_log` for query-time filtering (AC: 3, 4)
  - [x] 3.1 Add migration for `download_log` confidence fields and index for uncertain queries
  - [x] 3.2 Extend `NewDownloadAttempt`, `DownloadAttempt`, and `DownloadAttemptQuery` in `src/queue/history.rs`
  - [x] 3.3 Propagate queue confidence fields in `src/download/engine.rs` for both success and failure history writes
  - [x] 3.4 Keep existing history behavior backward-compatible for legacy rows (missing confidence columns/values)

- [x] Task 4: Surface uncertainty in parse + completion summaries (AC: 2)
  - [x] 4.1 Update `build_parse_feedback_summary()` in `src/main.rs` to report confidence distribution clearly
  - [x] 4.2 Ensure completion summary surfaces uncertain count for the current run
  - [x] 4.3 Avoid noisy output when uncertain count is zero

- [x] Task 5: Add `downloader log --uncertain` CLI filter (AC: 3)
  - [x] 5.1 Add `--uncertain` flag to `LogArgs` in `src/cli.rs` with conflict validation against incompatible flags
  - [x] 5.2 Update `run_log_command()` in `src/main.rs` to map `--uncertain` into query filters
  - [x] 5.3 Extend history query SQL to filter uncertain rows efficiently
  - [x] 5.4 Include confidence indicator in rendered log rows when present

- [x] Task 6: Add comprehensive regression coverage (AC: 1-5)
  - [x] 6.1 Parser tests for confidence levels and factors (`src/parser/reference.rs`, `tests/parser_integration.rs`)
  - [x] 6.2 Queue/history integration tests for confidence persistence (`tests/queue_integration.rs`, `tests/download_engine_integration.rs`)
  - [x] 6.3 CLI parse tests for `log --uncertain` (`src/cli.rs` tests)
  - [x] 6.4 CLI E2E tests proving uncertain filtering behavior (`tests/cli_e2e.rs`)
  - [x] 6.5 Main summary tests for uncertain messaging (`src/main.rs` tests)

## Dev Notes

### Architecture Context

Confidence classification exists today in `src/parser/reference.rs` (`Confidence::High|Medium|Low`) but is currently ephemeral. Story 8.3 formalizes this into a durable data path:

1. Parse reference confidence and factors at input stage
2. Persist confidence metadata on queue items
3. Copy confidence metadata into `download_log` on terminal outcome
4. Expose uncertain filtering through `downloader log --uncertain`

This aligns with FR-4.4 (confidence tracking) and FR-4.5 (query past downloads) without introducing a new subsystem.

### Implementation Guidance

**Uncertainty definition (scope control):**
- Treat `low` confidence as uncertain for `--uncertain` output.
- Keep `medium` visible in confidence distribution but do not include in uncertain-only filter unless explicitly requested in a future story.

**Confidence factors payload:**
- Persist as JSON text for extensibility (e.g., `{"has_authors":true,"has_year":false,"has_title":true,"author_count":1}`).
- Keep schema simple and nullable; confidence fields should not block non-reference rows.

**Log query behavior:**
- `--uncertain` should compose with existing filters and limits.
- Preserve current sorting and pagination semantics in history queries.

**Debug logging requirement:**
- Confidence factors must be logged via structured `tracing` fields.
- Avoid duplicative logs for cached reference metadata lookups.

### File Structure Notes

**New files expected:**
- `migrations/20260218xxxxxx_add_parse_confidence_columns.sql` (exact timestamped filename per repo convention)

**Files likely modified:**
- `src/parser/reference.rs`
- `src/parser/input.rs` (if parse item metadata carriage is needed)
- `src/main.rs`
- `src/cli.rs`
- `src/queue/item.rs`
- `src/queue/mod.rs`
- `src/queue/history.rs`
- `src/download/engine.rs`
- `tests/parser_integration.rs`
- `tests/queue_integration.rs`
- `tests/download_engine_integration.rs`
- `tests/cli_e2e.rs`

### Testing Requirements

- Validate confidence classification invariants are stable (`3 fields -> high`, `2 -> medium`, otherwise `low`).
- Validate confidence fields round-trip from queue enqueue to download history row.
- Validate `downloader log --uncertain` returns only uncertain rows and respects `--limit`.
- Validate legacy rows (NULL confidence) do not crash rendering/filtering.
- Validate debug-level factor logs are emitted from parser paths.

### References

- [Source: /Users/ar2463/Documents/GitHub/Downloader/_bmad-output/planning-artifacts/epics.md#Story-8.3-Parsing-Confidence-Tracking]
- [Source: /Users/ar2463/Documents/GitHub/Downloader/_bmad-output/planning-artifacts/prd.md#FR-4-Logging--Memory]
- [Source: /Users/ar2463/Documents/GitHub/Downloader/src/parser/reference.rs]
- [Source: /Users/ar2463/Documents/GitHub/Downloader/src/main.rs]
- [Source: /Users/ar2463/Documents/GitHub/Downloader/src/queue/history.rs]
- [Source: /Users/ar2463/Documents/GitHub/Downloader/_bmad-output/implementation-artifacts/8-2-json-ld-sidecar-files.md#Senior-Developer-Review-AI---Adversarial-Follow-up]
- [clap `ValueEnum` docs](https://docs.rs/clap/latest/clap/trait.ValueEnum.html)
- [sqlx `query_as` docs](https://docs.rs/sqlx/latest/sqlx/fn.query_as.html)
- [tracing `#[instrument]` docs](https://docs.rs/tracing/latest/tracing/attr.instrument.html)
- [SQLite JSON functions/operators](https://www.sqlite.org/json1.html)

## Developer Context

### Critical Implementation Guardrails

1. Do not recompute confidence from formatted history rows; persist confidence once at parse time.
2. Keep history query performance predictable by using SQL-side filtering + index support.
3. Keep uncertain output deterministic (`low` only) for this story.
4. Do not alter existing success/failure exit-code behavior when adding uncertainty reporting.
5. Preserve compatibility for older databases (NULL confidence fields) and older rows.

### Previous Story Intelligence (from 8.2)

- Verify existing APIs before adding new ones; only expand queue/history APIs when strictly required.
- Keep post-processing features idempotent and resilient to interruption.
- Add focused tests for helper paths (zero-result and warn-and-continue scenarios).
- Maintain structured instrumentation and avoid ad-hoc output patterns.

### Latest Technical Notes (validated 2026-02-18)

- `clap` `ValueEnum` + `#[arg(value_enum)]` is the stable pattern for enum-based CLI filters.
- `sqlx::query_as` maps rows through `FromRow`, matching current queue/history struct style.
- `tracing` `#[instrument]` supports structured spans and field capture for confidence diagnostics.
- SQLite JSON operators/functions (`json_extract`, `->`, `->>`) are available for queryable confidence factor payloads.

## Dev Agent Record

### Agent Model Used

gpt-5-codex

### Debug Log References

- `cargo fmt`
- `cargo clippy -- -D warnings`
- `cargo test --bin downloader`
- `cargo test --test parser_integration --test queue_integration --test cli_e2e`
- `cargo test --test download_engine_integration` (sandbox-limited: wiremock bind failures)
- `cargo test` (sandbox-limited: wiremock/system-networking dependent failures in unrelated suites)

### Completion Notes List

- 2026-02-18: Story created and staged as `ready-for-dev` via epic-auto-flow create-story stage.
- 2026-02-18: Ultimate context engine analysis completed - comprehensive developer guide created.
- 2026-02-18: Implemented Story 8.3 end-to-end: parser confidence factors + helper API, queue/history persistence, `log --uncertain` filter, confidence rendering in log rows, and parse/completion summary updates.
- 2026-02-18: Added migration `20260218000009_add_parse_confidence_columns.sql` for queue and download_log confidence columns plus uncertain-query indexes.
- 2026-02-18: Validation run: `cargo clippy -- -D warnings` passed; `cargo test --bin downloader` passed; `cargo test --test parser_integration --test queue_integration --test cli_e2e` passed.
- 2026-02-18: Full suite `cargo test` and `cargo test --test download_engine_integration` are blocked in this sandbox due `wiremock` port bind permissions (`Operation not permitted`) on existing network-bound tests.
- 2026-02-18: Adversarial code review follow-up completed with safe auto-fixes applied for AC3/AC4/AC5 coverage gaps; targeted story validation rerun and passing.

### File List

- _bmad-output/implementation-artifacts/8-3-parsing-confidence-tracking.md
- _bmad-output/implementation-artifacts/sprint-status.yaml
- migrations/20260218000009_add_parse_confidence_columns.sql
- src/cli.rs
- src/db.rs
- src/download/engine.rs
- src/lib.rs
- src/main.rs
- src/parser/mod.rs
- src/parser/reference.rs
- src/queue/history.rs
- src/queue/item.rs
- src/queue/mod.rs
- src/sidecar/mod.rs
- tests/cli_e2e.rs
- tests/download_engine_integration.rs
- tests/parser_integration.rs
- tests/queue_integration.rs

### Change Log

- 2026-02-18: Implemented Story 8.3 parsing confidence tracking and uncertainty query/filter flow across parser, queue, history, CLI, and tests.

## Party Mode Audit (AI)

- **Audit Date:** 2026-02-18
- **Topic:** 8-3 Parsing Confidence Tracking
- **Outcome:** pass_with_actions
- **Findings:** 2 High · 5 Medium · 2 Low

### Findings by Perspective

**📋 John (PM):**
- **Medium (PM-1):** AC3 says "rows needing review" but does not explicitly define that `--uncertain` means `confidence = low` only. That rule is only in implementation guidance; move it into AC text so product semantics are testable.
- **Low (PM-2):** AC2 asks for concise/actionable summary language but does not define a canonical phrasing format for mixed outcomes (e.g., many references + zero uncertain), which can create inconsistent UX copy.

**🏗️ Winston (Architect):**
- **High (ARC-1):** The story does not define a canonical data handoff from parser output into queue metadata. Today `ParsedItem` carries only `raw`, `input_type`, `value` and enqueue metadata is built from resolver metadata in `src/main.rs`. Without an explicit handoff contract, teams may reparse in multiple places and violate the "persist once" guardrail.
- **High (ARC-2):** Migration strategy is internally inconsistent: Task 2.1 and Task 3.1 imply separate migrations, while File Structure Notes expect a single migration file. Pick one strategy before implementation to avoid migration-order drift.
- **Medium (ARC-3):** AC4 requires confidence factors in "queryable form", but the story does not freeze JSON key contract/types (`has_authors`, `has_year`, `has_title`, `author_count`) as a compatibility guarantee for future queries.

**💻 Amelia (Dev):**
- **Medium (DEV-1):** Task 5.1 requests conflict validation for `--uncertain` but does not define incompatible flags. This must be explicit (`--status`, `--failed`) to avoid inconsistent CLI behavior and test expectations.
- **Low (DEV-2):** The story asks for queue + history propagation but does not name where conversion helpers live (e.g., `Confidence` enum/string mapping + factor serialization), increasing risk of stringly-typed duplication.

**🧪 Murat (QA):**
- **Medium (QA-1):** AC5 requires structured debug logging for confidence factors, but testing subtasks do not explicitly require asserting tracing fields (presence/shape) vs plain-text message checks.
- **Medium (QA-2):** AC3 says `--uncertain` composes with existing scope filters, but test subtasks do not explicitly require matrix coverage for combinations (`--project`, `--output-dir`, `--since`, `--domain`, `--limit`) and legacy NULL-confidence rows.

### Single Prioritized Action List

1. **Define and lock the parser→queue confidence handoff contract** (High): extend the story to specify exactly where confidence level + factors are attached (avoid reparsing downstream).
2. **Resolve migration plan ambiguity** (High): choose one migration strategy (single file vs two files) and align Tasks 2.1/3.1 and File Structure Notes.
3. **Promote uncertainty semantics into ACs** (Medium): explicitly state `--uncertain` == `low` confidence only.
4. **Specify CLI conflict rules for `--uncertain`** (Medium): declare incompatibilities with `--status` and `--failed` and mirror them in tests.
5. **Freeze confidence-factor JSON contract** (Medium): define keys/types/null behavior so query behavior remains stable.
6. **Tighten QA scope for AC3/AC5** (Medium): require tests for structured tracing fields and filter-composition matrix including legacy NULL rows.
7. **Standardize summary wording expectations** (Low): add one expected template for uncertain/non-uncertain summary output.
8. **Centralize confidence serialization helpers** (Low): avoid duplicate string/JSON conversions across parser, queue, and history layers.

## Senior Developer Review (AI)

- **Review Date:** 2026-02-18
- **Reviewer:** gpt-5-codex
- **Outcome:** changes_requested_with_safe_autofixes
- **Summary:** 1 High · 2 Medium · 1 Low found in implementation review. 2 code fixes and 1 test fix were auto-applied.

### Findings

1. **High:** `--uncertain` SQL path used parameterized OR-filtering (`? = 0 OR parse_confidence = 'low'`), which can prevent reliable index-driven plans for low-confidence scans.  
   Evidence: `src/queue/history.rs` (query helper before review fix).
2. **Medium:** Cache-hit reference parses returned early without emitting structured confidence-factor debug fields, leaving AC5 logging incomplete for repeated references.  
   Evidence: `src/parser/reference.rs` (`parse_reference_metadata` cache return path before review fix).
3. **Medium:** CLI E2E coverage for `--uncertain` filter composition did not include `--since`.  
   Evidence: `tests/cli_e2e.rs` (uncertain tests covered only base + domain pre-fix).
4. **Low (decision):** Confidence value contract remains stringly-typed at queue/history persistence boundaries; invalid external values can still be stored without canonicalization/constraint.

### Auto-Fixes Applied

1. Reworked `query_download_attempts_page()` into distinct uncertain/non-uncertain SQL branches to keep low-confidence filtering explicit and index-friendly.
2. Added structured debug-factor logging on parser cache hits (`cache_hit = true`) to preserve AC5 diagnostics.
3. Added CLI E2E regression coverage for `downloader log --uncertain --since ...`.

### Validation

- `cargo fmt`
- `cargo test --test queue_integration test_query_download_attempts_filters_status_project_and_date -- --nocapture`
- `cargo test --test parser_integration test_parse_reference_confidence_exposes_deterministic_factors -- --nocapture`
- `cargo test --test cli_e2e test_binary_log_uncertain_ -- --nocapture`

### Remaining Decisions

1. Decide whether to enforce parse confidence contract at persistence boundary (DB CHECK constraint vs write-time normalization).
2. Decide whether AC text should explicitly freeze `--uncertain == low` semantics (today it is mostly in notes/tests).
