# Epic 2 Retrospective - Smart Reference Resolution

- Epic: Epic 2 - Smart Reference Resolution
- Date: 2026-02-15
- Facilitator: fierce
- Participants: solo async
- Stories in scope: 2-1, 2-2, 2-3, 2-4, 2-5, 2-6, 2-7
- Outcome: Healthy with Actions

## 1. Epic Goal vs Delivered Value

### Epic Goal
"I can paste DOIs and references."

### Delivered Capability Checklist

- DOI detection and validation: Done
  - Evidence: Story 2-1 status is `done`; parser DOI extraction and normalization are active.
- Resolver trait and registry: Done
  - Evidence: Story 2-2 status is `done`; resolver registry and trait-based dispatch are in source.
- Crossref DOI resolution: Done
  - Evidence: Story 2-3 status is `done`; Crossref resolver is implemented and registered.
- Reference string parsing: Done
  - Evidence: Story 2-4 status is `done`; reference metadata parsing and confidence logic exist.
- Bibliography extraction: Done
  - Evidence: Story 2-5 status is `done`; bibliography segmentation and parsing are present.
- BibTeX format support: Done
  - Evidence: Story 2-6 status is `done`; BibTeX parsing for supported entry types is present.
- Mixed-format handling: Done
  - Evidence: Story 2-7 status is `done`; parser supports URL/DOI/reference/BibTeX mixed input.

### User-Visible Outcome

- Users can paste mixed inputs containing direct URLs, DOIs, references, and BibTeX entries in one batch.
- DOI inputs can be classified and resolved through the resolver pipeline before download queueing.
- Parsing reports type-level counts and surfaces skipped malformed entries without aborting valid neighbors.

### Known Limitations

- Crossref coverage depends on remote metadata quality and API behavior.
- Resolver breadth is currently limited to implemented resolvers; more site-specific resolvers are deferred.
- Queue source typing now preserves `bibtex`, but broader history/UX surfaces for that type are deferred to future stories.

## 2. Quality and Reliability Review

### Quality Gates Evidence

- `cargo fmt --check`: pass (latest run during Story 2-7 review-fix cycle, 2026-02-15)
- `cargo clippy -- -D warnings`: pass (latest run during Story 2-7 review-fix cycle, 2026-02-15)
- `cargo test --lib parser::`: pass during Story 2-7 delivery
- `cargo test --test parser_integration`: pass during Story 2-7 delivery
- `cargo test --test queue_integration`: pass, including mixed parser output source-type regression
- `cargo test --test cli_e2e test_binary_malformed_input_surfaces_skipped_output`: pass (2026-02-15)

### Defect Review

- Escaped defects found during Epic 2 code-review phases: 4 material findings in Story 2-7.
- Critical/High issues fixed before close:
  - Non-URL items previously reached downloader path without resolver routing (`src/main.rs`) - fixed.
  - Malformed-only input skipped diagnostics not surfaced in CLI empty-item path (`src/main.rs`) - fixed.
- Medium issues fixed before close:
  - BibTeX source type collapsed into reference (`src/parser/input.rs`, queue schema/mapping) - fixed with explicit `bibtex`.
  - Story file list drift for review-fix changes (`_bmad-output/implementation-artifacts/2-7-mixed-format-input-handling.md`) - corrected.
- Residual risks accepted:
  - No unresolved High/Medium blockers remaining for Epic 2 completion.

## 3. Throughput and Rework

### Throughput

- Stories completed: 7/7
- Re-opened stories: 1 (2-7 received post-review code fixes)
- Avg review cycles per story: 1 primary cycle; 2-7 required one additional fix cycle.

### Rework Pattern

Top patterns observed:
- AC interpreted but not implemented end-to-end (routing path mismatch until review).
- Story File List drift from actual changed files during follow-up fixes.

Root cause summary:
- End-to-end behavior checks were under-emphasized relative to parser-local behavior.
- Story documentation updates were not always synced after late-cycle code-review fixes.

Preventive actions:
- Add a required AC-to-runtime-path checklist before marking review complete.
- Require final git-vs-File-List reconciliation before moving story to `done`.

## 4. What Went Well / Did Not / Learned

### Went Well

- Parser capabilities were delivered incrementally and composed into mixed-format handling by 2-7.
- Adversarial review caught practical behavior gaps before Epic close.
- Regression tests were added where defects were found, reducing repeat risk.

### Did Not Go Well

- Early acceptance leaned too much on parser classification without verifying runtime routing behavior.
- Documentation (File List) briefly diverged from actual code changes during fast follow-up patches.
- Some validation evidence depended on scattered story notes rather than a single consolidated summary.

### Concrete Learnings

- Process:
  - AC validation must include runtime execution path checks, not only static/type checks.
  - Story completion should enforce a final documentation sync gate (File List + Change Log + status).
- Technical:
  - Mixed input systems need explicit source-type preservation end-to-end, including queue schema.
  - Malformed-only input behavior requires explicit CLI tests for skipped-output visibility.

## 5. Action Items

Format: `[ID] [Severity] [Type] Action | Owner | Target Story | Due Trigger | Done Criteria`

- `[E2-R1] [M] [Process] Add mandatory AC end-to-end runtime-path checklist in review workflow output. | Owner: Dev/Reviewer flow | Target: 3-1-input-parsing-feedback | Due: before story set to review | Done: checklist present and explicitly answered in story review section.`
- `[E2-R2] [M] [Docs] Enforce git-vs-File-List reconciliation at story close. | Owner: Reviewer flow | Target: 3-1-input-parsing-feedback | Due: before story set to done | Done: discrepancy count recorded and resolved/accepted in story notes.`
- `[E2-R3] [M] [Test] Expand CLI/parser integration matrix for malformed-only and unresolved-path cases. | Owner: Test discipline | Target: 3-2-progress-spinner-display | Due: during Epic 3 first integration pass | Done: tests fail when skipped diagnostics or routing regress.`
- `[E2-R4] [L] [Code] Preserve and expose source-type metadata consistently (`direct_url`, `doi`, `reference`, `bibtex`) in downstream logs/history. | Owner: Parser/Queue/History modules | Target: 6-1-download-attempt-logging | Due: when history schema/output is extended | Done: history query output can filter/report by all four source types.`
- `[E2-R5] [M] [Process] Run cargo clippy -- -D warnings after every task completion, not just at story completion gate. | Owner: Dev agent workflow | Target: 3-1-input-parsing-feedback | Due: every task boundary | Done: zero clippy warnings at story completion without needing fix-up pass.`
- `[E2-R6] [M] [Test] Strengthen test assertions to verify content, not just counts — assert on specific values, types, and fields, not only collection lengths. | Owner: Dev/Reviewer flow | Target: 3-1-input-parsing-feedback | Due: during test writing | Done: code review finds zero count-only assertions for non-trivial test cases.`
- `[E2-R7] [L] [Docs] Update doc comments referencing "future" or deferred epic language when implementing the feature — do not leave stale forward-looking comments on shipped code. | Owner: Dev agent workflow | Target: 3-1-input-parsing-feedback | Due: during implementation | Done: grep for "future" and "Epic N" in doc comments returns zero stale hits in modified files.`

## 6. Epic 3 Readiness Gate

- Epic 2 stories all `done`: Pass
- Retrospective saved: Pass
- No unresolved High items: Pass
- Next story selected (3-1): Pass
- Dependencies from Epic 2 stable for UX/progress work: Pass

Decision: Ready for Epic 3

## 7. Final Retro Outcome

- Outcome: Healthy with Actions
- Decision statement: Epic 2 delivered the promised DOI/reference/BibTeX capability and is production-ready to hand off to Epic 3, with explicit process/test hardening actions queued.
- Note: Action items increased from 4 to 7 after a second retrospective round identified recurring patterns across all 7 stories (clippy compliance timing, weak test assertions, stale doc comments).
- Top 3 priorities entering Epic 3:
  1. Preserve parser accuracy signals in UX-facing parsing feedback.
  2. Keep end-to-end routing and skipped diagnostics covered by regression tests.
  3. Enforce tighter close-out discipline on story docs and status sync.

## 8. Status Sync Applied

- `development_status.epic-2-retrospective` -> `done`
- `development_status.epic-2` -> `done`
- `2-1` through `2-7` remain `done`

Next epic startup recommendation:
- Set `epic-3` to `in-progress` when beginning 3-1.
- Set `3-1-input-parsing-feedback` to `ready-for-dev` (or `in-progress` if started immediately).
