# Phase 4: Reflection and Critical-Point Tests

**Date:** 2026-02-19

## Reflection

Phase 4 introduced two new modules used by the runtime:

1. **queue_manager** — Creates `.downloader` state dir, DB, and queue; resets in-progress items; returns `(Arc<Queue>, Option<i64>)` for project append. Critical that the return type matches `append_project_download_log(..., history_start_id: Option<i64>)` and that calling it is safe (e.g. idempotent on same path).
2. **progress_manager** — Returns `(handle, stop)`; when spinner is disabled, the stop signal must already be true so the caller’s `store(true)` and `await` remain no-ops. When enabled, the task must exit when stop is set so the runtime doesn’t hang.

Existing coverage already included:
- **exit_handler:** All four exit-outcome cases (success, partial, failure, zero/zero).
- **resolution_orchestrator:** No-input returns zeros.
- **config_manager, input_processor, command_dispatcher:** Unit tests for key paths.
- **Binary tests:** 227 tests exercising CLI, config, project, output, etc., which indirectly cover runtime’s use of queue_manager and progress_manager.

What was missing was **direct unit tests** for the new modules’ contracts so that:
- Refactors don’t break return types or ordering.
- The “no spinner” path keeps its no-op stop/handle contract.
- The “spinner enabled” path exits on stop (no hang).

## Tests Added

### queue_manager (2 tests)

| Test | What it guards |
|------|----------------|
| `create_queue_with_temp_dir_returns_queue_and_history_id` | `create_queue` with a temp output dir returns `Ok((queue, history_start_id))`; queue is usable (e.g. `list_by_status(Pending)`); `history_start_id` is `Option<i64>` (None for fresh DB). |
| `create_queue_twice_on_same_path_yields_valid_queues` | Calling `create_queue` twice on the same path yields two valid queues that both see the same DB (reopen is safe; no corruption). |

### progress_manager (2 tests)

| Test | What it guards |
|------|----------------|
| `spawn_progress_ui_when_disabled_returns_none_handle_and_stop_already_true` | When `use_spinner` is false: returns `(None, stop)` and `stop.load() == true`, so the caller’s `progress_stop.store(true)` and `progress_handle.await` are no-ops. |
| `spawn_progress_ui_when_enabled_returns_handle_and_stop_and_stop_ends_task` | When `use_spinner` is true: returns `(Some(handle), stop)` with stop false; after `stop.store(true)`, `handle.await` completes (spinner task exits; no hang). |

## Not covered (by design)

- **queue_manager:** Error when output_dir is not writable — plan marked as optional; can be added later or left to integration tests.
- **progress_manager:** Spinner message content or timing — plan said no change to behavior; existing binary tests give indirect coverage.
- **Runtime integration:** Full flow (create_queue → resolution → progress → run_download → stop/await) is covered by the 231 binary tests; no new integration test added.

## Result

- **Before:** 227 tests for the downloader binary.
- **After:** 231 tests (4 new unit tests in queue_manager and progress_manager).
- All 231 pass; no new linter issues.

These tests lock the critical contracts for Phase 4 so future changes don’t regress return types, ordering, or the spinner lifecycle.
