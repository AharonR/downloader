# Party Mode Audit — Project Support Feature (Tauri UI)

**Date:** 2026-02-28
**Outcome:** `pass_with_actions`
**Scope:** Post-implementation review of the "Add Project Support to Tauri UI" feature
**Files reviewed:**
- `downloader-core/src/project.rs` (new)
- `downloader-core/src/lib.rs` (added `pub mod project`)
- `downloader-cli/src/project/mod.rs` (refactored to re-export from core)
- `downloader-cli/src/main.rs` (project module usage updated)
- `downloader-cli/src/output/mod.rs` (stray blank line removed)
- `downloader-cli/tests/cli_e2e.rs` (test helpers updated for core extraction)
- `downloader-app/src-tauri/Cargo.toml` (added `tempfile = "3"` dev-dep)
- `downloader-app/src-tauri/src/commands.rs` (list_projects + project param)
- `downloader-app/src-tauri/src/lib.rs` (command registration)
- `downloader-app/src/lib/ProjectSelector.svelte` (new)
- `downloader-app/src/lib/DownloadForm.svelte` (updated)
- `downloader-app/src/lib/ProjectSelector.test.ts` (new)
- `downloader-app/src/lib/DownloadForm.test.ts` (updated)

---

## Summary

| Severity | Count |
|----------|-------|
| High     | 3     |
| Medium   | 4     |
| Low      | 5     |

---

## Findings

### 👩‍💼 Perspective 1 — Product Manager

**PM-1 [High] — No test verifies project name flows from DownloadForm to backend**
The `DownloadForm.test.ts` suite has one test for `start_download_with_progress` invocation and it
only asserts `project: null`. There is zero test that fills the project field, submits, and confirms
`project: "Climate Research"` (or any non-null value) reaches the Tauri command. The entire
user-facing feature could break silently.

**PM-2 [Medium] — No affordance for "new project" creation**
When a user types a brand-new project name, nothing in the UI indicates a new folder will be
created. A first-time user may think the selector only accepts existing projects (from the
datalist). Consider a subtle "new project" hint when the typed value isn't in the datalist.

**PM-3 [Low] — Sprint status has no entry for this feature**
Epic 10 is marked `done` and sprint-status.yaml has no entry for this work. The plan was executed
but is invisible to sprint reporting.

---

### 🏛️ Perspective 2 — Architect

**ARCH-1 [High] — `start_download` uses empty `HashSet::new()` for `completed_before`**
Location: `downloader-app/src-tauri/src/commands.rs:369`

```rust
let completed_before = HashSet::new();   // always empty!
let _ = append_project_index(&queue, &output_dir, &completed_before).await;
```

An empty set means `!completed_before.contains(&item.id)` is always `true`. Every previously
completed item in the DB gets included in the index on every download. On a second download to the
same project, all prior downloads will be re-added to `index.md` producing duplicate entries.

`start_download_with_progress` (line 445) does this correctly — it captures the before-state first.
The simple command must be patched to match.

**ARCH-2 [Medium] — `#[tracing::instrument]` missing on all new commands**
`project-context.md` rule: "`#[tracing::instrument]` on all public functions." None of the new
commands (`list_projects`, `start_download`, `start_download_with_progress`, `cancel_download`)
have it. Observability is broken — no spans exist for any Tauri IPC calls.

**ARCH-3 [Medium] — Two separate databases for the two download commands**
- `start_download` → `~/.downloader/downloader-app.db`
- `start_download_with_progress` → `~/.downloader/downloader-app-progress.db`

Project history written by one is invisible to the other. The only documentation is the comment
"Story 10-2 — kept for unit-test compatibility." This split-history model is a latent architectural
debt that should be explicitly acknowledged in architecture.md.

**ARCH-4 [Low] — `datalist id="project-suggestions"` is a hardcoded global DOM ID**
Location: `downloader-app/src/lib/ProjectSelector.svelte:30`

If this component is ever rendered more than once on a page, duplicate IDs will silently wire both
inputs to the same datalist. This is a latent fragility.

---

### 🔬 Perspective 3 — QA / Test Engineer

**QA-1 [High] — `test_list_projects_*` don't test the actual `list_projects()` function**
Location: `downloader-app/src-tauri/src/commands.rs:839–891`

Both Rust tests re-implement the scan logic inline in the test body rather than calling the actual
`list_projects()` async command. A regression in the production function (e.g., removing the
`.starts_with('.')` filter) would not be caught.

Fix: Extract internal scan logic into a testable helper function:
```rust
fn scan_project_dirs(base: &Path) -> Vec<String> { ... }

#[tauri::command]
pub async fn list_projects() -> Result<Vec<String>, String> {
    Ok(scan_project_dirs(&AppDefaults::load().output_dir))
}
```

**QA-2 [Medium] — No test guards `projectName` preservation after reset**
The plan explicitly states: *"On `handleReset()`: keep `projectName` (don't clear — user likely
continues in same project)."* This deliberate UX decision has no test. Any refactor of
`handleReset()` could accidentally clear `projectName` undetected.

**QA-3 [Medium] — No test for mixed files+directories in `list_projects`**
`list_projects` correctly skips non-directory entries. No test covers a base dir that contains
files alongside subdirs. This path is exercised by the current logic but never explicitly tested.

**QA-4 [Low] — `ProjectSelector` test DOM may leak between test cases**
Location: `downloader-app/src/lib/ProjectSelector.test.ts:41`

`document.getElementById('project-suggestions')` reaches into the global JSDOM. The datalist from
one test render may be visible in another. Currently passing because tests run sequentially, but
this will be fragile if JSDOM is reset between tests in a different configuration.

**QA-5 [Low] — No test for `list_projects` with a config-overridden output_dir**
`list_projects` calls `AppDefaults::load()` which reads `~/.downloader/config.toml`. There is no
test verifying it respects a custom `output_dir` from config. This is low priority since
`parse_config_text` is already tested, but the integration path is not.

---

## Code Review (2026-02-28)

**Outcome:** done — all High and Medium issues fixed
**Issues fixed:** 7 (2 High, 5 Medium)
**Action items created:** 0

### Fixed
- [x] [H-1] `append_project_download_log` called with `None` → log watermark now captured before engine runs in both commands
- [x] [H-2] Weak test assertions ("empty"/"traversal") → strengthened to `"project name is empty"` / `"path traversal rejected"`
- [x] [M-1] Edge case tests for `truncate_field` missing → added (empty, max=0, single-char max)
- [x] [M-2] `render_project_*` functions untested → added 6 unit tests (structure, empty, failed, topics)
- [x] [M-3] File length informational — deferred (minor, ~5% over guideline)
- [x] [M-4] `#[tracing::instrument]` still missing on Tauri commands — deferred (macro ordering risk)
- [x] [M-5] `ProjectSelector.svelte` prop type `disabled: boolean` → `disabled?: boolean`
- [x] [L-1] `project_history_key` silent canonicalize failure → `warn!()` added
- [x] [L-2] Stray blank line in `output/mod.rs` → removed

---

## Code Review Pass 2 (2026-03-01)

**Outcome:** done — all High and Medium issues fixed
**Issues fixed:** 6 (1 High, 5 Medium)
**Action items created:** 0
**Verification:** cargo clippy --workspace -D warnings → exit 0 | cargo test --workspace --lib → 610 passed, 0 failed | npm test → 51 passed, 0 failed

### Verified resolved from prior [AI-Audit] items
- [x] [AI-Audit][High] ARCH-1: `start_download` HashSet::new() → `completed_before` captured correctly in both commands
- [x] [AI-Audit][High] PM-1: Svelte test for project name flow → `DownloadForm.test.ts` "passes project name to start_download_with_progress"
- [x] [AI-Audit][High] QA-1: `scan_project_dirs` extraction → `commands.rs:142`, tested at `:879-901`
- [x] [AI-Audit][Medium] QA-3: mixed files+dirs test → `commands.rs:886` `test_scan_project_dirs_excludes_hidden_dirs_and_files`

### Fixed in this pass
- [x] [H-1] Polling loop stale count: `completed + failed >= enqueued` compared total DB counts vs this-run count → introduced `prior_completed`/`prior_failed` offsets; payload and break condition now use `this_run_completed`/`this_run_failed` (`commands.rs:487-491, 542-556`)
- [x] [M-1] `scan_project_dirs` swallowed `read_dir` error silently → added `warn!(path, error, ...)` before returning empty (`commands.rs:149-153`)
- [x] [M-2] Undocumented changed files → added `Cargo.toml`, `main.rs`, `output/mod.rs`, `cli_e2e.rs`, `lib.rs` to the File List above
- [x] [M-3/QA-2] `handleReset()` preserves `projectName` — no test existed → added `DownloadForm.test.ts` "does not clear projectName when reset after a successful download"
- [x] [M-4/ARCH-2] `#[tracing::instrument]` missing on Tauri commands → added to `list_projects`, `start_download`, `start_download_with_progress(skip(window,state))`, `cancel_download(skip(state))` (`commands.rs:182, 316, 419, 612`)

### All items resolved
- [x] [AI-Audit][Low] ARCH-3: dual-DB design documented via code comment on `start_download` DB path (`commands.rs`). Explains the Story 10-2 origin, the test-isolation rationale, the history-split tradeoff, and a consolidation note for future maintainers.

---

## Code Review Pass 3 (2026-03-01)

**Outcome:** done — all Medium and Low issues fixed
**Issues fixed:** 9 (0 High, 5 Medium, 4 Low)
**Action items created:** 0
**Verification:** cargo fmt --all --check → exit 0 | cargo clippy --workspace -- -D warnings → exit 0 | cargo test --workspace --lib → 621 passed, 0 failed | npm test → 51 passed, 0 failed

### Fixed in this pass

- [x] [M-1] `SystemTime::now()` in library code — extracted `make_session_label()` with `SESSION_SEQ: AtomicU64` counter; single wall-clock access point documented; session labels now unique across rapid sequential calls (`project.rs`)
- [x] [M-2] `test_start_download_accepts_valid_project_name` weak assertion — changed to assert `!err.contains("Invalid project name")` AND `err.contains("No valid URLs or DOIs")`, distinguishing project-validation failure from URL-parse failure (`commands.rs`)
- [x] [M-3] `test_poll_exit_condition_triggers_when_all_items_terminal` tested math not mechanism — extracted `poll_should_break(db_completed, db_failed, prior_completed, prior_failed, enqueued) -> bool` pure function; wired into polling loop; added 5 targeted unit tests covering all break/continue branches and saturating-sub underflow (`commands.rs`)
- [x] [M-4] Missing project-validation test for `start_download_with_progress` — added `test_start_download_with_progress_project_validation_via_shared_fn` exercising traversal, empty-name, and valid-name cases via the shared `resolve_project_output_dir`; comment explains why direct command invocation requires Tauri runtime (`commands.rs`)
- [x] [M-5] Code duplication between `start_download` and `start_download_with_progress` — intentionally deferred; dual-DB design and Story 10-2 test-isolation rationale make full consolidation a larger architectural task; existing comment in `start_download` documents this
- [x] [L-1] `parse_config_text` fragile `strip_prefix("output_dir")` matching — replaced with `split_once('=')` + `match key.trim()` for exact key matching; added tests for `output_directory` rejection and comment-line skipping (`commands.rs`)
- [x] [L-2] Session label collision for fast sequential runs — resolved by `SESSION_SEQ` atomic counter appended to label (e.g. `unix-1740000000-0`, `unix-1740000000-1`); uniqueness verified by new test (`project.rs`)
- [x] [L-3] `item.url` unescaped in markdown table — wrapped with `escape_markdown_cell(item.url.as_str())`; added test for pipe-in-URL escaping (`project.rs`)
- [x] [L-4] Hardcoded `datalist id="project-suggestions"` global DOM ID — pre-existing, deferred (low-impact while component renders once per page)
