# Open Tasks

A single place to track open work: bugs, improvements, refactoring, and follow-up items. Add items here when they are out of scope for the current effort or discovered during review.

---

## Refactoring & code quality

- **resolver/youtube:** Replace polling-based transcript fetch with a streaming/event-driven approach to reduce latency on long videos.
- **resolver/utils:** Audit remaining regex duplication across resolvers now that 4 patterns are centralised in `utils.rs`; consider a shared citation-parse function.
- **download/client:** Extend Content-Type → extension mapping to cover `application/zip`, `text/plain`, and common data formats currently falling through to `.bin`.
- **Epic 10 retro:** Review Tauri inter-process communication patterns — current `tauri::command` functions block the async executor; migrate long-running calls to a side-channel with progress streaming.

---

## Bugs & fixes

- **parser/url:** Backslash-strip heuristic may over-eagerly strip intentional `\` in rare local file paths passed as URLs; add a guard that only strips when the string looks like an http/https URL.
- **resolver/crossref:** `extract_year_from_date_parts()` silently returns `None` for malformed date arrays instead of logging a warning; add a `tracing::warn!` so failures are visible in logs.

---

## Features & enhancements

- **Throughput benchmark:** No automated throughput regression guard exists. Add a Criterion benchmark (`benches/download_throughput.rs`) gating at ≥ 5 MB/s for a 10 MB in-memory payload; integrate into CI as an informational (non-blocking) step initially.
- **resolver/youtube:** Support YouTube Shorts URLs (`/shorts/<id>`); currently only standard watch URLs are resolved.
- **resolver/youtube:** Add chapter/timestamp metadata extraction from description when `chapters` are present in oEmbed response.
- **UI — CompletionSummary:** "Expand all" / "Collapse all" toggle for the expandable error list when there are > 5 failures.
- **UI — ProjectSelector:** Keyboard navigation (arrow keys + Enter) through the project list for accessibility.
- **Carry-forward:** Evaluate whether `downloader-cli` still needs the `--robots-txt` flag now that the download client enforces robots.txt automatically; remove the flag if redundant.

---

## Documentation & process

- **CHANGELOG:** Add an entry summarising the post-epic-10 polish batch (YouTube resolver, resolver deduplication, URL backslash fix, Tauri UI improvements).
- **README:** Document the YouTube resolver capability and its transcript-fetching behaviour (language selection, fallback).
- **Epic 10 retro action items:** Schedule a retrospective session; capture outcomes in `_bmad-output/` before next sprint planning.

---

## How to use

- **Add:** One bullet per task; optional `**Label:**` for area (e.g. module name).
- **Complete:** Move the bullet to a "Done" subsection at the bottom of the relevant section, or delete it and note in CHANGELOG if notable.
- **Scope:** Keep descriptions short; link to issues, plans, or code when helpful.
