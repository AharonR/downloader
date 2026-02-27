# Story 9.1: Architecture As-Built Amendment

Status: done

## Story

As a developer working on the Downloader codebase,
I want the architecture document to reflect the actual as-built module layout,
so that future contributors and AI agents use an accurate reference instead of a plan that was superseded during implementation.

## Acceptance Criteria

1. The "Complete Project Directory Structure" section in `architecture.md` is replaced with the actual `src/` tree (as documented in the Dev Notes below).
2. The "Module Ownership Mapping" table reflects the actual modules; planned-but-absent modules (`src/storage/`, `src/util/`, `src/error.rs`) are removed and actual new modules (`src/app/`, `src/commands/`, etc.) are added.
3. The cookie storage subsection is corrected: spec'd SQLite table → actual XChaCha20Poly1305 encrypted file + OS keychain master key.
4. ~~The Tauri migration section is updated~~ — **VOIDED/MINIMAL**: Tauri GUI is still the planned next step; an as-built validation note was added to the section but the migration plan and decision record remain intact.
5. The `tests/` structure section reflects the actual test files (see Dev Notes).
6. The CI workflows listed in the doc reflect the actual workflows (`phase-rollout-gates.yml`, `coverage.yml`, not `release.yml`).
7. All architectural decision records (WHY WAL, WHY XChaCha20, WHY thiserror-in-lib, WHY lib/bin split, etc.) are preserved verbatim — this is an as-built amendment, NOT a redesign.
8. No code changes — this story produces only a documentation update to `_bmad-output/planning-artifacts/architecture.md`.

## Tasks / Subtasks

- [x] Task 1: Update Project Directory Structure section (AC: #1)
  - [x] Replace planned `src/` tree with actual as-built tree (see Dev Notes §As-Built Layout)
  - [x] Update `tests/` tree to reflect actual test files
  - [x] Update `.github/workflows/` list to `phase-rollout-gates.yml`, `coverage.yml`, `stress-sidecar-flaky.yml`
  - [x] Added `src/bin/` targets (extract_md_links.rs, stress_sidecar_flaky.rs) found during Task 8 gap check
  - [x] Fixed `engine.rs` + `engine/` Rust 2018-style notation
  - [x] Updated Tauri section: added as-built validation note while keeping GUI as next planned step

- [x] Task 2: Update Module Ownership Mapping table (AC: #2)
  - [x] Remove rows for `src/error.rs`, `src/config/`, `src/storage/`, `src/util/`
  - [x] Add rows for: `src/app/`, `src/commands/`, `src/failure/`, `src/search/`, `src/sidecar/`, `src/topics/`, `src/project/`, `src/user_agent.rs`, `src/db.rs`
  - [x] Updated dependency columns: `src/download/` → db (not storage), `src/queue/` → db (not storage), `src/auth/` → db (not storage)
  - [x] Updated Error Module Structure section: replaced planned unified `src/error.rs` with accurate module-local pattern

- [x] Task 3: Correct cookie storage section (AC: #3)
  - [x] Removed `CREATE TABLE cookies` from SQLite schema; added explanatory comment pointing to encrypted-file approach
  - [x] ADR table "Credential Security: Keychain for Key, File for Data" was already correct — preserved verbatim
  - [x] `auth/storage.rs` holding `KeyStorage` enum already correct in ADR code block — preserved verbatim

- [x] Task 4: Tauri section — NO CHANGE (AC: #4 voided)
  - [x] Tauri GUI remains a planned next step; section is accurate and stays verbatim

- [x] Task 0 (pre-edit): Record ADR section content before any edits (AC: #7)
  - [x] All 6 ADR sections snapshotted before any edits (lines 265–502)

- [x] Task 5: Verify decision records are untouched (AC: #7)
  - [x] All 6 ADR sections confirmed byte-for-byte identical to Task 0 snapshots
  - [x] Sections now at: Decision Priority (~273), Resolver Architecture (~294), Authentication & Security (~355), Data Architecture (~387), Concurrency Model (~425), Resilience & Crash Safety (~468)

- [x] Task 6: Exhaustive grep for removed-module references (AC: #2, audit High)
  - [x] Grep run before editing; all 14 hit locations catalogued
  - [x] All stale refs in: directory trees, Module Ownership table, Error Module Structure, Requirements Coverage, Test Coverage Guidelines, Quick Start key files, Development Sequence — all updated
  - [x] Refs at lines 848 and 1177 (`~/.config/downloader/`) correctly identified as user-facing paths — left unchanged

- [x] Task 7: Add complete updated module dependency map to Dev Notes (AC: #2)
  - [x] Dependency edges captured directly in the Module Ownership Mapping table

- [x] Task 8: Post-edit doc correctness gate (AC: #1, #2)
  - [x] `find src -name "*.rs" | sort` (80 files) matches written tree exactly — zero discrepancies
  - [x] `git diff --name-only` → `architecture.md` + `sprint-status.yaml` only; zero `src/` files touched

### Review Follow-ups (AI)

- [x] [AI-Audit][High] Task 6 added: exhaustive grep for all occurrences of removed modules across all architecture.md sections — 14 hit locations addressed
- [x] [AI-Audit][Medium] Task 0 added: ADR sections snapshotted before any edits; all 6 sections diff-verified unchanged after
- [x] [AI-Audit][Medium] Complete updated module dependency map added directly to Module Ownership Mapping table in architecture.md
- [x] [AI-Audit][Medium] Task 8 added: `find src -name "*.rs" | sort` diff gate — 80 files verified, zero discrepancies
- [x] [AI-Audit][Medium] Task 8 added: `git diff --name-only` check — only architecture.md and sprint-status.yaml appear
- [x] [AI-Audit][Medium] Section heading anchors addressed: grep targets (section names + line estimates) captured in Task 5 subtasks; grep performed directly during execution

## Dev Notes

### This Is a Documentation-Only Story

No `src/` code changes. No tests to write. The output is a single updated file:
`_bmad-output/planning-artifacts/architecture.md`

Run `cargo test` before and after as a sanity check that nothing was accidentally touched.

---

### As-Built `src/` Layout (ground truth — use this for Task 1)

```
src/
├── lib.rs                        # Library root: pub mod declarations + pub use re-exports
├── main.rs                       # CLI entry point (anyhow, single #[tokio::main])
├── cli.rs                        # clap derive-style argument definitions
├── app_config.rs                 # App configuration (replaces planned src/config/)
├── db.rs                         # Database connection + migrations (replaces planned src/storage/)
├── user_agent.rs                 # User-Agent string construction
│
├── app/                          # Application orchestration layer (NEW — not in original arch)
│   ├── mod.rs
│   ├── command_dispatcher.rs
│   ├── config_manager.rs
│   ├── config_runtime.rs
│   ├── context.rs
│   ├── download_orchestrator.rs
│   ├── exit_handler.rs
│   ├── input_processor.rs
│   ├── progress_manager.rs
│   ├── queue_manager.rs
│   ├── resolution_orchestrator.rs
│   ├── runtime.rs
│   ├── terminal.rs
│   └── validation.rs
│
├── commands/                     # CLI command implementations (NEW — not in original arch)
│   ├── mod.rs
│   ├── auth.rs
│   ├── config.rs
│   ├── dry_run.rs
│   ├── log.rs
│   └── search.rs
│
├── auth/                         # Authentication & cookie management
│   ├── mod.rs
│   ├── capture.rs
│   ├── cookies.rs
│   ├── runtime_cookies.rs
│   └── storage.rs                # KeyStorage enum lives here (not keychain.rs)
│
├── download/                     # Download engine
│   ├── mod.rs
│   ├── client.rs
│   ├── constants.rs
│   ├── error.rs                  # Module-local error type
│   ├── filename.rs
│   ├── rate_limiter.rs
│   ├── retry.rs
│   ├── robots.rs
│   └── engine/                   # Engine internals
│       ├── mod.rs  (engine.rs)
│       ├── error_mapping.rs
│       ├── persistence.rs
│       └── task.rs
│
├── failure/                      # Failure taxonomy (NEW — not in original arch)
│   └── mod.rs
│
├── output/                       # CLI output formatting
│   └── mod.rs
│
├── parser/                       # Input parsing
│   ├── mod.rs
│   ├── bibliography.rs
│   ├── bibtex.rs
│   ├── doi.rs
│   ├── error.rs                  # Module-local error type
│   ├── input.rs
│   ├── reference.rs
│   └── url.rs
│
├── project/                      # Project directory management (was planned in storage/)
│   └── mod.rs
│
├── queue/                        # Download queue + history
│   ├── mod.rs
│   ├── error.rs                  # Module-local error type
│   ├── history.rs
│   ├── item.rs
│   └── repository.rs
│
├── resolver/                     # Resolver trait + registry + site resolvers (flat, no sites/ subdir)
│   ├── mod.rs
│   ├── arxiv.rs
│   ├── crossref.rs               # Crossref DOI resolution (was planned as doi.rs)
│   ├── direct.rs
│   ├── error.rs                  # Module-local error type
│   ├── http_client.rs
│   ├── ieee.rs
│   ├── pubmed.rs
│   ├── registry.rs               # build_default_resolver_registry() lives here
│   ├── sciencedirect.rs
│   ├── springer.rs
│   └── utils.rs
│
├── search/                       # Past-download search (NEW — not in original arch)
│   └── mod.rs
│
├── sidecar/                      # JSON-LD sidecar generation (NEW — not in original arch)
│   └── mod.rs
│
├── test_support/                 # Test support utilities (replaces planned tests/common/)
│   ├── mod.rs
│   └── socket_guard.rs
│
└── topics/                       # Topic extraction + normalization (NEW — not in original arch)
    ├── mod.rs
    ├── extractor.rs
    └── normalizer.rs
```

**Planned modules that do NOT exist:**
| Planned | Why Absent |
|---------|-----------|
| `src/error.rs` (unified error) | Each module has its own `error.rs`; unified re-export not needed |
| `src/config/` | Replaced by `src/app_config.rs` |
| `src/storage/` | Replaced by `src/db.rs` + domain-local storage logic |
| `src/util/clock.rs` | Clock trait not centralized; timing handled differently per module |
| `src/resolver/context.rs` | Resolution context folded into orchestrator |
| `src/resolver/doi.rs` | Renamed `crossref.rs` (more accurate to its responsibility) |
| `src/resolver/sites/` subdir | Resolvers are flat in `src/resolver/` — simpler and sufficient |
| `src/auth/keychain.rs` | KeyStorage enum lives in `src/auth/storage.rs` |

---

### As-Built `tests/` Layout

```
tests/
├── README.md
├── auth_integration.rs
├── cli_e2e.rs
├── critical.rs                      # Entry point for critical/ suite
├── critical/                        # Adversarial failure-mode tests
│   ├── auth_bypass.rs
│   ├── concurrent_load.rs
│   ├── cookie_poisoning.rs
│   ├── corrupted_state.rs
│   ├── crash_recovery.rs
│   ├── credential_leakage.rs
│   ├── data_corruption.rs
│   ├── disk_space_failures.rs
│   ├── encryption_failures.rs
│   ├── file_descriptor_exhaustion.rs
│   ├── intermittent_connectivity.rs
│   ├── interrupted_operations.rs
│   ├── memory_leaks.rs
│   ├── network_failures.rs
│   ├── persistence_recovery.rs
│   ├── power_failure_simulation.rs
│   ├── race_conditions.rs
│   ├── rate_limit_handling.rs
│   ├── timeout_edge_cases.rs
│   └── transaction_failures.rs
├── download_engine_integration.rs
├── download_integration.rs
├── exit_code_partial_e2e.rs
├── integration_matrix.rs
├── nonfunctional_regression_gates.rs
├── optimization_refactor_commands.rs
├── parser_integration.rs
├── queue_integration.rs
├── resolver_integration.rs
├── support/                         # Shared test utilities (was planned as tests/common/)
│   ├── mod.rs
│   ├── critical_utils.rs
│   └── socket_guard.rs
```

**Note:** `tests/common/` was planned; actual is `tests/support/`. Also `src/test_support/` holds in-lib test utilities.

---

### As-Built CI Workflows

```
.github/workflows/
├── phase-rollout-gates.yml    # Build, test, clippy, fmt, audit (was planned as ci.yml)
├── coverage.yml               # cargo llvm-cov coverage reporting
└── stress-sidecar-flaky.yml   # Sidecar stress test
```

No `release.yml` yet — binary release workflow not yet implemented.

---

### Cookie Storage Correction (for Task 3)

**Architecture specified (incorrect):**
> Cookie data stored in a `cookies` SQLite table with encrypted values

**Actual implementation:**
- Cookie data stored in an encrypted file using **XChaCha20Poly1305**
- Master encryption key stored in OS keychain (macOS Keychain / Windows Credential Manager)
- `KeyStorage` enum in `src/auth/storage.rs`:
  ```rust
  enum KeyStorage {
      OsKeychain,          // Production
      InMemory(String),    // Testing
      Environment,         // CI (DOWNLOADER_MASTER_KEY env var)
  }
  ```
- This is a **better** security decision — cookie data never transits through SQLite unencrypted
- The architectural decision record (WHY use keychain for key, file for data) remains valid and should be updated to say "as implemented" not "as planned"

---

### What Must NOT Change (AC #7)

Preserve these decision records verbatim (they are correct and authoritative):

| Section | Decision | Why Preserve |
|---------|---------|-------------|
| Core Architecture | Lib/bin split | Held across 8 epics — canonical |
| Resolver Architecture | Resolver trait + registry | Held and extended with `build_default_resolver_registry()` |
| Data Architecture | SQLite + WAL mode | Correct; WAL + DashMap confirmed NFR gate |
| Auth Architecture | KeyStorage enum OsKeychain/InMemory/Environment | Correct |
| Error Handling | thiserror in lib, anyhow in binary | Correct |
| Concurrency | Tokio + Semaphore | Correct |

---

### References

- Retrospective action item: [Source: `_bmad-output/implementation-artifacts/project-retro-2026-02-23.md` §Architecture Action Items]
- Original architecture (to be amended): [Source: `_bmad-output/planning-artifacts/architecture.md`]
- Actual module layout: `find src -name "*.rs" | sort` (captured 2026-02-25)
- `lib.rs` public API: [Source: `src/lib.rs:26-68`]

## Party Mode Audit (AI)

**Date:** 2026-02-25
**Outcome:** pass_with_actions
**Counts:** 1 High · 5 Medium · 3 Low

### Findings

| Sev | Perspective | Finding |
|-----|-------------|---------|
| High | Architect | Removed modules (`src/storage/`, `src/util/`, `src/error.rs`, `src/config/`) appear in multiple sections beyond the directory tree: Dependency Direction diagram, Quick Reference Card (~line 581), Architecture Completeness Checklist (~line 1404). Tasks 1–2 alone leave these stale. |
| Medium | Architect | Task 2 says "update rows that changed" but provides no complete updated dependency map. Dev agent has no spec to work from. |
| Medium | PM | AC #7 ("preserve ADRs verbatim") is unoperationalized — no section headings or line anchors given; dev agent could reword an ADR while editing surrounding prose. |
| Medium | PM | No doc-correctness validation step: nothing in story confirms updated doc is internally consistent after edits. |
| Medium | QA | `cargo test` before/after only proves no code broke, not that the doc is correct. A filesystem-diff gate against the written tree is missing. |
| Medium | Developer | Task 5 (verify ADRs) is sequenced after edits. If an ADR was touched, work must be redone. Must snapshot ADRs before any edits. |
| Medium | Developer | No section headings / line anchors for target blocks in architecture.md — dev agent must grep blindly across a 1,500-line file. |
| Low | PM | AC #8 ("no code changes") has no enforcement suggestion (e.g. check `git diff --name-only`). |
| Low | QA | No escalation path if additional undocumented divergences are discovered during the edit. |
| Low | Developer | Directory structure lives in a large fenced code block; story does not identify block boundaries, risking accidental prose inclusion. |

*(Follow-up tasks moved to Tasks / Subtasks § Review Follow-ups (AI) — all resolved)*

---

## Dev Agent Record

### Agent Model Used

claude-sonnet-4-6

### Debug Log References

N/A — documentation-only story

### Completion Notes List

**Dev-story pass:**
- Replaced full planned `src/` tree (18 modules/files) with as-built layout (80 `.rs` files across 17 modules)
- Added `src/bin/` targets discovered during correctness gate (not in Dev Notes — gap found and fixed)
- Fixed `engine.rs` + `engine/` Rust 2018 module style notation
- Removed `CREATE TABLE cookies` from SQLite schema; added accurate explanatory comment
- Updated Error Module Structure: removed unified `src/error.rs` concept; documented module-local pattern
- Updated Module Ownership Mapping: removed 4 absent modules, added 9 as-built modules with correct dependency edges
- Updated Requirements Coverage and Test Coverage Guidelines tables
- Updated Quick Start key files and Development Sequence
- Updated Tauri section: added as-built validation note; GUI goal preserved intact
- All 6 ADR sections verified byte-for-byte unchanged
- `git diff --name-only`: only `architecture.md` and `sprint-status.yaml` — zero source code touched
- AC #4 minimal update per user direction: Tauri GUI remains planned next step

**Code review pass (7 additional fixes):**
- Removed duplicate stale `.github/` block (`ci.yml`, `release.yml`) from top of directory tree — tree had `.github/` appearing twice
- Replaced stale "Library Boundary" code block (used planned `config`/`storage`/`util`/`error` modules) with actual `lib.rs` public API (as-built)
- Updated "Test Fixtures" example: `// tests/common/mod.rs` → `// tests/support/mod.rs`; `use crate::common::` → `use crate::support::`
- Updated "Test Utilities" section: `// tests/common/mod.rs` → `// tests/support/mod.rs`; `downloader_core::storage::Database` → `downloader::db::Database`
- Updated "CI Pipeline" header: `ci.yml` → `phase-rollout-gates.yml`
- Added `src/bin/` to early summary tree (lines ~193–216) for consistency with full tree
- Added `sprint-status.yaml` to File List (was modified by story tracking but absent from record)

### File List

- `_bmad-output/planning-artifacts/architecture.md` (modified)
- `_bmad-output/implementation-artifacts/sprint-status.yaml` (modified — story tracking)
