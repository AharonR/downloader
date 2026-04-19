# Phase 4 Code Review: progress_manager + queue_manager

**Review date:** 2026-02-19  
**Scope:** `queue_manager.rs`, `progress_manager.rs`, and `runtime.rs` refactor.

---

## 1. Correctness

### queue_manager.rs
- **create_queue** builds `state_dir`, creates it if missing, opens DB at `queue.db`, wraps in `Queue` and `Arc`, resets in-progress, logs "Recovered interrupted..." when `reset_count > 0`, then gets `history_start_id` via `latest_download_attempt_id()`.
- **Return type:** `Result<(Arc<Queue>, Option<i64>)>` matches `append_project_download_log(queue, output_dir, history_start_id: Option<i64>)` in `project/mod.rs`. The plan doc mentioned `i64`; the implementation correctly uses `Option<i64>` to match the Queue API and project API.
- **Call order:** Caller (runtime) creates `output_dir` before calling; queue_manager only creates `state_dir`. No redundant creation.

### progress_manager.rs
- **spawn_progress_ui:** When `!use_spinner`, returns `(None, Arc::new(AtomicBool::new(true)))` so the stop flag is already true and the caller’s `progress_stop.store(true)` and `handle.await` remain no-ops. When `use_spinner`, returns `(Some(handle), stop)` in **(handle, stop)** order as specified.
- **Spinner loop:** Matches prior behavior: 100ms steady tick, poll completed/failed/in_progress, domain from first in-progress item or `"queue"`, message `"[current/total] Downloading from {domain}..."`, 120ms sleep, `finish_and_clear()` when stop is set. No behavioral change.
- **Error handling:** Queue `count_by_status` / `get_in_progress` use `unwrap_or(0)` / `unwrap_or_default()`; same as original `spawn_spinner`. Acceptable for a best-effort progress UI.

### runtime.rs
- **Flow:** state_dir/has_prior_state for early return → output dir creation → `queue_manager::create_queue` → resolution → bail/early success → completed_before → ctrl_c + progress → `progress_manager::spawn_progress_ui` → `run_download` → progress stop + await → summary/sidecars/interrupt check → project append (using `history_start_id`) → exit. Matches plan.
- **Progress lifecycle:** `progress_stop.store(true, Ordering::SeqCst)` then `if let Some(handle) = progress_handle { let _ = handle.await; }` preserves ordering and avoids leaking the spinner task.
- **history_start_id:** Passed as `Option<i64>` to `project::append_project_download_log`; type is correct.

---

## 2. Plan and convention alignment

- **Visibility:** New items are `pub(crate)`; no new public API.
- **Behavior:** No change to user-visible behavior, log text, or error handling.
- **queue_manager:** Does not log `state_dir` or `db_path` (plan/audit security note).
- **progress_manager:** Does not decide when to show the spinner; runtime calls `terminal::should_use_spinner(...)` and passes `use_spinner`.
- **Module wiring:** `progress_manager` and `queue_manager` declared in `mod.rs` in alphabetical order.

---

## 3. Edge cases and safety

- **total_queued == 0:** Runtime returns before calling `spawn_progress_ui`, so `total` is never 0 in practice. If it were, `current.min(total)` and denominator `total` would still be safe.
- **JoinHandle.await:** Result of `handle.await` is ignored (`let _ = handle.await`). Same as before; spinner task panic would only surface as a dropped `JoinError`. Acceptable for this UI.
- **output_dir missing:** Plan says caller ensures output_dir exists. Runtime creates it before `create_queue`. If someone else called `create_queue` with a non-existent path, `create_dir_all(&state_dir)` would create the full path; no change needed for current usage.

---

## 4. Minor observations (non-blocking)

- **queue_manager:** Doc comment says "Do not log state_dir or db_path in debug"; there are no `debug!` calls in the file, so the constraint is trivially satisfied and the comment is a good guard for future edits.
- **progress_manager:** No `tracing` use; other orchestrators (e.g. download_orchestrator) use `debug!`. Plan does not require logging here; adding optional `debug!` for “spinner started/stopped” could be a later improvement.
- **runtime:** Imports are minimal; `Queue` is not imported (type comes from `create_queue` and method calls via `Arc<Queue>`). No unused imports.

---

## 5. Verdict

- **Correctness:** Implementation matches the plan and the existing contract (including `Option<i64>` for `history_start_id` and (handle, stop) return order).
- **Security:** No logging of `state_dir` or `db_path`; no new sensitive data in logs.
- **Tests:** `cargo test --bin downloader` (227 tests) passes. The three failing tests in the full suite are in `downloader_core` (parser/resolver) and are unrelated to Phase 4.

**Conclusion:** Phase 4 code is correct, consistent with the plan and conventions, and ready to merge. No mandatory changes; optional improvements (e.g. optional progress_manager debug logging) can be done later if desired.
