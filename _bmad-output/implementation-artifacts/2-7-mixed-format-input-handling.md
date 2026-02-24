# Story 2.7: Mixed Format Input Handling

Status: done

<!-- Note: Validation is optional. Run validate-create-story for quality check before dev-story. -->

## Story

As a **user**,
I want **to paste URLs, DOIs, references, and BibTeX entries in one input**,
so that **I do not need to pre-separate content before downloading**.

## Acceptance Criteria

1. **AC1: Mixed-Format Classification**
   - **Given** input containing mixed formats (URLs + DOIs + references + BibTeX)
   - **When** the parser processes the input
   - **Then** each parsed item is classified by type (`url`, `doi`, `reference`, `bibtex`)
   - **And** classification is deterministic across repeated runs with identical input

2. **AC2: Type-Specific Handling**
   - **Given** classified mixed-format parser output
   - **When** items are routed into processing
   - **Then** each item is handled by the appropriate path (direct URL download vs resolver-driven DOI/reference/BibTeX flows)
   - **And** malformed entries are surfaced in skipped output without aborting valid neighbors

3. **AC3: Summary by Type**
   - **Given** mixed-format parsing completes
   - **When** summary data is produced
   - **Then** counts by type are available (`url`, `doi`, `reference`, `bibtex`)
   - **And** total accounting is consistent (`total_parsed + skipped` aligns with extraction results)

4. **AC4: Unified Queue Ingestion**
   - **Given** mixed parsed items from one input
   - **When** queue ingestion begins
   - **Then** all valid items enter the same queue model regardless of source type
   - **And** source type metadata remains available for downstream behavior and logging

## Tasks / Subtasks

- [x] **Task 1: Finalize mixed-format parser contract and classification model** (AC: 1, 3)
  - [x] Confirm canonical type taxonomy and naming for parser output (`InputType`)
  - [x] Define explicit mapping rules from raw input fragments to `url`/`doi`/`reference`/`bibtex`
  - [x] Preserve deterministic merge order and de-duplication guarantees from Stories 2.1-2.6
  - [x] Document treatment for BibTeX dual outputs (BibTeX entry classification + extracted DOI/reference derivatives)

- [x] **Task 2: Implement classification and routing updates in parser pipeline** (AC: 1, 2)
  - [x] Update `parse_input()` residual handling so mixed segments are classified without dropping valid neighboring content
  - [x] Ensure BibTeX-derived items preserve explicit source typing for downstream routing
  - [x] Keep malformed-isolated behavior: bad DOI/reference/BibTeX fragments do not suppress valid items
  - [x] Keep runtime behavior panic-free in library paths

- [x] **Task 3: Add summary-by-type support** (AC: 3)
  - [x] Add parser summary counters by type and expose through `ParseResult` helper(s) used by CLI/reporting layers
  - [x] Ensure counts stay stable across mixed inputs with duplicates and malformed fragments
  - [x] Confirm summary output remains compatible with upcoming Epic 3 parsing-feedback UX

- [x] **Task 4: Verify unified queue ingestion compatibility** (AC: 2, 4)
  - [x] Validate queue enqueue path accepts all mixed parser item types from one batch
  - [x] Ensure enqueue logic stores source type metadata needed for resolver selection and logging
  - [x] Confirm no type-specific item is silently dropped before queue persistence

- [x] **Task 5: Unit tests for mixed classification and accounting** (AC: 1, 3)
  - [x] Add parser unit tests for canonical mixed samples containing all four types
  - [x] Add tests for ordering stability and DOI dedupe in mixed inputs
  - [x] Add tests ensuring malformed fragments increment skipped while preserving valid items
  - [x] Add tests for summary counters by type and total accounting invariants

- [x] **Task 6: Integration tests for parser-to-queue flow** (AC: 2, 4)
  - [x] Extend `tests/parser_integration.rs` mixed scenarios to assert explicit type classification and summary counts
  - [x] Add/extend queue integration coverage to validate ingestion of mixed parser output in one batch
  - [x] Add regression case for line-level mixtures (URL/DOI/reference on same line) to prevent content loss

- [x] **Task 7: Quality gates and completion evidence** (AC: all)
  - [x] `cargo fmt --check`
  - [x] `cargo clippy -- -D warnings`
  - [x] `cargo test --lib parser::`
  - [x] `cargo test --test parser_integration`
  - [x] `cargo test --test queue_integration`

## Dev Notes

### Story Context and Intent

- This story completes Epic 2 parser composition by making mixed-format input behavior explicit and reliable across all currently supported input types.
- Stories 2.1-2.6 already introduced DOI extraction, resolver registry usage, reference parsing, bibliography segmentation, and BibTeX support; Story 2.7 must compose these without regressions.
- Primary risk to avoid: silently dropping valid items when one line or segment mixes multiple patterns.

### Current Implementation Baseline

- `src/parser/mod.rs` already applies staged extraction:
  - DOI extraction first (with DOI de-duplication),
  - URL extraction second (excluding `doi.org` URLs from URL output),
  - residual parsing for bibliography/reference and BibTeX segments.
- `process_residual_content()` currently merges bibliography and BibTeX-derived outputs; this story formalizes mixed-type classification and summary behavior expected by downstream processing.
- Line-level fragment stripping is handled by `strip_matched_fragments()` and is a critical surface for mixed-line correctness.

### Continuity from Story 2.6

- BibTeX parsing now supports malformed-segment isolation and consumed-segment removal from residual text; preserve this behavior.
- Existing DOI de-duplication contract must remain: canonical DOI wins by first extractor phase order.
- Existing regression coverage includes line-level URL/reference and DOI/reference combos; extend rather than replace.

### Scope Boundaries

- In scope: parser classification/routing behavior, parse summary accounting, and queue-ingestion compatibility for mixed batches.
- Out of scope: new resolver implementations, downloader engine behavior changes, and Epic 3 UX formatting/presentation polish beyond exposing required counts.

### Technical Requirements

- Maintain deterministic parser output ordering for mixed input:
  - DOI items first (canonicalized, deduplicated),
  - non-DOI URL items next,
  - residual reference/bibliography items next,
  - BibTeX-derived outputs merged according to explicit contract.
- Preserve and clarify type classification semantics:
  - `ParsedItem.input_type` must reflect intended downstream handling.
  - If BibTeX content emits derived DOI/reference items, classification and source tracking must remain unambiguous.
- Ensure mixed-line extraction correctness:
  - A line containing URL/DOI plus reference text must keep both representations when valid.
  - Fragment stripping must remove only consumed tokens, not neighboring reference payload.
- Enforce malformed isolation:
  - One malformed DOI/reference/BibTeX fragment cannot suppress valid siblings in the same paste.
  - Malformed segments must produce actionable skipped entries (What/Why/Fix style where applicable).
- Summary/accounting contract:
  - Type-level counts must be derivable from `ParseResult`.
  - Total invariants must hold under mixed + malformed + duplicate input.
- Queue ingestion contract:
  - All valid parsed items from one mixed batch must enter unified queue pathways.
  - Source type metadata required by resolver/queue/logging must remain available.

### Architecture Compliance

- Preserve architecture boundary from `architecture.md`:
  - `src/parser/` performs parsing/classification only (pure/synchronous where practical).
  - No parser code may call download, storage, or network layers.
- Keep resolver architecture assumptions intact:
  - Resolver selection remains driven by parsed item type and downstream registry logic, not parser-side resolver invocation.
- Respect module ownership:
  - Parser changes should remain in parser modules plus minimal call-site integration in queue/CLI as needed.
  - Queue/storage ownership remains in queue/storage modules.
- Maintain error-handling architecture:
  - Library code uses structured errors and skipped outputs; no panics for runtime parse paths.
  - User-facing diagnostics should remain actionable and consistent with existing What/Why/Fix pattern.
- Preserve observability expectations:
  - Continue using `tracing` in parser public entry points and keep summary telemetry coherent for mixed inputs.

### Library / Framework Requirements

- Keep current dependency strategy; no new crates unless strictly required by demonstrated parser gaps.
- Reuse existing parser stack and patterns:
  - `regex` for extraction patterns,
  - `url` for URL parsing/host checks,
  - existing parser helper modules (`doi`, `reference`, `bibliography`, `bibtex`) for composition.
- Keep async boundaries unchanged:
  - Parser code remains synchronous; async behavior belongs in downstream resolver/download/queue orchestration layers.
- Use existing error and logging conventions:
  - `thiserror` patterns in library modules where needed,
  - `tracing` instrumentation for parser entry points and relevant diagnostics.
- Follow project-context rule: avoid dependency feature expansion unless justified and documented.

### Project Structure Notes

- Primary implementation files:
  - `src/parser/mod.rs` (mixed-flow orchestration, ordering, residual handling)
  - `src/parser/input.rs` (type model + summary helper additions)
  - `src/parser/bibtex.rs` (classification/source metadata adjustments if required)
  - `src/parser/reference.rs` and/or `src/parser/bibliography.rs` (only if mixed residual behavior needs corrections)
- Integration touch points:
  - `tests/parser_integration.rs` (mixed-flow regressions, ordering, counts)
  - `tests/queue_integration.rs` (unified queue ingestion assertions for mixed batches)
- Keep module boundary strict:
  - Do not introduce parser logic into queue/download/resolver internals.
  - Do not bypass existing parser APIs from CLI/queue paths.
- Naming and type consistency:
  - Reuse `InputType` and `ParsedItem` conventions in `src/parser/input.rs`.
  - If new helpers are added, follow RFC-430 naming and existing parser module style.

### Testing Requirements

- Unit tests (parser-local):
  - Cover mixed inputs containing URL + DOI + reference + BibTeX in one paste.
  - Cover line-level mixtures where one line contains DOI/URL and reference text.
  - Cover malformed isolation: one bad segment does not block valid neighbors.
  - Cover deterministic ordering and dedupe invariants.
  - Cover summary-by-type counts and total accounting consistency.
- Integration tests (cross-module behavior):
  - Parser integration for full mixed-flow scenarios and stable output classification.
  - Queue integration confirming all valid parsed items from mixed input are enqueued.
- Regression focus from prior findings:
  - Prevent reintroduction of “line contains DOI/URL + reference text loses reference” behavior.
  - Keep prior BibTeX malformed recovery and author normalization regressions green.
- Required quality gates:
  - `cargo fmt --check`
  - `cargo clippy -- -D warnings`
  - `cargo test --lib parser::`
  - `cargo test --test parser_integration`
  - `cargo test --test queue_integration`

### Previous Story Intelligence

- Story 2.6 introduced and hardened BibTeX parsing; do not duplicate parser branches for mixed handling.
- Story 2.6 review fixes already addressed:
  - malformed BibTeX segment recovery without swallowing neighbors,
  - author normalization for `and` variants,
  - consumed-segment removal to avoid reference duplication.
- Reuse existing mixed-line regression patterns from parser tests rather than creating parallel test style.
- Maintain compatibility with current parser ordering contract so downstream behavior remains predictable.

### Git Intelligence Summary

- Repository history is short (`Initial commit`, `Prepare first version`), so git-level pattern signal is limited.
- Current working patterns are better inferred from existing parser/test modules than commit evolution.
- Practical guidance:
  - keep changes tightly scoped to parser + related tests,
  - avoid unrelated refactors while implementing Story 2.7,
  - preserve existing test naming and module organization conventions.

### Latest Technical Information

- Web check performed against official docs.rs crate pages for stack drift awareness.
- Observed current docs.rs latest tracks at time of check:
  - `tokio`: latest documentation track is `1.48.0`; project currently uses major `1` (compatible intent).
  - `reqwest`: latest documentation track is `0.12.24`; project currently pins `0.13` and should remain on project baseline for this story.
  - `clap`: docs.rs shows `4.5.49`; project uses `4.5` (compatible major/minor intent).
  - `regex`: docs.rs shows `1.12.2`; project uses `1` (compatible intent).
  - `sqlx`: docs.rs shows `0.8.6`; project uses `0.8` (compatible intent).
- Story 2.7 recommendation:
  - Do not perform dependency upgrades as part of this story.
  - Keep focus on parser behavior and regression safety; handle dependency upgrades in dedicated maintenance stories.

### Project Context Reference

- Enforce `_bmad-output/project-context.md` rules during implementation:
  - no `.unwrap()`/`.expect()` in library runtime paths,
  - `tracing` over `println!`,
  - deterministic and isolated tests,
  - import ordering and naming conventions,
  - run fmt/clippy/tests before marking work complete.
- Story requirements take precedence if any rule conflict appears, but no conflict is expected for this scope.

### References

- [Source: _bmad-output/planning-artifacts/epics.md#Story-2.7-Mixed-Format-Input-Handling]
- [Source: _bmad-output/planning-artifacts/epics.md#Story-2.6-BibTeX-Format-Support]
- [Source: _bmad-output/planning-artifacts/prd.md#FR-1-Input-Parsing]
- [Source: _bmad-output/planning-artifacts/prd.md#FR-2-Download-Engine]
- [Source: _bmad-output/planning-artifacts/architecture.md#Data-Flow-Architecture]
- [Source: _bmad-output/planning-artifacts/architecture.md#Module-Ownership-Mapping]
- [Source: _bmad-output/planning-artifacts/ux-design-specification.md#Input-Feedback-Pattern]
- [Source: _bmad-output/planning-artifacts/ux-design-specification.md#Progress-Design]
- [Source: _bmad-output/project-context.md]
- [Source: _bmad-output/implementation-artifacts/2-6-bibtex-format-support.md]
- [Source: https://docs.rs/crate/tokio/latest]
- [Source: https://docs.rs/crate/reqwest/latest]
- [Source: https://docs.rs/crate/clap/latest]
- [Source: https://docs.rs/crate/regex/latest]
- [Source: https://docs.rs/crate/sqlx/latest]

### Story Completion Status

- Story context generated and saved to `_bmad-output/implementation-artifacts/2-7-mixed-format-input-handling.md`.
- Story status remains `ready-for-dev` for handoff to `dev-story`.
- Completion note: Ultimate context engine analysis completed - comprehensive developer guide created.

## Dev Agent Record

### Agent Model Used

GPT-5 Codex (CLI)

### Debug Log References

- create-story workflow executed via `_bmad/core/tasks/workflow.xml`
- source analysis performed across epics, PRD, architecture, UX, project-context, prior story, parser code, and parser integration tests
- latest library info checked via docs.rs crate pages (see references)
- sprint status updated for story key `2-7-mixed-format-input-handling`
- implemented parser + queue updates in `src/parser/input.rs`, `src/parser/bibtex.rs`, `src/parser/mod.rs`, and `src/main.rs`
- validation commands run:
  - `cargo fmt --check`
  - `cargo clippy -- -D warnings`
  - `cargo test --lib parser::`
  - `cargo test --test parser_integration`
  - `cargo test --test queue_integration`
  - `cargo test` (full suite, escalated due wiremock sandbox bind limits)

### Completion Notes List

- Generated full Story 2.7 implementation guide with AC-aligned tasks and parser/queue guardrails.
- Included continuity constraints from Story 2.6 to reduce regression risk in mixed-format handling.
- Included test plan and quality gate requirements aligned with project standards.
- Added `InputType::queue_source_type()` mapping to preserve source-type metadata for queue ingestion.
- Added `ParseTypeCounts` and `ParseResult::{type_counts,count_by_type}` for mixed-format summary-by-type accounting.
- Added explicit BibTeX classification items to parser output while preserving DOI/reference derivative extraction and dedupe behavior.
- Updated CLI queue ingestion to use parsed item source type metadata instead of hardcoded `direct_url`.
- Added/updated parser and queue integration tests for mixed-format classification, ordering, and queue ingestion coverage.
- Full regression suite passed; story status advanced to `review`.
- Code review follow-up fixes applied:
  - routed DOI/reference/BibTeX queue inputs through resolver pipeline before download queueing,
  - ensured skipped parser output is surfaced even when no valid items are queued,
  - preserved explicit `bibtex` source-type metadata in queue schema and mapping,
  - added regression check for malformed-only CLI input skipped-output visibility.

### File List

- `src/main.rs`
- `src/lib.rs`
- `src/parser/input.rs`
- `src/parser/bibtex.rs`
- `src/parser/mod.rs`
- `src/queue/item.rs`
- `src/queue/mod.rs`
- `migrations/20260128000001_create_queue_table.sql`
- `tests/parser_integration.rs`
- `tests/queue_integration.rs`
- `tests/cli_e2e.rs`
- `_bmad-output/implementation-artifacts/2-7-mixed-format-input-handling.md`
- `_bmad-output/implementation-artifacts/sprint-status.yaml`

### Change Log

- 2026-02-15: Implemented Story 2.7 mixed-format classification and queue-ingestion updates; added type-count summaries, BibTeX classification markers, CLI source-type mapping, and parser/queue regression coverage.
- 2026-02-15: Applied code-review fixes for Story 2.7; added resolver-driven routing for non-URL inputs, restored skipped-output surfacing on empty-queue outcomes, and preserved explicit `bibtex` queue source metadata.
- 2026-02-15: Second code review fixes: fixed CLI E2E test timeout (127.0.0.1:1 + -r 0), added ParseTypeCounts/ParsedItem to lib.rs re-exports, updated BibTex doc comment, added bibtex tracing counter, fixed ResidualMergeStats field naming for clippy compliance.

## Senior Developer Review (AI) - Round 1

Reviewer: fierce
Date: 2026-02-15
Outcome: Approve

### Findings Resolved

- HIGH: Non-URL parsed items were previously sent directly to downloader URL path; fixed by resolver-driven pre-queue routing in `src/main.rs`.
- HIGH: Malformed-only input skipped entries were hidden on empty parsed output; fixed by warning skipped entries before empty-result early return in `src/main.rs`.
- MEDIUM: BibTeX source metadata collapsed into `reference`; fixed with explicit `bibtex` source type mapping and queue schema support.
- MEDIUM: Story file list incompleteness for review-fix changes; fixed by updating File List and Change Log.

### Validation Evidence

- `cargo fmt --check`
- `cargo clippy -- -D warnings`
- `cargo test --test queue_integration test_enqueue_mixed_parser_output_preserves_source_type_metadata`
- `cargo test --test cli_e2e test_binary_malformed_input_surfaces_skipped_output`

## Senior Developer Review (AI) - Round 2

Reviewer: fierce
Date: 2026-02-15
Outcome: Approve

### Findings Resolved

- HIGH: CLI E2E test `test_binary_stdin_with_invalid_domain_exits_cleanly` took 94s due to unreachable IP network timeout; fixed by using `127.0.0.1:1` (instant connection refused) and `-r 0` to skip retries.
- MEDIUM: `ParseTypeCounts` not re-exported from `parser/mod.rs` or `lib.rs`; fixed by adding to both re-export lists along with `ParsedItem`.
- MEDIUM: `InputType::BibTex` doc comment was stale (`"BibTeX entry (future - Epic 2)"`); updated to `"BibTeX entry (@article, @book, @inproceedings)"`.
- MEDIUM: `parse_input()` tracing summary missing bibtex count; added `bibtex_count` tracking and `bibtex` field to structured log output.
- LOW: `ResidualMergeStats` field names all shared `_added` postfix (clippy `struct_field_names` lint); renamed to `references`, `bibtex`, `errors`.
- NOT A BUG: CLI E2E `.stdout()` assertion is correct; `tracing_subscriber::fmt()` 0.3.x defaults to stdout.

### Validation Evidence

- `cargo fmt --check`: PASS
- `cargo clippy -- -D warnings`: PASS
- `cargo test --lib parser::`: 167/167 PASS
- `cargo test --test parser_integration`: 30/30 PASS
- `cargo test --test queue_integration`: 25/25 PASS
- `cargo test --test cli_e2e`: 11/11 PASS (previously-slow test now ~1s)
