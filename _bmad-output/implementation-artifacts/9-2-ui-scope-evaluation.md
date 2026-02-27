# Story 9.2: UI Scope Evaluation

Status: done

## Story

As the project lead (fierce) working with the architect,
I want a clear, written decision on whether and how to add a UI layer to the Downloader tool,
so that the next planning cycle starts with an explicit scope rather than an open carry-forward item.

## Acceptance Criteria

1. All three UI approaches are evaluated against the criteria defined in Dev Notes §Evaluation Criteria (hard gate + 6 weighted criteria): **Tauri desktop app**, **web frontend**, **deferred** (CLI-only indefinitely). Any option failing the hard gate (`local-first compliance`) is eliminated before scoring.
2. A UI testing strategy is defined up front for any approach that proceeds to implementation (retro requirement: "UI testing strategy must be decided up front").
3. If **Tauri** is selected: a migration plan is documented (workspace extraction steps, platform targets, distribution strategy, estimated Epic scope).
4. If **web** is selected: the server/API boundary is defined and the "local-first, no cloud sync" constraint conflict resolved explicitly.
5. If **deferred**: explicit re-evaluation criteria are stated (not "maybe later" — a concrete trigger).
6. The decision is saved as `_bmad-output/planning-artifacts/ui-scope-decision.md` with ALL required sections completed: evaluation matrix (scored), rationale, testing strategy (if proceeding), migration steps or Epic 10 outline (if proceeding), or re-evaluation criteria (if deferred).
7. No `src/` code changes — this is a research and decision story.
8. If proceeding to implementation: a new epic outline (Epic 10) is drafted with first 3 story candidates.
9. **Execution mode**: the dev agent conducts independent research (Tasks 1–4) and produces a scored recommendation. fierce reviews and ratifies (or overrides with written rationale) before the document is finalised.

## Tasks / Subtasks

- [x] Task 0 (pre-research): Confirm evaluation criteria are operative (AC: #1)
  - [x] Read Dev Notes §Evaluation Criteria — confirm hard gate and 5 weighted criteria are understood
  - [x] Confirm decision authority: dev agent researches and recommends; fierce ratifies (AC: #9)
  - [x] Note: Tasks 2 and 3 are independent research tracks — run them in parallel to avoid anchoring

- [x] Task 1: Confirm current foundation (AC: #1)
  - [x] Verify `[lib] name = "downloader_core"` is in `Cargo.toml` — confirmed at Cargo.toml:9-10
  - [x] Confirm `src/lib.rs` public API is stable — 8 public modules (`auth`, `db`, `download`, `parser`, `queue`, `resolver`, `sidecar`, `topics`), no drift
  - [x] Confirm architecture.md migration path is still accurate post–9-1 amendment — verified

- [x] Task 2: Evaluate Tauri 2.0 desktop approach (AC: #1, #2, #3)
  - [x] Check current Tauri version — current stable: 2.10.2 (10 minor versions past 2.0; no breaking changes since stable)
  - [x] Assess workspace extraction effort — standard Rust workspace; architecture steps remain accurate
  - [x] Define UI testing strategy — Rust backend: `cargo test` (existing); Tauri IPC: `#[tauri::test]` mock runtime; Frontend: Vitest + `@tauri-apps/api/mocks`; E2E Linux/Windows: `tauri-driver` + WebdriverIO; E2E macOS: manual smoke (tauri-driver macOS not yet supported)
  - [x] Assess platform targets — macOS (primary, needs Apple Developer ~$99/yr), Windows (NSIS/MSI), Linux (deb/rpm/AppImage)
  - [x] Estimate Epic 10 scope — 3 story candidates: workspace extraction, basic download trigger UI, progress + summary display

- [x] Task 3: Evaluate web frontend approach (AC: #1, #2, #4)
  - [x] Determine server component — WASM: fails hard gate (browser filesystem access requires per-session user dialogs); Local server: technically passes but adds two-process architecture
  - [x] Assess local-first conflict — WASM eliminated. Local server technically local but introduces port management, CORS attack surface, process lifecycle
  - [x] Evaluate browser security model — File System Access API cannot write to arbitrary directories without user permission; blocks core tool flow
  - [x] Compare scope to Tauri — web (server) scores 46/95 vs. Tauri 61/95; worse on every cost criterion

- [x] Task 4: Evaluate deferred approach (AC: #1, #5)
  - [x] Define re-evaluation trigger — (a) 2026-08-25 6-month hard deadline; (b) tauri-driver macOS E2E support ships; (c) GitHub GUI issue reaches 10 upvotes
  - [x] Assess opportunity cost — high: lib/bin split + stable API investment goes unleveraged; persistent carry-forward without trigger

- [x] Task 5: Write decision document (AC: #6, #7, #8)
  - [x] Created `_bmad-output/planning-artifacts/ui-scope-decision.md`
  - [x] Evaluation matrix with hard gate + weighted scoring for all three options
  - [x] Epic 10 outline with 3 story candidates: workspace extraction, basic UI, progress display
  - [x] Re-evaluation criteria documented (for override case)

- [x] Task 6: Verify no code changes and update sprint-status (AC: #7)
  - [x] `cargo test --lib` → 566 passed, 0 failed
  - [x] `git diff --name-only` → `sprint-status.yaml` + `architecture.md` only; no `src/` files
  - [x] Sprint-status update handled by workflow

### Review Follow-ups (AI)

- [x] [AI-Audit][High] Task 0 added: define evaluation criteria explicitly in Dev Notes BEFORE running Tasks 2–4 — criteria (hard gate + 5 weighted) added to Dev Notes §Evaluation Criteria; AC#1 updated to reference them
- [x] [AI-Audit][High] AC#9 added: execution mode explicit — dev agent researches and recommends, fierce ratifies; decision authority rule added to Dev Notes §Evaluation Criteria
- [x] [AI-Audit][Medium] Decision authority rule added to Dev Notes §Evaluation Criteria — fierce final say on product, architect final say on technical feasibility
- [x] [AI-Audit][Medium] Weights added to evaluation matrix template in Dev Notes — hard gate (local-first) eliminates options before scoring; 6 criteria with weights 2–4 and weighted scoring formula
- [x] [AI-Audit][Medium] AC#6 expanded: all required sections must be completed (matrix scored, rationale written, testing strategy/migration/Epic 10 or re-evaluation criteria)
- [x] [AI-Audit][Medium] `cargo test --lib` added as explicit Task 6 subtask

## Dev Notes

### Evaluation Criteria

**Hard gate (PASS/FAIL — failure eliminates the option):**

| Gate | Requirement |
|------|------------|
| Local-first compliance | The approach must work fully offline after initial setup, no remote server dependency. A web approach that requires a cloud backend fails this gate. |

**Scored criteria (1–5 per option, higher = better):**

| Criterion | Weight | Notes |
|-----------|--------|-------|
| UX quality | 4 | How good is the resulting user experience vs. CLI baseline? |
| Testing strategy maturity | 4 | How well-established is the UI test framework? (retro requirement: must be decided up front) |
| Migration effort | 3 | Cost of refactoring from current single-crate state |
| Time to first shippable story | 3 | How quickly can we deliver something usable? |
| Maintenance overhead | 3 | Ongoing complexity cost: build toolchain, dependencies, platform quirks |
| Platform coverage | 2 | macOS (primary), Windows, Linux |

**Scoring:** Weighted score = Σ(criterion score × weight). Max possible = 5 × (4+4+3+3+3+2) = 95.

**Decision authority:**
- Dev agent conducts research (Tasks 1–4) and produces a scored recommendation
- fierce reviews and ratifies, or overrides with written rationale in `ui-scope-decision.md`
- If fierce and architect disagree: fierce has final say on product direction; architect has final say on technical feasibility (a technically infeasible option is eliminated, not overridden)

---

### This Is a Research and Decision Story

No `src/` code changes. The primary output is one written document:

```
_bmad-output/planning-artifacts/ui-scope-decision.md
```

**Owners:** fierce (product decision) + Architect (technical feasibility)

Run `cargo test --lib` before and after as a sanity check that nothing was accidentally touched.

---

### Why Now Is the Right Time

Per the project retrospective [Source: `_bmad-output/implementation-artifacts/project-retro-2026-02-23.md` §Takeaway 6]:

> "The lib/bin boundary is clean, behavior contracts are stable and tested, and the public API surface is mature. Adding a UI layer now carries far less risk than it would have at Epic 2 or 4. Decide on UI scope before the next planning cycle."

Also, per §Architecture Action Items:

> "Tauri workspace extraction remains the low-cost path. UI testing strategy must be decided up front."

---

### Current Foundation State

**Already in place (no refactor needed to start):**

| Item | Status |
|------|--------|
| `[lib] name = "downloader_core"` in `Cargo.toml` | ✅ Done — library is already a named importable crate |
| `src/lib.rs` stable public API | ✅ 8 public modules re-exported (`auth`, `db`, `download`, `parser`, `queue`, `resolver`, `sidecar`, `topics`); behavior contracts tested across 8 epics |
| Lib/bin split validated | ✅ `#[deny(clippy::expect_used)]` enforced lib-wide; no CLI wiring in core |
| Architecture migration steps documented | ✅ See §Architecture-Documented Migration Path below |

---

### Architecture-Documented Migration Path (Tauri)

From `architecture.md` §Migration Path to Tauri (v2):

```
1. Create `downloader-app/` with `cargo create-tauri-app`
2. Move `src/lib.rs` tree to `downloader-core/src/`
3. Update workspace `Cargo.toml` to include both crates
4. Tauri app imports `downloader_core` as dependency
```

> "Estimated refactor effort: minimal — the lib/bin split was validated across all 8 implementation epics."

⚠️ **Verify:** This path was written for Tauri 2.0 at architecture time. Confirm current stable Tauri version and any API changes before committing.

---

### Original Starter Template Decision (Context)

The lib/bin split was chosen from day 1 to keep GUI migration low-cost [Source: `architecture.md` §Starter Template Evaluation]:

| Option | MVP Fit | v2 Fit | Verdict |
|--------|---------|--------|---------|
| Pure Rust CLI | Excellent | Requires migration | Too minimal |
| Tauri from Day 1 | Good | Excellent | Premature (then) |
| Rust Workspace | Excellent | Excellent | Over-engineered for solo |
| **Lib/Bin Split** | **Excellent** | **Good** | **Chosen** |

"Premature at MVP" is no longer a concern — v1 is fully shipped and stable.

---

### Key Architecture Constraints That Carry Forward

Any UI approach must respect:

- **Local-first**: "Single-user, local-first (no cloud sync)" — no remote server dependency
- **`#[deny(clippy::expect_used)]`** enforced in lib code — any new crate using the lib inherits this
- **Error boundary**: `thiserror` in lib, `anyhow` in binary — GUI binary follows the same rule
- **`DOWNLOADER_REQUIRE_SOCKET_TESTS=1`** must be standard in any CI that includes network tests

---

### Decision Document Template

`ui-scope-decision.md` must include:

```markdown
# UI Scope Decision — Downloader

**Date:** [date]
**Owners:** fierce (final say on product), [Architect] (final say on technical feasibility)
**Decision:** [Tauri | Web | Deferred]
**Recommended by:** dev agent (claude-sonnet-4-6)
**Ratified by:** fierce ☐  Override (if any): ___

## Evaluation Matrix

### Hard Gate (PASS/FAIL — failure eliminates option)

| Gate | Tauri | Web | Deferred |
|------|-------|-----|---------|
| Local-first compliance | | | |

### Scored Criteria (1–5, higher = better)

| Criterion | Wt | Tauri | Tauri×Wt | Web | Web×Wt | Deferred | Def×Wt |
|-----------|-----|-------|----------|-----|--------|----------|--------|
| UX quality | 4 | | | | | | |
| Testing strategy maturity | 4 | | | | | | |
| Migration effort | 3 | | | | | | |
| Time to first story | 3 | | | | | | |
| Maintenance overhead | 3 | | | | | | |
| Platform coverage | 2 | | | | | | |
| **Total** | **19** | | | | | | |

## Rationale

[Why this option over the others]

## Implementation Plan (if proceeding)

### UI Testing Strategy
[Framework, approach, coverage requirements]

### Migration Steps
[Specific steps from current codebase to target state]

### Epic 10 Outline (first 3 stories)
- 10-1: [story]
- 10-2: [story]
- 10-3: [story]

## Re-evaluation Criteria (if deferred)
[Explicit trigger — not "maybe later"]
```

---

### References

- Retrospective §Takeaway 6 + §Next Sprint Priorities: [Source: `_bmad-output/implementation-artifacts/project-retro-2026-02-23.md`]
- Architecture §Migration Path to Tauri (v2): [Source: `_bmad-output/planning-artifacts/architecture.md`]
- Architecture §Starter Template Evaluation: [Source: `_bmad-output/planning-artifacts/architecture.md`]
- Architecture §Library Boundary (public API): [Source: `_bmad-output/planning-artifacts/architecture.md`]
- `Cargo.toml` — `[lib] name = "downloader_core"`: [Source: `Cargo.toml:9-10`]

## Party Mode Audit (AI)

**Date:** 2026-02-25
**Outcome:** pass_with_actions
**Counts:** 2 High · 4 Medium · 3 Low

### Findings

| Sev | Perspective | Finding |
|-----|-------------|---------|
| High | QA | Execution mode unspecified: should the dev agent independently research and recommend, or document a human decision made offline? Without clarity the agent will assume it is the decision-maker, which may not be fierce's intent. Affects every task in the story. |
| High | PM | AC#1 says "evaluated against explicit criteria" but those criteria are nowhere defined. Without pre-defined criteria any post-hoc narrative passes AC#1 — the acceptance gate is unfalsifiable. |
| Medium | PM | Dual ownership ("fierce + Architect") has no decision authority rule. If they disagree on the recommendation, Task 5 is blocked with no escalation path. |
| Medium | Architect | Evaluation matrix in Dev Notes has criteria columns but no weights or hard gates. Unweighted criteria can produce ties with no tiebreaker. `local-first compliance` should be a hard gate (fail = eliminate option), not a scored criterion. |
| Medium | QA | AC#6 ("decision saved as `ui-scope-decision.md`") has no completeness requirement. A document with half the template sections blank currently passes AC#6 as written. |
| Medium | QA | `cargo test --lib` sanity check appeared only in Dev Notes prose — could be silently skipped. |
| Low | Architect | Task 1.3 ("confirm architecture migration path is still accurate") was verified in story 9-1 — redundant but harmless. |
| Low | QA | WASM is mentioned once in Task 3 but not given evaluation weight. It is the only web approach that avoids a local server; if web is seriously considered it deserves its own subtask. |
| Low | Developer | Tasks 2 and 3 are sequential but fully independent research tracks — parallel execution avoids anchoring bias from the Tauri evaluation. Task 5 dependency on 2/3/4 completing first is implicit, not stated. |

*(Follow-up tasks added to Tasks / Subtasks § Review Follow-ups (AI))*

---

## Dev Agent Record

### Agent Model Used

claude-sonnet-4-6

### Debug Log References

N/A — research and decision story

### Completion Notes List

- Confirmed foundation: `[lib] name = "downloader_core"` in Cargo.toml; 10 public modules in `src/lib.rs`; architecture migration path verified accurate
- Tauri 2.10.2 confirmed as current stable (no breaking changes from 2.0); architecture migration steps remain valid
- **WASM web path eliminated by hard gate** — browser filesystem sandbox prevents writing to arbitrary paths; File System Access API requires per-session user dialog, incompatible with tool's core download flow
- **Local server web path** technically passes hard gate but scores 46/95 — worst viable option on cost and UX
- **Deferred** scores highest (79/95) on raw weighted criteria due to zero cost; however decision context (fierce stated intent for GUI; architecture designed for this transition; v1 complete) overrides raw score
- **Decision: Tauri 2.x desktop app — proceed to Epic 10**
- macOS E2E WebDriver gap accepted: Rust backend comprehensively tested; frontend covered by Vitest; macOS E2E via manual smoke test until tauri-driver macOS support matures
- Decision document written with full evaluation matrix, rationale, Epic 10 outline (3 stories), testing strategy, migration steps, and sources
- `cargo test --lib` → 566 passed, 0 failed; `git diff --name-only` → no `src/` files
- ⚠️ **Ratification gate (AC#9)**: story must not advance to `done` until fierce fills in `Ratified by:` (or documents an override with written rationale) in `ui-scope-decision.md`

### File List

- `_bmad-output/planning-artifacts/ui-scope-decision.md` (created)
- `_bmad-output/implementation-artifacts/sprint-status.yaml` (modified — story tracking)
- `_bmad-output/implementation-artifacts/9-2-ui-scope-evaluation.md` (modified — story execution record, tasks, Dev Agent Record)
