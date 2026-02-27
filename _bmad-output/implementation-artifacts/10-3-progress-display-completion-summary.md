# Story 10.3: Progress Display + Completion Summary

Status: done

## Story

As a user,
I want real-time per-download progress and a completion summary in the desktop app,
so that I can see what's happening and know when my batch is done — matching the CLI's experience.

## Acceptance Criteria

1. A new Tauri command `start_download_with_progress(inputs: Vec<String>, window: tauri::Window) -> Result<DownloadSummary, String>` replaces (or wraps) the `start_download` command from Story 10-2. It drives the same resolve+enqueue flow then runs the engine in a background task while emitting progress events.
2. While downloads are in flight, Tauri emits events named `"download://progress"` to the frontend with a JSON payload: `{ completed: usize, failed: usize, total: usize, in_progress: [{ url: String, bytes_downloaded: i64, content_length: Option<i64> }] }`.
3. Events are emitted every 300 ms via polling `Queue::count_by_status()` and `Queue::get_in_progress()` (same pattern as CLI `progress_manager.rs` which polls every 120 ms).
4. The DownloadForm component is updated (or replaced by `DownloadPage`) to listen to `"download://progress"` events using `@tauri-apps/api/event` and render:
   - An aggregate progress bar: `completed / total` items
   - A list of in-progress items showing URL domain + bytes ratio (e.g. "arxiv.org — 1.2 MB / 3.4 MB")
   - A spinner/animated indicator while in flight
5. A "Cancel" button appears while downloads are in progress. Clicking it sets the `interrupted` flag, stopping the engine gracefully via `process_queue_interruptible()` (uses `Arc<AtomicBool>` interrupt mechanism from `downloader-core`).
6. After cancel, the status area shows "Cancelled — N completed, M failed" with the same What/Why/Fix layout for any failures.
7. On completion, the status area shows a `CompletionSummary` component with:
   - "Downloaded N file(s) to `<output_dir>`"
   - When failures > 0: a generic What/Why/Fix guidance block (per-item error strings are not returned by `DownloadSummary`; aggregate guidance is provided instead)
   - A "Download more" button that resets the form to idle
8. The `start_download` command from Story 10-2 is kept unchanged for unit-test compatibility; the new progress command is additive.
9. A `#[cfg(test)]` unit test in `commands.rs` verifies that the polling loop terminates after `DownloadStats::total() == enqueued` items are processed (using a mock queue that reports all items completed).
10. A Vitest component test covers the `ProgressDisplay` component: given a progress event payload with `completed=2, failed=1, total=5`, the component renders "2 / 5" and "1 failed".
11. `cargo build --workspace` exits 0.
12. `cargo test --workspace --lib` passes (all existing tests + new command tests).
13. `cargo clippy --workspace -- -D warnings` exits 0.
14. E2E smoke test (manual macOS): paste 2+ URLs → click Download → observe live progress update → completion summary appears with correct counts. Document in Completion Notes.

## Tasks / Subtasks

- [x] Task 1: Extend Tauri command layer for progress events (AC: #1, #2, #3, #8)
  - [x] Add `ProgressPayload` struct in `commands.rs`: `#[derive(serde::Serialize, Clone)]` with `completed: usize, failed: usize, total: usize, in_progress: Vec<InProgressItem>`
  - [x] Add `InProgressItem` struct: `#[derive(serde::Serialize, Clone)]` with `url: String, bytes_downloaded: i64, content_length: Option<i64>`
  - [x] Create `start_download_with_progress(inputs: Vec<String>, window: tauri::Window)` command in `commands.rs`:
    1. Same resolve+enqueue flow as `start_download` (extract shared helper `resolve_and_enqueue()`)
    2. Create `Arc<AtomicBool>` interrupted flag
    3. Spawn tokio task: `engine.process_queue_interruptible(&queue, &client, &output_dir, Arc::clone(&interrupted)).await`
    4. Spawn polling loop: every 300ms, call `queue.count_by_status()` (completed + failed) and `queue.get_in_progress()`, build `ProgressPayload`, emit `window.emit("download://progress", payload)`
    5. Poll until engine task completes; join engine task; return `DownloadSummary`
  - [x] Register `start_download_with_progress` in `lib.rs` `invoke_handler`
  - [x] Add `#[cfg(test)]` unit test: verify polling terminates when total == enqueued

- [x] Task 2: Build `ProgressDisplay` Svelte component (AC: #4, #7, #10)
  - [x] Create `downloader-app/src/lib/ProgressDisplay.svelte`:
    - Props: `payload: ProgressPayload | null`
    - Renders aggregate progress bar using `<progress value={completed} max={total}>`
    - Lists in-progress items: domain + bytes ratio (format bytes using human-readable helper)
    - Shows spinner CSS class when `payload && completed < total`
  - [x] Create `downloader-app/src/lib/CompletionSummary.svelte`:
    - Props: `summary: DownloadSummary`, `onReset: () => void`
    - Renders "Downloaded N file(s) to `<output_dir>`"
    - "Download more" button calls `onReset`
  - [x] Add `formatBytes(n: number): string` utility in `downloader-app/src/lib/utils.ts` (e.g. `"1.2 MB"`)
  - [x] Vitest test for `ProgressDisplay`: render with payload `{completed:2,failed:1,total:5,in_progress:[]}`, assert text "2 / 5" and "1 failed"

- [x] Task 3: Update `DownloadForm` to use `start_download_with_progress` (AC: #4, #5, #6)
  - [x] Add state: `progressPayload: ProgressPayload | null = null`, `interrupted = false`
  - [x] On click Download: call `invoke("start_download_with_progress", { inputs: [...] })`
  - [x] Set up event listener `await listen("download://progress", (e) => { progressPayload = e.payload })` before invoking; unlisten on cleanup
  - [x] Show `ProgressDisplay` while status === 'downloading'
  - [x] Show Cancel button (disabled after first click); on click, set interrupted state and call `invoke("cancel_download")` (see Task 4)
  - [x] Show `CompletionSummary` on success; show error panel (What/Why/Fix) on failure

- [x] Task 4: Implement `cancel_download` command (AC: #5, #6)
  - [x] Store `Arc<AtomicBool>` interrupted flag in `tauri::State<AppState>` so `cancel_download` can set it
  - [x] Define `struct AppState { interrupted: Arc<AtomicBool> }`; register with `.manage(AppState::default())` in `lib.rs`
  - [x] Add `#[tauri::command] async fn cancel_download(state: tauri::State<'_, AppState>) -> Result<(), String>`: sets `state.interrupted.store(true, Ordering::SeqCst)`; returns Ok
  - [x] Update `start_download_with_progress` to use `state.interrupted` instead of local `Arc::new(AtomicBool::new(false))`
  - [x] Register `cancel_download` in `lib.rs` `invoke_handler`

- [x] Task 5: Validation gates (AC: #11, #12, #13, #14)
  - [x] `cargo build --workspace` → exit 0
  - [x] `cargo test --workspace --lib` → all pass
  - [x] `cargo clippy --workspace -- -D warnings` → exit 0
  - [x] `npm test` in `downloader-app/` → all Vitest tests pass
  - [x] Document manual E2E smoke test in Completion Notes
  - [x] Update sprint status: mark `10-3-progress-display-completion-summary` as done

## Dev Notes

### Architecture: Polling-Based Progress (No Push Callbacks)

`DownloadEngine` has **no push-based progress callback mechanism**. Progress flows through the queue database:

- Engine calls `queue.update_progress(id, bytes_downloaded, content_length)` during download (see `downloader-core/src/download/engine/persistence.rs`)
- Caller polls `queue.count_by_status(QueueStatus::Completed/Failed)` and `queue.get_in_progress()` to observe state
- CLI does this via `progress_manager.rs` — spawn a tokio task that loops with 120 ms sleep

For the Tauri command, use the same pattern but emit Tauri events instead of updating a spinner.

### Key APIs

```rust
// downloader-core/src/download/engine.rs — line 328
pub async fn process_queue_interruptible(
    &self,
    queue: &Queue,
    client: &HttpClient,
    output_dir: &Path,
    interrupted: Arc<AtomicBool>,
) -> Result<DownloadStats, EngineError>

// downloader-core/src/queue/mod.rs — line 334
pub async fn count_by_status(&self, status: QueueStatus) -> Result<i64>

// downloader-core/src/queue/mod.rs — line 371
pub async fn get_in_progress(&self) -> Result<Vec<QueueItem>>

// downloader-core/src/queue/item.rs — line 76
pub struct QueueItem {
    pub id: i64,
    pub bytes_downloaded: i64,          // resumable progress
    pub content_length: Option<i64>,    // from Content-Length header
    pub url: String,
    pub status_str: String,
    ...
}
```

### State Management for Cancel

Use Tauri's managed state (`tauri::State`) to share the `interrupted` flag between the two commands:

```rust
// lib.rs
.manage(AppState::default())
.invoke_handler(tauri::generate_handler![
    commands::start_download,
    commands::start_download_with_progress,
    commands::cancel_download,
])
```

The `interrupted` flag must be reset to `false` at the start of each new `start_download_with_progress` call (so re-downloads work after a cancel).

### Parallel Polling + Engine Task

```rust
// Inside start_download_with_progress
let interrupted = Arc::clone(&state.interrupted);
interrupted.store(false, Ordering::SeqCst); // reset for new run

let engine_task = tokio::spawn(async move {
    engine.process_queue_interruptible(&queue, &client, &output_dir, interrupted).await
});

// Polling loop
loop {
    tokio::time::sleep(Duration::from_millis(300)).await;
    let completed = queue.count_by_status(QueueStatus::Completed).await.unwrap_or(0) as usize;
    let failed = queue.count_by_status(QueueStatus::Failed).await.unwrap_or(0) as usize;
    let in_progress = queue.get_in_progress().await.unwrap_or_default();
    let payload = ProgressPayload { completed, failed, total: enqueued, in_progress: ... };
    let _ = window.emit("download://progress", &payload);
    if completed + failed >= enqueued { break; }
}

let stats = engine_task.await??;
```

### Svelte Event Listener Pattern

```typescript
import { listen } from '@tauri-apps/api/event';

onMount(async () => {
  const unlisten = await listen<ProgressPayload>('download://progress', (e) => {
    progressPayload = e.payload;
  });
  return unlisten; // called on component destroy
});
```

### Vitest SSR Fix (from Story 10-2)

Tests MUST be run via `vitest.config.ts` (the separate file, not `vite.config.js`). The SvelteKit plugin causes SSR module resolution which breaks `mount()`. The `vitest.config.ts` uses standalone `svelte()` plugin + `resolve.conditions: ['browser']`.

### References

- CLI progress polling: `downloader-cli/src/app/progress_manager.rs`
- Download engine interruptible: `downloader-core/src/download/engine.rs:328`
- Queue status API: `downloader-core/src/queue/mod.rs`
- Tauri events: https://v2.tauri.app/develop/inter-process-communication/
- Project coding rules (85 rules): `_bmad-output/project-context.md`

## Party Mode Audit (AI)

**Date:** 2026-02-27
**Outcome:** pass_with_actions
**Counts:** 2 High · 4 Medium · 1 Low

### Findings

| Sev | Perspective | Finding |
|-----|-------------|---------|
| High | Architect | Race: `interrupted.store(false, ...)` at the start of each `start_download_with_progress` call could clobber a concurrent cancel. Use `Mutex<Option<Arc<AtomicBool>>>` so each invocation owns its own flag, or guard with a per-session token. Simplest fix: replace `AppState.interrupted: Arc<AtomicBool>` with `AppState { interrupted: Mutex<Arc<AtomicBool>> }` and create a fresh `Arc::new(AtomicBool::new(false))` per call, storing it for `cancel_download` to retrieve. |
| High | Architect | Polling race on completion: the `if completed + failed >= enqueued { break }` exit condition may fire before the last progress event is emitted. The engine may also complete between polls, leaving the frontend stuck at N-1/N. Fix: after `engine_task.await`, emit one final `ProgressPayload` with accurate counts before returning `DownloadSummary`. |
| Medium | QA/TEA | `ProgressDisplay.test.ts` calls `listen()` from `@tauri-apps/api/event` on mount — must mock that module in Vitest the same way `invoke` was mocked in 10-2: `vi.mock('@tauri-apps/api/event', () => ({ listen: vi.fn().mockResolvedValue(() => {}), emit: vi.fn() }))`. |
| Medium | QA/TEA | `cancel_download` has no unit test. Add `#[cfg(test)]` test: create `AppState::default()`, call `cancel_download(state)`, assert `state.interrupted.lock().unwrap().load(SeqCst) == true`. |
| Medium | Developer | `QueueStatus` may not be publicly re-exported from `downloader_core`. Check `downloader-core/src/lib.rs` for `pub use queue::QueueStatus`. If absent, use `downloader_core::queue::QueueStatus` or the `str` variant API. |
| Medium | PM | AC#9 refers to a "mock queue" — `Queue` is concrete. Use SQLite in-memory (`Database::new(":memory:")`) for the unit test, the same pattern used in existing queue integration tests. Update AC#9 to say "real Queue backed by in-memory SQLite". |
| Low | PM | AC#6 cancel wording conflates cancellation with failure errors. Update to: "After cancel, status shows `Cancelled — N completed, M failed`; if M > 0, each failed item is listed with its error string (What/Why/Fix format)." |

### Review Follow-ups (AI)

- [x] [AI-Audit][High] `AppState` race: replace `Arc<AtomicBool>` field with `Mutex<Option<Arc<AtomicBool>>>`. In `start_download_with_progress`: create `let flag = Arc::new(AtomicBool::new(false)); *state.interrupted.lock().unwrap() = Some(Arc::clone(&flag));`. In `cancel_download`: `if let Some(flag) = state.interrupted.lock().unwrap().as_ref() { flag.store(true, SeqCst); }`.
- [x] [AI-Audit][High] After `engine_task.await` returns `stats`, emit one final `window.emit("download://progress", ProgressPayload { completed: stats.completed(), failed: stats.failed(), total: enqueued, in_progress: vec![] })` before constructing `DownloadSummary`. This guarantees the frontend sees 100%.
- [x] [AI-Audit][Medium] Add `vi.mock('@tauri-apps/api/event', () => ({ listen: vi.fn().mockResolvedValue(() => {}), emit: vi.fn() }))` at the top of `ProgressDisplay.test.ts`.
- [x] [AI-Audit][Medium] Add `cancel_download` unit test: `AppState::default()` → `cancel_download(state)` → assert flag is true.
- [x] [AI-Audit][Medium] Before using `QueueStatus::Completed` etc. in `commands.rs`, grep `downloader-core/src/lib.rs` for `pub use queue::QueueStatus`. Add the import path accordingly.

## Dev Agent Record

### Agent Model Used

claude-sonnet-4-6

### Debug Log References

### Completion Notes List

- AC#11 ✅ `cargo build --workspace` → exit 0
- AC#12 ✅ `cargo test --workspace --lib` → 571 passed (566 core + 5 app: 3 error cases, cancel flag test, AC#9 poll exit condition test)
- AC#13 ✅ `cargo clippy --workspace -- -D warnings` → exit 0
- AC#10 ✅ Vitest: 16/16 pass (`npm test` in downloader-app: utils×6, ProgressDisplay×6, DownloadForm×4)
- **Key fix:** `tauri::Emitter` trait must be explicitly imported for `Window::emit()` to work in Tauri 2.x.
- **Architecture note:** Progress model is polling-based (not push). Polling loop checks `Queue::count_by_status()` + `Queue::get_in_progress()` every 300ms. Final event emitted after engine task joins to guarantee 100% delivery.
- **Cancel mechanism:** `AppState { interrupted: Mutex<Option<Arc<AtomicBool>>> }` — each run creates a fresh Arc, stores it in state; `cancel_download` command stores the flag. Race-safe via Mutex.
- AC#14 E2E smoke test (manual macOS checklist):
  1. `cd downloader-app && npm install`
  2. `cargo tauri dev` (from repo root)
  3. Native window opens with "Downloader" title
  4. Paste 2+ URLs (e.g. `https://arxiv.org/abs/2301.00001`) → click "Download"
  5. "Resolving…" text appears, then progress bar with `X / N` counter updates every 300ms
  6. In-progress list shows domain + bytes
  7. On completion: CompletionSummary shows "Downloaded N file(s) to ./downloader-output"
  8. "Download more" button resets the form
  9. Clicking "Cancel" mid-download stops engine gracefully; status shows "Cancelled — N completed, M failed" (+ error-hint block if any failed)
- **Code review fixes (post-dev):** AC#9 poll-exit unit test added; `$props()` runes migration applied to `ProgressDisplay.svelte` + `CompletionSummary.svelte`; cancel error-hint gap fixed; File List completed; `cargo fmt` applied.
- **Post-completion coverage hardening:** `cargo test --workspace --lib` → 584 passed (567 core + 17 app command tests)
- **Post-completion coverage hardening:** Vitest → 43/43 passing (`DownloadForm` 13, `ProgressDisplay` 12, `CompletionSummary` 10, `utils` 8)
- **Newly covered branches:** interrupt-slot cleanup helpers, listener teardown, cancel rejection stability, byte-only progress rendering, zero-success partial summaries

### File List

- `downloader-app/src-tauri/src/commands.rs` (extend — add `start_download_with_progress`, `cancel_download`, `AppState`, progress types; polling exit test)
- `downloader-app/src-tauri/src/lib.rs` (extend — register new commands + manage AppState)
- `downloader-app/src/lib/ProgressDisplay.svelte` (new — progress bar, in-progress list, spinner)
- `downloader-app/src/lib/CompletionSummary.svelte` (new — success/fail/cancel summary + Download more)
- `downloader-app/src/lib/utils.ts` (new — `formatBytes`, `urlDomain` helpers)
- `downloader-app/src/lib/DownloadForm.svelte` (update — uses `start_download_with_progress`, cancel, event listener)
- `downloader-app/src/lib/ProgressDisplay.test.ts` (new — 6 tests for ProgressDisplay)
- `downloader-app/src/lib/utils.test.ts` (update — 8 tests: formatBytes×6 + urlDomain×2, added negative/NaN guards)
- `downloader-app/src/lib/DownloadForm.test.ts` (update — added `@tauri-apps/api/event` mock)
- `downloader-app/src/lib/CompletionSummary.test.ts` (new — 8 tests covering all 5 render paths)

## Post-Completion Coverage Hardening

- [x] AC#3: expanded polling-adjacent backend coverage with helper tests for interrupt-slot cleanup across success, engine-error, and join-error paths
- [x] AC#4: expanded `ProgressDisplay.test.ts` for byte-only rendering, `progress` value/max assertions, empty active list, and invalid URL fallback display
- [x] AC#5: expanded `DownloadForm.test.ts` to verify cancel visibility, single-fire cancel invocation, and disabled state after the first click
- [x] AC#6: codified cancel-result UI behavior, including the cancelled summary path even when `cancel_download` itself rejects
- [x] AC#7: expanded `CompletionSummary.test.ts` for zero-success partial output and retained reset-button availability
- [x] AC#9: retained and extended command-side branch tests in `commands.rs` with new cleanup and edge-case coverage
- [x] AC#10: expanded component coverage in `ProgressDisplay.test.ts` beyond the original single payload assertion
- [x] AC#12: re-ran `cargo test --workspace --lib` successfully after the added hardening tests
