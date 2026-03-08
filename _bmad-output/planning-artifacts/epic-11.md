# Epic 11: Backlog Cleanup

**User Value:** "The tool is more reliable and polished — rough edges filed down, docs complete"

## Scope

A focused cleanup sprint addressing open backlog items accumulated through Epics 1–10.
No new features beyond the backlog items; every story resolves tracked issues.

- Bug fixes: URL backslash guard, crossref date warning, session label format
- Code quality: error message convention doc, parse_confidence doc comments
- CI hardening: cargo deny, critical test named step, socket test env var
- UI enhancements: YouTube Shorts support, CompletionSummary expand/collapse, ProjectSelector keyboard nav
- Documentation: CHANGELOG, README, architecture doc, smoke test checklist
- Backlog closure: OPEN_TASKS final state, sprint status updated

**Exit Criteria:** All 35 backlog items resolved (closed or consciously deferred). Verification gates pass.

**Dependencies:** All Epics 1–10 complete.

---

## Architecture Constraints

All constraints from existing epics carry forward.

---

## Stories

### Story 11-1: Bug Fixes and Micro-Improvements

**As a developer,**
I want known bugs and minor issues addressed,
**so that** the codebase is clean and reliable.

**Items:** #28 (URL backslash guard), #29 (crossref date warn), #5 (session label format)

**Exit Criteria:**
- `cargo clippy --workspace -- -D warnings` exits 0
- `cargo test --workspace --lib` passes
- `cargo test --test critical -p downloader-core` passes

---

### Story 11-2: Code Quality and Convention Documentation

**As a developer,**
I want coding conventions documented and enforced,
**so that** all contributors follow consistent patterns.

**Items:** #7 (error message convention), #6 (parse_confidence doc comments)

**Exit Criteria:**
- Error message convention section added to project-context.md
- `parse_confidence` field has doc comments with valid range and usage

---

### Story 11-3: CI Hardening

**As a developer,**
I want the CI pipeline to be robust and comprehensive,
**so that** regressions are caught automatically.

**Items:** #21 (critical tests named step), #19 (socket test env var), #16 (cargo deny), #10/#11 (docs)

**Exit Criteria:**
- `cargo deny check` passes with deny.toml
- `DOWNLOADER_REQUIRE_SOCKET_TESTS=1` set in all CI test steps
- Critical tests run as a named CI step

---

### Story 11-4: YouTube Shorts and UI Enhancements

**As a user,**
I want YouTube Shorts URLs to resolve and the desktop app UI to be more polished,
**so that** more content types are supported and errors are easier to review.

**Items:** #22 (YouTube Shorts), #24 (CompletionSummary expand/collapse), #25 (ProjectSelector keyboard nav)

**Exit Criteria:**
- YouTube Shorts URLs resolve via `extract_video_id`
- CompletionSummary shows expand/collapse all toggle for >5 failures
- ProjectSelector keyboard nav documented

---

### Story 11-5: Documentation Batch

**As a user and developer,**
I want comprehensive, up-to-date documentation,
**so that** the project is easy to understand and use.

**Items:** #30 (CHANGELOG), #31/#32/#33 (README), #8 (architecture doc), #12/#20 (smoke checklist)

**Exit Criteria:**
- CHANGELOG entry for Epic 11
- README updated with YouTube resolver, mixed stdin examples, config alignment note
- Architecture doc created
- Smoke test checklist created

---

### Story 11-6: Backlog Closure

**As a project maintainer,**
I want the backlog formally closed,
**so that** the next sprint starts with a clean slate.

**Items:** #9 (story closure checklist), all remaining backlog items resolved

**Exit Criteria:**
- OPEN_TASKS.md rewritten with Decisions and Deferred sections
- Sprint status updated with all 11-x stories as done

---

## Sources

- `OPEN_TASKS.md` — backlog items
- `_bmad-output/implementation-artifacts/sprint-status.yaml` — tracking
- All prior epic files for context
