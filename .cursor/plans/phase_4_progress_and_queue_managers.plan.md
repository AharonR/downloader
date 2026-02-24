# Phase 4: progress_manager + queue_manager

**Single source of truth for Phase 4 implementation.**  
Completes the app modularization by moving queue/DB setup and progress UI into dedicated modules.  
Last updated: 2026-02-19.

**Party-mode audit (2026-02-19) incorporated:** 7 parties; outcome pass_with_actions. Recommendations applied: (1) progress_manager does not decide spinner—runtime calls `terminal::should_use_spinner` and passes `use_spinner`; (2) `spawn_progress_ui` return order (handle, stop) and caller usage documented in Section 2; (3) optional queue_manager failure-path test noted in Section 5; (4) queue_manager must not log state_dir/db_path in debug (Section 1 Notes); (5) runtime imports explicitly keep Arc, AtomicBool, Ordering (Section 3 step 1).

---

## What Phase 4 Does

Phase 4 extracts the last two concerns from `runtime.rs`:

1. **queue_manager** — Creates the state directory (`.downloader` under output dir), initializes the database and queue, resets in-progress items, and returns the queue plus `history_start_id` for project append. Runtime no longer creates the queue or uses `Database`/`db_path`; it still keeps `state_dir` and `has_prior_state` for the no-input early return.
2. **progress_manager** — Hosts the progress spinner when requested: runtime decides via `terminal::should_use_spinner` and passes a bool; progress_manager only implements the spinner task that polls queue counts and updates the message. Runtime calls a single entry point and then signals stop and awaits the handle after download completes.

After Phase 4, `runtime.rs` is a thin orchestrator that only coordinates modules and flow; no direct queue/DB or progress UI logic.

---

## Conventions and Constraints

- **Visibility:** All new types and functions are `pub(crate)`; no new public API.
- **Behavior:** No change to user-visible behavior, log messages, or error handling.
- **Ordering:** Runtime still decides *when* to create the queue (after dry-run and no-input checks) and when to start/stop progress (around `run_download`). The new modules only encapsulate *how*.

---

## 1. queue_manager

**New file:** `src/app/queue_manager.rs`

### Role

Create the state directory under `output_dir`, initialize the database and queue, reset in-progress items, and return the queue and `history_start_id`. The caller must ensure `output_dir` exists before calling (runtime does this). The caller is responsible for using `history_start_id` in project append.

### Public API

- **async fn create_queue(output_dir: &Path, db_options: &DatabaseOptions) -> Result<(Arc<Queue>, i64)>**  
  Returns `(queue, history_start_id)`. Steps:
  - `state_dir = output_dir.join(".downloader")`.
  - If `!state_dir.exists()`, create it with `fs::create_dir_all(&state_dir)?`.
  - `db_path = state_dir.join("queue.db")`.
  - `db = Database::new_with_options(&db_path, db_options).await?`.
  - `queue = Arc::new(Queue::new(db))`.
  - `reset_count = queue.reset_in_progress().await?`; if `reset_count > 0`, log exactly: `info!(reset_count, "Recovered interrupted queue items from previous run")`.
  - `history_start_id = queue.latest_download_attempt_id().await?`.
  - Return `Ok((queue, history_start_id))`.

### Dependencies

- `std::path::Path`, `std::sync::Arc`, `std::fs`, `anyhow::Result`, `downloader_core::{Database, DatabaseOptions, Queue}`, `tracing::info`.

### Module wiring

In `src/app/mod.rs`, add in alphabetical order with existing modules:  
`pub(crate) mod queue_manager;`

### Notes

- Runtime continues to compute `state_dir = ctx.output_dir.join(".downloader")` and `has_prior_state = state_dir.exists()` for the early return when there is no input and no prior state. Only the creation of the queue and the "recovered interrupted" log move into queue_manager.
- Output dir creation (`fs::create_dir_all(&ctx.output_dir)` and the "Created output directory" log) stays in runtime; queue_manager only creates `state_dir`.
- **Security (AI-Audit):** Do not log `state_dir` or `db_path` in debug; they can reveal user directory layout.

---

## 2. progress_manager

**New file:** `src/app/progress_manager.rs`

### Role

Provide a single entry point to spawn the progress UI (spinner) when requested. Runtime decides whether to use the spinner via `terminal::should_use_spinner` and passes the result as `use_spinner`; progress_manager only implements the spinner. The spinner polls the queue for completed/failed/in-progress counts and updates a spinner message. Return order is **(handle, stop)**. Caller uses: `progress_stop.store(true, Ordering::SeqCst)`; `if let Some(h) = progress_handle { let _ = h.await; }`.

### Public API

- **fn spawn_progress_ui(**
  - **use_spinner: bool,**
  - **queue: Arc<Queue>,**
  - **total: usize** — total queued count (used as spinner denominator, e.g. "[current/total]")
  - **) -> (Option<tokio::task::JoinHandle<()>>, Arc<AtomicBool>)** — order: **(handle, stop)**
  - If `!use_spinner`, return `(None, Arc::new(AtomicBool::new(true)))` so the stop signal is already true; the caller may still call `progress_stop.store(true)` and `progress_handle.await` (no-op when handle is None).
  - If `use_spinner`, create `stop = Arc::new(AtomicBool::new(false))`, spawn the same spinner loop as today (indicatif `ProgressBar`, poll queue counts, format message with domain, sleep 120ms), and return `(Some(join_handle), stop)`. Order: **(handle, stop)**.

### Implementation details (match current runtime `spawn_spinner`)

- Use `indicatif::{ProgressBar, ProgressStyle}`, `downloader_core::{Queue, QueueStatus}`, `url::Url`, `std::sync::atomic::{AtomicBool, Ordering}`, `std::time::Duration`, `tokio`.
- Spinner loop: same logic as current `spawn_spinner` — `ProgressBar::new_spinner()`, steady_tick 100ms, loop: count completed/failed/in_progress, format "[current/total] Downloading from {domain}...", sleep 120ms per iteration, `finish_and_clear()` when stop is set.

### Dependencies

- `std::sync::Arc`, `std::sync::atomic::{AtomicBool, Ordering}`, `std::time::Duration`, `downloader_core::{Queue, QueueStatus}`, `indicatif::{ProgressBar, ProgressStyle}`, `url::Url`.

### Module wiring

In `src/app/mod.rs`, add in alphabetical order with existing modules:  
`pub(crate) mod progress_manager;`

### Notes

- Runtime will call `terminal::should_use_spinner(io::stderr().is_terminal(), ctx.args.quiet, terminal::is_dumb_terminal())` and pass the result as `use_spinner`. So terminal remains the authority for "should we use spinner"; progress_manager only implements the spinner.
- No change to when the spinner is shown; only the implementation moves.

---

## 3. runtime.rs refactor

### Goal

Replace inline queue/DB creation with `queue_manager::create_queue`, and replace inline spinner creation and lifecycle with `progress_manager::spawn_progress_ui`. Remove direct use of `Database`, `db_path`, and the local `spawn_spinner` function. **Keep** `state_dir` and `has_prior_state` in runtime for the no-input early return (only queue/DB creation moves to queue_manager).

### Steps

1. **Imports**
   - Remove: `Database` from downloader_core. Keep `Queue`, `QueueStatus` (runtime still uses them for `list_by_status`, `as_ref()`, and passing queue to other modules).
   - Keep: `Arc`, `AtomicBool`, `Ordering` (used for progress_stop and interrupted ctrl_c).
   - Remove: `indicatif`, `ProgressBar`, `ProgressStyle`, `Duration`, `Url` (only used by spinner; spinner moves to progress_manager).
   - Add: `queue_manager`, `progress_manager`.

2. **Queue creation block** (after output dir creation, before resolution)
   - Keep: `state_dir = ctx.output_dir.join(".downloader")`, `has_prior_state = state_dir.exists()` for the no-input early return.
   - Keep: `if !ctx.output_dir.exists() { fs::create_dir_all(&ctx.output_dir)?; info!(dir = %ctx.output_dir.display(), "Created output directory"); }`.
   - Replace: `if !state_dir.exists() { fs::create_dir_all(&state_dir)?; }`, `db_path`, `db`, `queue = Arc::new(Queue::new(db))`, `reset_count`, log "Recovered interrupted", `history_start_id` with:
     - `let (queue, history_start_id) = queue_manager::create_queue(&ctx.output_dir, &ctx.db_options).await?;`  
       (Return order: queue first, history_start_id second.)

3. **Progress UI block** (spinner setup before `run_download`, and stop/await after it)
   - Replace: `progress_stop = Arc::new(AtomicBool::new(false))`, `progress_handle = if terminal::should_use_spinner(...) { Some(spawn_spinner(...)) } else { None }` with:
     - `let use_spinner = terminal::should_use_spinner(io::stderr().is_terminal(), ctx.args.quiet, terminal::is_dumb_terminal());`
     - `let (progress_handle, progress_stop) = progress_manager::spawn_progress_ui(use_spinner, Arc::clone(&queue), total_queued);`
   - After `run_download`: keep `progress_stop.store(true, Ordering::SeqCst)` and `if let Some(handle) = progress_handle { let _ = handle.await; }` (same as today).

4. **Remove**
   - Delete the entire `fn spawn_spinner(...)` at the end of the file.

5. **Imports cleanup**
   - Keep: `HashSet`, `fs`, `io`, `PathBuf`, `Arc`, `AtomicBool`, `Ordering`, `Queue`, `QueueStatus` (still used in runtime). Remove: `Duration`, `Url`, `indicatif`, `ProgressBar`, `ProgressStyle`, `Database`.

### Verification

- Same flow: dry-run / no-input checks → output dir creation → queue creation (via queue_manager) → resolution → bail/early return → completed_before → interrupted + progress (via progress_manager) → run_download → progress stop and await → completion summary → sidecars → interrupt check → project append → exit.
- No behavioral or log change.

---

## 4. Dependencies and visibility

- **queue_manager:** Depends on `std::path::Path`, `std::fs`, `std::sync::Arc`, `anyhow`, `downloader_core::{Database, DatabaseOptions, Queue}`, `tracing`. Does not depend on runtime, progress_manager, or terminal.
- **progress_manager:** Depends on `std::sync::Arc`, `std::sync::atomic::{AtomicBool, Ordering}`, `std::time::Duration`, `downloader_core::{Queue, QueueStatus}`, `indicatif`, `url::Url`, `tokio`. Does not depend on runtime, queue_manager, or terminal.
- **runtime:** Depends on queue_manager, progress_manager, terminal (for should_use_spinner), and all other existing app/crate deps. Still uses `Queue` and `QueueStatus` for `list_by_status`, `as_ref()`, and passing the queue to other modules.

All new code is `pub(crate)`.

---

## 5. Testing

- **Unit tests (optional but recommended):**
  - **queue_manager:** Test that `create_queue` with a temp output dir returns a queue and a non-negative `history_start_id`; optionally that calling it twice on the same path yields a valid queue. Optional: test that create_queue returns an error when output_dir is not writable (or skip and rely on integration tests).
  - **progress_manager:** Test that when `use_spinner` is false, `spawn_progress_ui` returns `(None, _)` and the stop signal is already true (or that the returned handle is None). When `use_spinner` is true with a mock queue, spawn and immediately stop/await is possible but more involved; can be skipped and rely on integration tests.
- **Regression:** Run `cargo test --bin downloader` and full `cargo test`; manual smoke test that the spinner still appears when running without `-q` in a terminal.

---

## 6. Implementation order

1. Add `src/app/queue_manager.rs` with `create_queue`; add `pub(crate) mod queue_manager;` in `src/app/mod.rs`. Run `cargo build`.
2. Refactor `runtime.rs`: replace the queue/DB creation block with `queue_manager::create_queue`. Remove `Database` from imports. Run `cargo build` and `cargo test --bin downloader`.
3. Add `src/app/progress_manager.rs` with `spawn_progress_ui` (move spinner logic from runtime); add `pub(crate) mod progress_manager;` in `src/app/mod.rs`. Run `cargo build`.
4. Refactor `runtime.rs`: replace progress_stop/progress_handle and `spawn_spinner` with `progress_manager::spawn_progress_ui`; remove `spawn_spinner`, `indicatif`, `Duration`, `Url` from runtime. Run `cargo build` and `cargo test --bin downloader`.
5. Final pass: clean unused imports in runtime; run full test suite.

---

## 7. Out of scope for Phase 4

- Changing how the spinner message is formatted or how often it polls.
- Moving ctrl_c handling into a module (it stays in runtime).
- Exposing queue_manager or progress_manager as part of the public API.
- Changing config_runtime, validation, terminal, or other existing modules.
