# Story 2.6: BibTeX Format Support

Status: done

<!-- Note: Validation is optional. Run validate-create-story for quality check before dev-story. -->

## Story

As a **user**,
I want **to paste BibTeX entries**,
So that **I can use exports from reference managers**.

## Acceptance Criteria

1. **AC1: BibTeX Entry Type Recognition**
   - **Given** BibTeX formatted input
   - **When** the parser processes it
   - **Then** `@article`, `@book`, and `@inproceedings` entries are recognized
   - **And** entry boundaries are detected for one or many entries in a single paste

2. **AC2: DOI Field Extraction**
   - **Given** a recognized BibTeX entry containing a DOI field
   - **When** the entry is parsed
   - **Then** DOI values are extracted and normalized for resolver use
   - **And** DOI URL forms (`https://doi.org/...`) are normalized to bare DOI format

3. **AC3: Title/Author/Year Extraction**
   - **Given** a recognized BibTeX entry
   - **When** metadata extraction runs
   - **Then** `title`, `author`, and `year` fields are extracted when present
   - **And** metadata is mapped into parser output structures used by existing resolver/downstream flows

4. **AC4: Multi-Entry Handling**
   - **Given** input containing multiple BibTeX entries
   - **When** parsing completes
   - **Then** all valid entries are processed in deterministic order
   - **And** valid entries are not dropped because a neighboring entry is malformed

5. **AC5: Clear Malformed BibTeX Errors**
   - **Given** malformed BibTeX input
   - **When** parser validation fails
   - **Then** clear, actionable parse errors are surfaced in skipped output
   - **And** messages follow What/Why/Fix style for CLI consumption

## Tasks / Subtasks

**Strict Execution Sequence (follow in order, no skipping):**
1. Lock parsing contract and non-goals.
2. Implement segmentation and field extraction.
3. Add malformed diagnostics and unsupported-entry handling.
4. Integrate with parser coordinator and enforce dedupe/order contract.
5. Complete unit tests, then integration/regression tests.
6. Run quality/performance gates and record results.

- [x] **Task 1: Add BibTeX parser module and result models** (AC: 1, 2, 3, 4, 5)
  - [x] Create `src/parser/bibtex.rs` with pure parsing helpers (no I/O/network/async)
  - [x] Add entry model for parsed BibTeX content (entry type, key, core fields)
  - [x] Add parser API for batch input (`parse_bibtex_entries(input: &str) -> BibtexParseResult`)
  - [x] Define explicit non-goals for this story (full BibLaTeX coverage, CSL conversion, resolver-side changes)

- [x] **Task 2: Implement entry detection and segmentation** (AC: 1, 4)
  - [x] Detect `@article`, `@book`, `@inproceedings` tokens case-insensitively
  - [x] Segment complete entries with brace-balance tracking to support multiline fields
  - [x] Preserve source order for deterministic output
  - [x] Ignore unsupported entry types (`@misc`, `@techreport`, etc.) with informative skipped notes rather than hard failure

- [x] **Task 3: Extract and normalize BibTeX fields** (AC: 2, 3)
  - [x] Parse key fields: `doi`, `title`, `author`, `year`
  - [x] Support field assignment forms `field = {value}` and `field = "value"` (case-insensitive field keys, optional trailing commas)
  - [x] Normalize DOI values to canonical bare format (`10.xxxx/...`)
  - [x] Normalize authors from BibTeX `and` lists into downstream-consumable form
  - [x] Normalize year extraction to a 4-digit numeric year when possible, otherwise classify as uncertain with context
  - [x] Reuse existing DOI/reference normalization patterns where possible (do not duplicate heuristics)

- [x] **Task 4: Malformed input diagnostics** (AC: 5)
  - [x] Add explicit malformed cases (unbalanced braces, missing `@type{`, malformed field assignment)
  - [x] Emit actionable skipped messages with context snippets
  - [x] Ensure parse failures are isolated to bad entries without aborting whole input
  - [x] Ensure diagnostics follow What/Why/Fix pattern consistently for CLI output compatibility

- [x] **Task 5: Integrate into parser coordinator** (AC: 1, 2, 3, 4, 5)
  - [x] Register `mod bibtex;` in `src/parser/mod.rs`
  - [x] Call BibTeX parser in `parse_input()` on residual/unmatched content after DOI/URL stripping
  - [x] Merge parsed BibTeX outputs into existing `ParseResult` items/skipped collections
  - [x] Define and enforce deterministic merge-order contract for mixed inputs (URL/DOI/reference/bibtex) for Story 2.7 stability
  - [x] Define DOI de-duplication contract across extractors (single canonical DOI item, deterministic winner)
  - [x] Specify mapping contract from BibTeX parse results into `ParsedItem` fields used by downstream resolution
  - [x] Keep DOI-first and URL-second precedence behavior intact (no regressions from Stories 2.1-2.5)

- [x] **Task 6: Unit tests for BibTeX parsing** (AC: all)
  - [x] Add tests in `src/parser/bibtex.rs` for each supported entry type
  - [x] Add tests for DOI/title/author/year extraction including quoted/braced values
  - [x] Add edge tests for nested braces, escaped quotes, and multiline field values
  - [x] Add tests for comments/preamble/string blocks and unsupported entry types handling
  - [x] Add malformed BibTeX tests with expected actionable error strings
  - [x] Add multi-entry mixed-validity tests (valid + malformed entries in one paste)

- [x] **Task 7: Integration and regression tests** (AC: all)
  - [x] Add integration tests in `tests/parser_integration.rs` for BibTeX-only and mixed parser scenarios
  - [x] Verify existing URL/DOI/reference/bibliography behavior remains stable
  - [x] Verify malformed BibTeX isolation (bad entry does not suppress neighboring valid entries)
  - [x] Verify mixed-input deterministic ordering and DOI de-duplication behavior
  - [x] Verify parser summary/accounting remains deterministic and consistent

- [x] **Task 8: Quality gates** (AC: all)
  - [x] `cargo fmt --check`
  - [x] `cargo clippy -- -D warnings`
  - [x] `cargo test --lib parser::`
  - [x] `cargo test --test parser_integration`
  - [x] Add/execute a parser performance check for large BibTeX batches (100-150 entries) and record timing in completion notes

**Hard Stop Criteria (must pass before marking done):**
- [x] AC1-AC5 all validated by tests.
- [x] Mixed-input order + DOI de-duplication behavior is covered by integration tests.
- [x] Malformed-entry isolation is proven (bad entry cannot suppress valid neighbors).
- [x] Quality and performance gates completed with results logged in completion notes.

## Dev Notes

### Story Requirements

- Implement BibTeX ingestion support in parser flow for entry types: `@article`, `@book`, `@inproceedings`.
- Extract DOI/title/author/year from recognized entries and feed downstream resolution pipeline.
- Support multiple entries per paste, including multiline fields and mixed valid/invalid entries.
- Surface malformed BibTeX errors with clear remediation hints.

### Developer Context Section

- Epic context: Story 2.6 extends parsing capabilities after DOI detection (2.1), reference parsing (2.4), and bibliography extraction (2.5).
- This story should maintain parser determinism and avoid changing download/resolver behavior outside parser outputs.
- Story 2.7 depends on this work for mixed-format classification and stable parser composition.

### Technical Requirements

- Keep parser code pure and synchronous where practical (`src/parser/*` only).
- Do not introduce panics for runtime parse paths; malformed input must be reported as skipped/error context.
- Treat BibTeX as structured text extraction only in this story; resolver/network/database behavior remains unchanged.
- Preserve deterministic ordering of parsed output items matching input order.
- Ensure DOI extraction from BibTeX does not produce duplicates with existing DOI extractor.
- Define canonical DOI de-duplication and mixed-input merge order before coding integration to avoid ambiguity.
- Keep scope to Story 2.6 requirements only; do not expand to full BibLaTeX feature parity.

### Architecture Compliance

- Respect architecture boundary: parser module must not call resolver/download/storage modules directly.
- Follow existing parse pipeline sequencing in `parse_input()` (DOI/URL extraction first, then structured residual parsing).
- Keep module ownership aligned with architecture mapping where `src/parser/` is isolated and test-heavy.
- No new async runtime usage in parser path.

### Library / Framework Requirements

- Prefer current dependency set for this story; do not add crates unless coverage of malformed/multiline BibTeX is inadequate.
- If adding a BibTeX-specific dependency becomes necessary, evaluate against:
  - parser correctness on malformed input,
  - maintenance health,
  - and compatibility with Rust stable + current project constraints.
- Continue using existing crates/patterns: `regex`, `tracing`, `thiserror` in library code.

### File Structure Requirements

- New file expected:
  - `src/parser/bibtex.rs`
- Modified files expected:
  - `src/parser/mod.rs`
  - `src/parser/input.rs` (only if parse result modeling requires extension)
  - `tests/parser_integration.rs`
  - `src/lib.rs` (re-export only if needed externally)
- Keep test placement consistent: parser unit tests inline; cross-module flows in `tests/`.

### Testing Requirements

- Unit tests in `src/parser/bibtex.rs` must cover:
  - supported entry type detection,
  - multiline entry segmentation,
  - field extraction (`doi`, `title`, `author`, `year`),
  - supported field syntax variants (`{}` vs `""`) and optional trailing commas,
  - nested braces and escaped quote edge cases,
  - unsupported entry type behavior (`@misc`, `@techreport`, etc.),
  - malformed entry diagnostics,
  - mixed valid/malformed batches.
- Integration tests in `tests/parser_integration.rs` must cover:
  - BibTeX-only input,
  - BibTeX plus DOI/URL/reference residual interactions,
  - deterministic ordering and DOI de-duplication across mixed parsers,
  - malformed entry isolation in multi-entry input,
  - deterministic parse counts and skipped reporting.
- Enforce quality gates before marking implementation complete:
  - `cargo fmt --check`
  - `cargo clippy -- -D warnings`
  - `cargo test --lib parser::`
  - `cargo test --test parser_integration`
  - parser performance check for 100-150 BibTeX entries

### Previous Story Intelligence

- Story 2.5 established robust segmentation and conservative false-positive controls in parser flows.
- Reuse 2.5 line-local stripping strategy in `parse_input()` to avoid dropping non-matched content.
- Preserve Unicode-aware author parsing behavior introduced in `reference.rs`.
- Keep summary/counting invariants stable (`found == parsed + uncertain`) where bibliography and BibTeX summaries are combined.

### Git Intelligence Summary

- Recent commits show concentrated ownership in parser and CLI core modules.
- Existing parser files already absorb iterative hardening; keep this change tightly scoped to parser + parser tests.
- Avoid broad refactors in unrelated modules to reduce regression risk.

### Latest Technical Information

- Candidate BibTeX parser crates currently available include:
  - `biblatex` (Rust crate for parsing/formatting BibLaTeX/BibTeX-style data)
  - `nom-bibtex` (nom-based BibTeX parser)
  - `serde_bibtex` (Serde-driven BibTeX parser/serializer)
- Current recommendation for Story 2.6: implement initially with internal parser logic + tests; adopt crate only if malformed/multiline edge cases exceed in-house parser reliability.
- Keep dependency surface minimal unless measurable parser robustness gain is demonstrated.

### Project Context Reference

- Follow `_bmad-output/project-context.md` rules:
  - no `.unwrap()` in library runtime paths,
  - `tracing` instrumentation for public parser functions,
  - deterministic and isolated tests,
  - `cargo fmt`, `cargo clippy -D warnings`, and parser tests passing before completion.

### References

- [Source: _bmad-output/planning-artifacts/epics.md#Story-2.6-BibTeX-Format-Support]
- [Source: _bmad-output/planning-artifacts/epics.md#Story-2.7-Mixed-Format-Input-Handling]
- [Source: _bmad-output/planning-artifacts/prd.md#FR-1-Input-Parsing]
- [Source: _bmad-output/planning-artifacts/prd.md#NFR-1-Performance]
- [Source: _bmad-output/planning-artifacts/architecture.md#srcparser]
- [Source: _bmad-output/planning-artifacts/ux-design-specification.md#Input-Methods]
- [Source: _bmad-output/implementation-artifacts/2-5-bibliography-extraction.md]
- [Source: https://docs.rs/biblatex/latest/biblatex/]
- [Source: https://docs.rs/nom-bibtex/latest/nom_bibtex/]
- [Source: https://docs.rs/serde_bibtex/latest/serde_bibtex/]

### Story Completion Status

- Story file generated with comprehensive implementation context and guardrails.
- Status is set to `ready-for-dev` for handoff to `dev-story` workflow.
- Completion note: Ultimate context engine analysis completed - comprehensive developer guide created.

---

## Dev Agent Record

### Agent Model Used

GPT-5 Codex (CLI)

### Debug Log References

- dev-story workflow execution via `_bmad/core/tasks/workflow.xml`
- implemented Story 2.6 parser scope (`src/parser/bibtex.rs`, `src/parser/mod.rs`, `src/parser/doi.rs`, `src/parser/input.rs`)
- validation gates executed: `cargo fmt --check`, `cargo clippy -- -D warnings`, `cargo test --lib parser::`, `cargo test --test parser_integration`
- performance check executed: `cargo test --lib parser::bibtex::tests::test_parse_bibtex_large_batch_120_entries -- --nocapture`

### Completion Notes List

- Implemented `src/parser/bibtex.rs` with supported entry segmentation (`@article`, `@book`, `@inproceedings`), structured field extraction (`doi`, `title`, `author`, `year`), and actionable malformed diagnostics using What/Why/Fix wording.
- Integrated BibTeX parsing into `parse_input()` residual processing with deterministic merge ordering and canonical DOI de-duplication behavior.
- Preserved DOI-first extraction precedence and added BibTeX-line guard in fragment stripping to avoid mutating BibTeX fields before BibTeX parsing.
- Extended DOI cleanup to handle unmatched trailing braces in DOI suffixes for BibTeX-shaped text.
- Added unit and integration coverage for supported types, edge syntax variants, malformed isolation, mixed-input ordering, and dedupe expectations.
- Performance check result: parsed 120 BibTeX entries in ~23.8ms (`test_parse_bibtex_large_batch_120_entries`).
- Code-review fixes applied:
  - malformed BibTeX recovery now isolates bad segments without swallowing neighboring valid entries,
  - author normalization now handles case/whitespace variants of `and`,
  - residual bibliography parsing now removes all consumed BibTeX candidate segments (including malformed/unsupported) to prevent false reference duplication.

### File List

- `src/parser/bibtex.rs` (NEW)
- `src/parser/mod.rs` (MODIFIED)
- `src/parser/doi.rs` (MODIFIED)
- `src/parser/input.rs` (MODIFIED)
- `tests/parser_integration.rs` (MODIFIED)
- `_bmad-output/implementation-artifacts/2-6-bibtex-format-support.md` (MODIFIED)
- Scope note: workspace contains unrelated pre-existing source changes outside Story 2.6; this list remains Story 2.6-scoped.

### Change Log

- 2026-02-14: Implemented Story 2.6 BibTeX parsing support with parser integration, deterministic mixed-input behavior, DOI de-duplication, malformed diagnostics, expanded tests, and quality/performance gate validation.
- 2026-02-15: Applied code-review fixes for malformed-entry isolation and author normalization hardening; added regression coverage for malformed recovery and `and`-variant author parsing.
- 2026-02-15: Second adversarial code review — fixed 4 issues (1 HIGH, 3 MEDIUM). See Review 2 below.

### Senior Developer Review (AI)

#### Review 1

- 2026-02-15 reviewer pass completed.
- High and medium findings fixed in code and validated with:
  - `cargo fmt --check`
  - `cargo clippy -- -D warnings`
  - `cargo test --lib parser::`
  - `cargo test --test parser_integration`
- Low-priority parser overhead note (reference metadata parsed then discarded) is resolved via cached metadata reuse in `src/parser/reference.rs` (`parse_reference_metadata` + `REFERENCE_METADATA_CACHE`).

#### Review 2

##### Reviewer

fierce (adversarial re-review, Claude Opus 4.6)

##### Date

2026-02-15

##### Outcome

Approved after fixes.

##### Findings and Resolution

- **H1 (HIGH):** `bibtex_brace_delta` in `build_residual_input` (`mod.rs:280-284`) counted braces inside `"..."` quoted strings, desynchronizing brace depth tracking for BibTeX block detection. Fixed by rewriting `bibtex_brace_delta` with quote-aware scanning (escape + in_quotes state), consistent with `segment_entries` in `bibtex.rs`.
- **M1 (MEDIUM):** `build_reference_value` (`bibtex.rs:407`) unconditionally appended `.` to title, producing `".."` when title already ended with a period. Fixed by checking `title.ends_with('.')` before appending.
- **M2 (MEDIUM):** `parse_fields` (`bibtex.rs:354`) used `HashMap::insert` (last-value-wins) for duplicate field names. Standard BibTeX convention is first-value-wins. Fixed by replacing `insert` with `entry().or_insert()`.
- **M3 (MEDIUM):** No test for bare (unquoted/unbraced) BibTeX field values. Fixed by adding 3 new tests: `test_parse_bibtex_bare_field_values`, `test_parse_bibtex_duplicate_field_first_value_wins`, `test_build_reference_value_no_double_period`. Also added 2 tests in mod.rs: `test_bibtex_brace_delta_ignores_braces_inside_quotes`, `test_parse_input_multiline_bibtex_quoted_field_with_brace`.

##### Low Issues (not fixed, noted)

- L1: Dead code branch `recovery == 0` in `segment_entries` (`bibtex.rs:181`) — unreachable since `recovery = i + 1`.
- L2: `InputType::BibTex` variant inconsistently cased vs standard `BibTeX` — pre-existing, cosmetic.

##### Validation

- `cargo fmt --check` passed
- `cargo clippy -- -D warnings` passed
- `cargo test --lib parser::` — 167 passed, 0 failed
- `cargo test --test parser_integration` — 30 passed, 0 failed
