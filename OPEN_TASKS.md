# Open Tasks

Decisions and deferred items as of Epic 11 close (2026-03-08).
All other backlog items from Epics 1–11 have been resolved.

---

## Decisions

Items formally decided and closed — no further action required.

- **#1 YouTube streaming transcript** — Won't do. The transcript fetch is a single small
  XML GET to the public timedtext API. Switching to streaming/event-driven adds complexity
  for zero latency benefit on a response that is always small.

- **#2 Regex duplication audit** — Audited during Epic 11 planning. No duplication found
  beyond the 4 centralized patterns already in `resolver/utils.rs`. No action required.

- **#3 Content-Type mapping gaps** — Already done. `application/zip` → `.zip` and
  `text/plain` → `.txt` mappings exist in `download/filename.rs` lines 114 and 122.

- **#4 Tauri IPC blocking** — Acceptable in current form. Commands are `async fn`,
  downloads spawn tasks via `tokio::spawn`, and no blocking calls exist in practice.
  `AppState` concurrency contract is documented in `commands.rs`.

- **#9 Story-closure checklist** — Added 3-bullet checklist to `project-context.md`
  under `## Story Closure Checklist`. Process item; done.

- **#26 `--check-robots` flag redundancy** — Keep the flag. It is opt-in and NOT
  automatic enforcement. The download client does not enforce robots.txt by default;
  the flag explicitly enables the check. Not redundant.

- **#27 History-search scaling** — No action. SQLite pagination (`PROJECT_LOG_QUERY_PAGE_SIZE`)
  handles current scale. Revisit if query latency is ever measured as a problem.

- **#34 Epic 10 retro scheduling** — Already done. Retrospective outcomes captured in
  `_bmad-output/implementation-artifacts/project-retro-2026-02-23.md`.

- **#35–#43 Planning/governance prep** — Resolved by creating Epic 11 itself. All
  backlog tracking, sprint status, and planning artifacts are current.

---

## Deferred to Epic 12

Items that require new infrastructure or are genuine new features — not backlog cleanup.

- **#13 Throughput benchmark (Criterion)** — New infrastructure. Add a Criterion
  benchmark (`benches/download_throughput.rs`) gating at ≥ 5 MB/s for a 10 MB
  in-memory payload. Integrate into CI as an informational step.

- **#14 Windows CI runner** — Infrastructure requiring cross-platform setup.
  **Risk:** Windows compatibility is untested; any path-related changes (session label
  format now uses `h`/`m`/`s` markers instead of colons — already Windows-safe, but
  untested end-to-end) should be manually validated on Windows until CI is added.

- **#15 CI burn-in tracking + badge** — Waived in NFR gate (no CI run history yet).
  Needs CI history first before a meaningful burn-in badge can be generated.

- **#17 cargo geiger** — Zero unsafe blocks verified manually. Low value as a CI gate
  unless `unsafe` is ever introduced. Revisit if dependencies with unsafe are added.

- **#18 RSS/heap profiling** — Performance infrastructure, not backlog cleanup.
  Add `memory_stats` or `jemalloc` profiling harness in a dedicated performance epic.

- **#23 YouTube chapter extraction** — New feature. Chapter data is often in the video
  description rather than a separate API endpoint. May be a quick win in Epic 12 once
  the description field is accessible from the oEmbed response.
