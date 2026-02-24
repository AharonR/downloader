# Final Project Retrospective: Downloader CLI (Epics 1–8)
<!-- Supersedes project-retro-2026-02-18.md — includes NFR resolution arc and architecture/coding-level analysis -->

**Date:** 2026-02-23
**Project:** Downloader
**Scope:** Full implementation cycle — all 8 epics + NFR gate resolution arc
**Facilitated by:** Bob (Scrum Master)

## Participants

- Bob (Scrum Master)
- John (Product Manager)
- Amelia (Developer)
- Murat (Test Architect)
- Winston (Architect)
- fierce (Project Lead)

---

## Delivery Scorecard

| Metric | Result |
|--------|--------|
| Epics completed | 8 / 8 (100%) |
| Stories completed | 47 / 47 (100%) |
| Epic retrospectives | 8 / 8 (100%) |
| Unit tests passing | 564 / 564 |
| `cargo clippy -- -D warnings` | Exit 0 |
| `cargo audit` | Exit 0 (2 advisories accepted with rationale) |
| NFR gate | CONCERNS ⚠️ (non-blocking — all blockers resolved 2026-02-23) |

### NFR Gate Resolution Arc
The NFR assessment (2026-02-22) initially returned **FAIL** with two blockers:
- **Maintainability:** 11 clippy errors + 3 failing unit tests
- **Security:** `cargo audit` absent from CI

Both blockers were resolved same-day (2026-02-23):
- All 11 clippy errors fixed; `cargo clippy -- -D warnings` exits 0
- All 3 unit test failures fixed (crossref mailto header injection validation ×2; sidecar tracing callsite race condition ×1)
- `cargo audit` added to CI with 2 accepted advisories documented in `.cargo/audit.toml`

Gate moved from FAIL ❌ → **CONCERNS ⚠️ (non-blocking)**. Remaining CONCERNS are honest, not blocking:
- No download throughput benchmark
- No `cargo llvm-cov` coverage reporting
- Windows CI not validated

---

## Outcome by Epic

| Epic | Goal | Outcome |
|------|------|---------|
| 1 | Core CLI — URL download, queue, concurrency, retry, rate limiting | Complete |
| 2 | Smart input — DOI/reference/BibTeX resolution | Complete |
| 3 | Reliable batch — visibility, interrupt handling, resumability | Complete |
| 4 | Authenticated downloads — cookie capture, secure storage, ScienceDirect resolver | Complete |
| 5 | Organized output — project folders, metadata naming, index generation | Complete |
| 6 | Download history — SQLite logging, failure taxonomy, `downloader log` query | Complete |
| 7 | Professional CLI — dry-run, config file, What/Why/Fix errors, exit codes, terminal compat | Complete |
| 8 | Polish — topics, JSON-LD sidecars, confidence tracking, search, resolver expansion | Complete |

---

## Architecture Assessment

### What Held as Designed

The three core architectural bets performed across all 8 epics without requiring rework:

1. **Lib/bin split** — Core logic in `lib.rs`, CLI in `main.rs`. The clean boundary enabled `#[deny(clippy::expect_used)]` as a hard lib-level rule by Epic 7. No CLI wiring tangled into core logic.

2. **Resolver trait + registry** — `can_handle()` / `resolve()` contract held and scaled. When Epic 8 added arXiv, PubMed, IEEE, and Springer, the pattern was: implement trait, register in `build_default_resolver_registry`, done. No CLI changes, no routing changes. The centralization in Epic 8 removed a whole class of future drift risk.

3. **SQLite + WAL mode** — Queue persistence, download log, and metadata all in SQLite as designed. WAL mode + DashMap eliminated SQLITE_BUSY errors under concurrent write load (confirmed by NFR gate tests).

4. **Auth abstraction (KeyStorage enum)** — keychain / env / in-memory separation was clean. XChaCha20Poly1305 + keychain master key delivered correct security properties. The P0 security test suite passes: cookie domain isolation, credential non-leakage, encryption roundtrip.

5. **Error handling boundary** — `thiserror` in lib, `anyhow` in binary, module-scoped error enums unified via `From` impls. Held throughout.

### Where the Implementation Improved on the Design

- **Cookie storage**: The architecture specified a `cookies` SQLite table. The implementation chose an encrypted file (XChaCha20Poly1305) with keychain-protected master key. This is a *better* security decision — cookie data never transits unencrypted through SQLite.
- **Resolver centralization**: `build_default_resolver_registry` emerged in Epic 8 as the canonical registration point. The architecture anticipated the registry pattern but not this specific centralization.

### Where the Implementation Diverged (Healthily)

- **Module granularity**: The architecture drew 6 major modules. The actual `src/` layout is richer: `src/app/`, `src/commands/`, `src/failure/`, `src/search/`, `src/sidecar/`, `src/topics/`, `src/user_agent.rs` all emerged organically. The directory structure has evolved beyond the plan; the architecture doc's *decision record* (why WAL, why XChaCha20, why thiserror-in-lib) remains valid but the module map needs an as-built update.
- **Tauri deferred indefinitely**: The architecture invested time in a Tauri migration path. For v1, the product is CLI-only. The lib/bin split means the transition remains low-cost whenever the time comes — but v1 is CLI.
- **`src/util/` Clock trait**: The architecture specified a Clock abstraction in `src/util/`. The implementation distributed timing concerns differently. Not a quality gap, just structural drift.

### Architecture Action Items

1. **Update architecture doc with as-built module map** — document actual `src/` layout as the canonical reference, note Tauri migration as deferred, correct the cookie storage section. This is an amendment to the decision record, not a rewrite.
2. **Reconsider simple UI layer** — the lib/bin foundation is UI-ready. The lib/bin split, clean public API in `lib.rs`, and stable behavior contracts make this the right moment to evaluate. Tauri workspace extraction remains the low-cost path. UI testing strategy must be decided up front.

---

## Coding Level Assessment

The codebase produced in this program reflects professional-grade Rust:

**Strengths:**
- **Idiomatic Rust throughout**: `Result<T>` everywhere, no panics in lib code, `thiserror`/`anyhow` boundary maintained, `#[deny(clippy::expect_used)]` enforced.
- **Security-conscious implementation**: XChaCha20Poly1305 at rest, cookie domain isolation, credential non-leakage guarantees, HTTP header injection prevention in crossref mailto validation — these were designed with the attack surface in mind, not retrofitted.
- **Adversarial test suite**: `tests/critical/` has auth bypass, credential leakage, encryption failure, race condition, and network failure tests. These test the failure modes, not just the happy path.
- **Deterministic behavior contracts**: CLI exit codes, verbosity precedence, dry-run guarantees, terminal compatibility — all specified, implemented, and regression-tested. Scriptability and operator trust were first-class requirements.
- **Parallel test debugging**: The sidecar tracing callsite race condition required understanding how `DefaultCallsite` lazily initializes interest caches and why `rebuild_interest_cache()` inside a `with_default` closure differs from the call `with_default` makes on entry. This is advanced Rust parallel testing knowledge (documented in `memory/debugging.md`).

**Growth areas:**
- The crossref mailto regression (two tests that caught real header injection regressions) shows that fast-cycle implementation can outpace regression guard updates. The fix existed; the tests surfaced the gap.
- Story closure documentation (checklists, evidence sections) lagged under fast review-fix cycles in later epics. Code quality stayed high; governance hygiene dipped.

---

## What Worked Across the Program

1. **Story sequencing discipline**: Foundation before UX before polish, every epic. Never built a feature on unstable ground.
2. **Review-to-fix conversion**: High findings fixed before epic close, without exception. Adversarial review generated real findings.
3. **Shared infrastructure as a multiplier**: Resolver registry, terminal helpers, HTTP client policy — each centralization reduced future drift risk for all subsequent stories.
4. **NFR gate is proven**: Caught real blockers (header injection regression, tracing race condition, missing audit). Team fixed them same-day. Gate → fix → re-assess is the right loop.
5. **Behavioral test contracts**: By Epic 7, tests were asserting precedence rules, exit code semantics, and dry-run side-effect guarantees — not just "does it return something."

---

## Recurring Friction Patterns

1. **Carry-forward items without resolution loop**: The unix-* session label issue appeared in Epic 5, 6, 7, 8 retrospectives and the 2026-02-18 project retro without resolution. This is the canonical example of a low-priority item that survives indefinitely without an explicit decision. Needs: implement, accept-and-close, or delete from backlog.

2. **Sandbox/network constraint limiting integration confidence**: `DOWNLOADER_REQUIRE_SOCKET_TESTS=1` silently skips wiremock-based socket tests when not set. The flag should be standard in dev environment setup. This is the program's most persistent quality confidence gap.

3. **Story closure hygiene under fast cycles**: Evidence sections and checklists lagged after iterative review-fix cycles in Epics 7 and 8. Code was correct; the documentation of *why* was behind. A mandatory closure-sync checklist before marking `done` would address this.

---

## Main Takeaways

1. **The architecture held where it mattered most.**
   Lib/bin split, resolver registry, SQLite+WAL, auth abstraction — all performed as designed across 8 epics. The module structure evolved beyond the plan (healthily), but the foundational decisions were right. As-built amendment needed; the decision record stands.

2. **Review discipline was the program's highest-leverage process.**
   Every epic, High findings were fixed before close. This is the one process to protect unconditionally in future cycles. The adversarial format — party-mode audit plus code review — generated real findings, not ceremony.

3. **Deterministic behavior contracts are core product quality, not polish.**
   Exit codes, verbosity precedence, dry-run guarantees, terminal compatibility — these make a CLI trustworthy in scripts and pipelines. Every future feature should be specified with this lens from the story's acceptance criteria, not discovered during review.

4. **The NFR gate works and should stay.**
   It caught real issues. The team fixed them. Gate → fix → re-assess is the right loop. `DOWNLOADER_REQUIRE_SOCKET_TESTS=1` should be standard in dev workflow, not exceptional.

5. **Carry-forward items need explicit resolution this cycle.**
   Not necessarily implementation — explicit *decision*. Implement, accept-and-close, or delete. An open carry-forward that isn't tracked is friction without accountability.

6. **The codebase is UI-ready, and now is the right time to reconsider a simple UI.**
   The lib/bin boundary is clean, behavior contracts are stable and tested, and the public API surface is mature. Adding a UI layer now carries far less risk than it would have at Epic 2 or 4. Decide on UI scope before the next planning cycle.

---

## Outstanding Carry-Forward Actions

Actions carried from program epic retrospectives requiring explicit resolution:

| Item | Origin | Resolution Required |
|------|--------|-------------------|
| Replace `unix-*` session labels with human-readable timestamps | Epic 5 retro | Implement or accept-and-close |
| Add README examples for mixed stdin + positional input and no-input quick-start | Epic 7 retro | Implement or accept-and-close |
| Define large-history search scaling policy (`--exhaustive` or paging) | Epic 8 retro | Policy decision + story or accept-and-close |
| Finalize parse-confidence storage contract (normalization/validation) | Epic 8 retro | Policy decision |
| Establish non-sandbox integration test path for network/wiremock suites | Epic 6 retro | `DOWNLOADER_REQUIRE_SOCKET_TESTS=1` as dev standard |
| Add mandatory story-closure checklist | Epic 8 retro | Add to SM story-creation workflow |

---

## Next Sprint Priorities (from NFR Assessment)

| Priority | Item | Owner |
|----------|------|-------|
| High | Add `cargo llvm-cov` coverage reporting and establish baseline | Dev |
| High | Add download throughput benchmark (wiremock + criterion or NFR gate timing test) | Dev |
| Medium | Plan architecture doc as-built amendment | Architect |
| Medium | Evaluate simple UI scope — Tauri vs. web vs. deferred | fierce + Architect |
| Medium | Resolve carry-forward items (decisions, not necessarily implementation) | fierce + SM |
| Low | Add Windows CI runner | Dev |

---

## Final Readiness Assessment

| Dimension | Status |
|-----------|--------|
| Product scope | Complete for all 8 written epics |
| Technical health | Strong — no hard blockers, known carry-forward items explicit |
| Quality | Strong on targeted suites; full integration confidence improves once socket tests are standard and coverage is baselined |
| Architecture | Sound as decision record; needs as-built module map update |
| Process | Mature — closure hygiene and environment validation gaps now clearly identified |
| UI readiness | Foundation proven; simple UI evaluation appropriate for next planning cycle |

---

## Retrospective Outcome

**Status:** Program complete and in healthy state for next cycle.

The team should treat the architecture as-built amendment, carry-forward resolution decisions, and UI scope evaluation as entry criteria for the next planning cycle. Quality gains from Epics 2–8 are preserved and the foundation is solid.

**Next workflow:** `/bmad:bmm:workflows:sprint-status` to plan next sprint priorities.
