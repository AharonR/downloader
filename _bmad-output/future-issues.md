# Future Issues — Downloader

Items deferred for post-Epic 10/11 attention. Not blockers for current work.

---

## Carried Forward from Epic 8 Retrospective (2026-02-18)

These action items were committed in the Epic 8 retro but were not addressed during Epic 9
(Epic 9 was scoped to maintenance/documentation and UI decision only). Deferred to post-Epic 10 or 11.

| # | Item | Original Owner | Notes |
|---|------|---------------|-------|
| 1 | Add explicit README examples for mixed stdin + positional input and no-input quick-start behavior | Alice, Charlie | Still missing from README |
| 2 | Propose and document history-search scaling policy (`--exhaustive` or paging) with test implications | Winston, Charlie | Search dataset size still unbounded |
| 3 | Define and enforce parse-confidence storage contract (validation/normalization and compatibility expectations) | Winston, Charlie | Confidence values stored but contract informal |
| 4 | Replace `unix-*` session headers with human-readable labels in generated artifacts | Charlie | `unix-*` labels still present in output paths |
| 5 | Establish a non-sandbox validation path for network/wiremock integration suites and include it in quality gates | Dana, Winston | Wiremock-based tests still partially skipped in sandbox; relevant for Epic 10 CI pipeline setup |

**Decision:** Address after Epic 10 or 11. Item 5 (non-sandbox validation) has the highest relevance to Epic 10 CI work — flag for review when setting up Tauri CI pipeline.

---
