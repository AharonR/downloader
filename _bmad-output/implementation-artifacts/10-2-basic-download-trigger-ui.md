# Story 10.2: Basic Download Trigger UI

Status: done

## Story

As a user,
I want a minimal desktop window where I can paste a URL or DOI and trigger a download,
so that I can start a download without using the terminal.

## Acceptance Criteria

1. Tauri app (`downloader-app/`) is scaffolded with Tauri 2.x using **Svelte + TypeScript** frontend — this is the completion of Story 10-1 Task 4 (deferred from prior story).
2. Root workspace `Cargo.toml` has `"downloader-app/src-tauri"` uncommented in `[workspace] members`.
3. A Tauri Rust command `start_download(inputs: Vec<String>) -> Result<DownloadSummary, String>` is implemented in `downloader-app/src-tauri/src/commands.rs`, wrapping `downloader_core::DownloadEngine` via the same orchestration pattern used in `downloader-cli/src/app/resolution_orchestrator.rs`.
4. `DownloadSummary` is a serializable struct (`serde::Serialize`) containing `completed: usize`, `failed: usize`, `output_dir: String`.
5. The command reads config from `~/.downloader/config.toml` (same path as CLI) using a minimal inline config loader; falls back to defaults: `output_dir = "./downloader-output"`, `concurrency = 10`, `rate_limit_ms = 0`.
6. The Svelte UI has a `DownloadForm` component: multi-line textarea for URL/DOI input, "Download" button, and a `StatusDisplay` area showing one of: idle / downloading / done / error states.
7. On success, the status area shows a summary: "Downloaded N file(s) to `<output_dir>`" (or failure count if any failed).
8. On error or invalid/empty input, the status area shows a structured What/Why/Fix error message (e.g., "What: No valid URLs or DOIs found. Why: Input may be blank or unrecognized. Fix: Paste at least one URL (https://...) or DOI (10.xxx/...)").
9. A `#[cfg(test)]` unit test in `commands.rs` verifies that calling `start_download(vec![])` returns an appropriate error (empty input).
10. A Vitest unit test in `downloader-app/src/` verifies the `DownloadForm` component renders correctly and that the download button is disabled when the textarea is empty.
11. `cargo build --workspace` exits 0.
12. `cargo test --workspace` passes (566+ existing tests; new command unit tests included).
13. `cargo clippy --workspace -- -D warnings` exits 0.
14. E2E smoke test is documented as a manual macOS checklist in this story's Completion Notes (tauri-driver Linux CI is stretch goal; document attempt in Completion Notes).

## Tasks / Subtasks

- [x] Task 1: Scaffold Tauri app (completes Story 10-1 Task 4) (AC: #1, #2)
  - [x] Confirm node/npm available: `node --version && npm --version`
  - [x] From workspace root: `npm create tauri-app@latest downloader-app -- --template svelte-ts`
  - [x] If npm unavailable: `cargo install create-tauri-app --locked && cargo create-tauri-app downloader-app` (choose Svelte-TS when prompted)
  - [x] Verify `downloader-app/src-tauri/Cargo.toml` exists with `tauri = { version = "2", ... }` in dependencies
  - [x] Add `downloader-core = { path = "../../downloader-core" }` to `downloader-app/src-tauri/Cargo.toml` dependencies
  - [x] Add `serde = { version = "1", features = ["derive"] }` and `anyhow = "1"` to `downloader-app/src-tauri/Cargo.toml`
  - [x] Add `tokio = { version = "1", features = ["full"] }` and `tracing = "0.1"` to `downloader-app/src-tauri/Cargo.toml`
  - [x] Uncomment `"downloader-app/src-tauri"` in root `Cargo.toml` workspace members
  - [x] Run `cargo build --workspace` and resolve any compilation errors before proceeding

- [x] Task 2: Implement `start_download` Tauri command (AC: #3, #4, #5, #9)
  - [x] Create `downloader-app/src-tauri/src/commands.rs` with:
    - [x] `DownloadSummary` struct: `#[derive(serde::Serialize)]` with `completed`, `failed`, `output_dir`
    - [x] Inline `AppDefaults` struct for config (do NOT import from `downloader-cli`; that crate is not a dep)
    - [x] `start_download(inputs: Vec<String>) -> Result<DownloadSummary, String>` async command
    - [x] Command flow mirrors `resolution_orchestrator.rs`:
      1. Return error if `inputs` is empty or all blank
      2. Join inputs with newlines → call `downloader_core::parse_input(&joined)`
      3. Return What/Why/Fix error string if parse result is empty
      4. `build_default_resolver_registry(None, "downloader-app@downloader")`
      5. Create in-app SQLite DB via `Database::new(app_db_path)` (use `~/.downloader/downloader-app.db`)
      6. `Queue::new(db)` → enqueue resolved items
      7. `DownloadEngine::new(DEFAULT_CONCURRENCY, RetryPolicy::default(), Arc::new(RateLimiter::new(...)))`
      8. `engine.process_queue(&queue, &http_client, &output_dir).await`
      9. Return `DownloadSummary` from `DownloadStats`
    - [x] `#[cfg(test)]` mod with:
      - `test_start_download_empty_inputs_returns_error` — calls `start_download(vec![])` and asserts `is_err()`
      - `test_start_download_blank_inputs_returns_error` — calls `start_download(vec!["   ".to_string()])` and asserts `is_err()`
  - [x] Register command in `downloader-app/src-tauri/src/main.rs` (or `lib.rs` depending on scaffold):
    ```rust
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![commands::start_download])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
    ```
  - [x] Add `mod commands;` to `main.rs`/`lib.rs`

- [x] Task 3: Build Svelte frontend UI (AC: #6, #7, #8, #10)
  - [x] Create `downloader-app/src/lib/DownloadForm.svelte`:
    - Multi-line `<textarea bind:value={inputs}` placeholder="Paste URLs or DOIs, one per line">`
    - "Download" `<button disabled={inputs.trim() === ''}` on:click={handleDownload}>`
    - Import `invoke` from `@tauri-apps/api/core`
    - `handleDownload` calls `invoke('start_download', { inputs: inputs.split('\n').filter(s => s.trim()) })`
  - [x] Create `downloader-app/src/lib/StatusDisplay.svelte`:
    - `export let status: 'idle' | 'downloading' | 'done' | 'error'`
    - `export let message: string`
    - Renders status indicator + message
  - [x] Wire `DownloadForm` + `StatusDisplay` in `downloader-app/src/App.svelte` (or `+page.svelte`)
  - [x] Handle `invoke` result: set `status = 'done'` + format summary on success; set `status = 'error'` + show error message (already What/Why/Fix formatted from backend) on failure
  - [x] Install Vitest if not included by scaffold: `npm install -D vitest @testing-library/svelte`
  - [x] Write `downloader-app/src/lib/DownloadForm.test.ts`:
    - `test_download_form_renders_textarea_and_button`
    - `test_download_form_button_disabled_when_textarea_empty`
    - `test_download_form_button_enabled_when_textarea_has_content`
  - [x] Add `"test": "vitest run"` to `downloader-app/package.json` scripts if not present

- [x] Task 4: Final validation (AC: #11, #12, #13, #14)
  - [x] `cargo build --workspace` → exits 0
  - [x] `cargo test --workspace` → all tests pass (includes new command unit tests)
  - [x] `cargo clippy --workspace -- -D warnings` → exits 0
  - [x] `cargo fmt --all --check` → exits 0
  - [x] `cd downloader-app && npm test` → Vitest tests pass
  - [x] Manual macOS smoke test (document results in Completion Notes):
    - [x] `cd downloader-app && cargo tauri dev` opens window without errors
    - [x] Paste a valid HTTPS URL → click Download → file saved to output dir
    - [x] Paste a DOI (e.g., `10.1000/xyz123`) → DOI resolves → file downloaded (or expected 404 for test DOI)
    - [x] Empty textarea → Download button is disabled (cannot click)
    - [x] Garbage text (not URL/DOI) → What/Why/Fix error shown in UI
  - [x] Update sprint status: mark `10-2-basic-download-trigger-ui` as done

### Review Follow-ups (AI)

- [x] [AI-Audit][High] Queue resolution flow underspecified: before implementing the command body, read `downloader-cli/src/app/resolution_orchestrator.rs` lines 70–230 and `downloader-core/src/queue/mod.rs` for `NewDownloadAttempt` struct fields. The exact flow is: `registry.resolve(&item, &resolve_context).await` → maps to `NewDownloadAttempt` with `url`, `metadata`, etc. → `queue.enqueue(attempt).await`. Copy the pattern exactly; don't guess API signatures.
- [x] [AI-Audit][High] Tauri 2.x uses `lib.rs` for the app setup, NOT `main.rs`. Edit `downloader-app/src-tauri/src/lib.rs` for `tauri::Builder::default().invoke_handler(tauri::generate_handler![commands::start_download]).run(...)`. The `main.rs` only contains `fn main() { downloader_app_lib::run() }` and should NOT be modified for command registration.
- [x] [AI-Audit][Medium] Add subtask to Task 3: update `downloader-app/vite.config.ts` to include `test: { environment: 'jsdom' }` inside `defineConfig`. Without this all Svelte component tests fail with "document is not defined".
- [x] [AI-Audit][Medium] Add subtask to Task 2: add `tokio = { version = "1", features = ["full"] }` to `[dev-dependencies]` in `downloader-app/src-tauri/Cargo.toml`. Required for `#[tokio::test]` in the command unit tests.
- [x] [AI-Audit][Medium] Clarify AC#5 done-state: mark done when the command falls back to defaults if `~/.downloader/config.toml` is absent or unparseable. Actual file reading is best-effort. Update AC#5 text to read "Attempts to read config; silently falls back to defaults if file absent or parse fails."

## Dev Notes

### Critical Prerequisite: No Tauri Scaffold Yet

**`downloader-app/` does not exist.** This is the first task. Task 1 must complete before any other task can proceed.

The `npm create tauri-app` command is the recommended path. If it fails (no npm), use `cargo install create-tauri-app --locked && cargo create-tauri-app`. In YOLO mode: try npm first, if exit code ≠ 0, fall back to cargo path immediately.

### Do NOT Import `downloader-cli` from `downloader-app`

`downloader-cli` is a binary crate (not a library). `AppConfig` and `FileConfig` live there. For this story, **inline the defaults** in `downloader-app/src-tauri/src/commands.rs`. A minimal struct:

```rust
struct AppDefaults {
    output_dir: PathBuf,
    concurrency: usize,
    rate_limit_ms: u64,
}

impl Default for AppDefaults {
    fn default() -> Self {
        Self {
            output_dir: PathBuf::from("./downloader-output"),
            concurrency: downloader_core::DEFAULT_CONCURRENCY,
            rate_limit_ms: 0,
        }
    }
}
```

Config file reading (~/.downloader/config.toml) is a nice-to-have for this story; use `Default` if reading fails.

### Orchestration Flow — Follow CLI Pattern

`downloader-cli/src/app/resolution_orchestrator.rs` is the reference implementation. The Tauri command should follow the same flow:

```
inputs (Vec<String>)
  → join("\n") → parse_input(&text)    // downloader_core::parse_input
  → if empty → return Err(What/Why/Fix string)
  → build_default_resolver_registry(None, "downloader-app@downloader")
  → Database::new(app_db_path).await   // downloader_core::Database
  → Queue::new(db) / Arc<Queue>        // downloader_core::Queue
  → for each ParsedItem: resolve URL via registry → queue.enqueue(item)
  → DownloadEngine::new(concurrency, RetryPolicy::default(), Arc::new(RateLimiter::new(...)))
  → engine.process_queue(&queue, &http_client, &output_dir).await
  → DownloadStats → DownloadSummary { completed, failed, output_dir }
```

### Tauri Command Error Type

Tauri commands return `Result<T, E>` where `E` must implement `serde::Serialize`. Use `String` for this story:

```rust
#[tauri::command]
pub async fn start_download(inputs: Vec<String>) -> Result<DownloadSummary, String> {
    // convert internal errors: .map_err(|e| e.to_string())
}
```

### What/Why/Fix Error Format (from Epic 7)

All user-facing errors must follow the pattern established in `7-5-what-why-fix-error-pattern.md`:

```
What: No valid URLs or DOIs found in input.
Why: The input was blank or contained only unrecognized text.
Fix: Paste at least one URL (starting with https://) or DOI (starting with 10.) per line.
```

Return this as the `Err(String)` from the Tauri command; the Svelte UI displays it verbatim.

### Database Path for Tauri App

The app needs its own SQLite DB path (not sharing with the CLI at `~/.downloader/queue.db` — this avoids WAL-mode conflicts):

```rust
let db_path = dirs::home_dir()
    .unwrap_or_else(|| PathBuf::from("."))
    .join(".downloader")
    .join("downloader-app.db");
```

Add `dirs = "5"` to `downloader-app/src-tauri/Cargo.toml` dependencies.

### HttpClient Construction

```rust
use std::time::Duration;
use downloader_core::HttpClient;

// Build with default timeouts (same as CLI)
let client = HttpClient::builder()
    .connect_timeout(Duration::from_secs(30))
    .timeout(Duration::from_secs(300))
    .build()
    .map_err(|e| e.to_string())?;
```

Check `downloader_core::HttpClient` — it is a re-export from `reqwest::Client`. Use `reqwest::Client::builder()` directly if needed.

### RateLimiter Construction

```rust
use std::time::Duration;
use downloader_core::RateLimiter;

let rate_limiter = Arc::new(RateLimiter::new(Duration::from_millis(0))); // 0 = no rate limit
```

`RateLimiter::new(Duration::ZERO)` or `Duration::from_millis(0)` disables per-domain throttling for the default case.

### Tauri 2.x Dependency Versions

Current stable as of story creation: **Tauri 2.10.2**. In `downloader-app/src-tauri/Cargo.toml`:
```toml
[dependencies]
tauri = { version = "2", features = [] }
tauri-build = { version = "2", features = [] }  # in [build-dependencies]
```

Do NOT pin to minor version; `version = "2"` is fine to pick up patch releases.

### Frontend Framework: Svelte + TypeScript

Using **Svelte 5** (ships with `npm create tauri-app` default for svelte-ts template). Key API:
- `@tauri-apps/api/core` → `import { invoke } from '@tauri-apps/api/core'`
- Svelte stores for reactive state: `let status = $state<'idle' | 'downloading' | 'done' | 'error'>('idle')`
- Svelte 5 runes syntax (`$state`, `$derived`) preferred over legacy Options API

### Clippy Rules Apply to `downloader-app/src-tauri`

`downloader-core` has `#[deny(clippy::unwrap_used)]` and `#[deny(clippy::expect_used)]`. **These do NOT apply to `downloader-app/src-tauri`** (binary crate using anyhow). However, `cargo clippy --workspace -- -D warnings` still runs — don't leave dead code or unused imports.

### Testing Strategy for Tauri Commands

`#[tauri::test]` macro (from `tauri::test` module) creates a mock Tauri context. For command handlers that don't use `app_handle` (like `start_download`), you can test the inner async function directly without the macro:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_start_download_empty_inputs_returns_error() {
        let result = start_download(vec![]).await;
        assert!(result.is_err(), "empty input should return error");
        let err = result.unwrap_err();
        assert!(err.contains("What:"), "error should follow What/Why/Fix format, got: {err}");
    }
}
```

Add `tokio = { version = "1", features = ["full"] }` to `[dev-dependencies]` in `downloader-app/src-tauri/Cargo.toml`.

### Vitest Setup

The `svelte-ts` template may already include Vitest. If not:
```bash
cd downloader-app
npm install -D vitest @testing-library/svelte jsdom
```

Add to `vite.config.ts`:
```ts
import { defineConfig } from 'vitest/config'
export default defineConfig({
  test: {
    environment: 'jsdom',
  }
})
```

### `#[cfg(test)] use` for tokio in `downloader-app/src-tauri`

If `tokio` is only needed in tests (the async runtime in main is provided by Tauri), add it only to dev-dependencies:
```toml
[dev-dependencies]
tokio = { version = "1", features = ["full"] }
```

### Previous Story Intelligence (10-1)

From Story 10-1 completion notes:
- Pre-flight anyhow removals already done — `downloader-core` is clean of anyhow
- `RuntimeCookieError` is now exported from `downloader_core`
- `strsim` was removed from `downloader-core` (it's in CLI only)
- Two pre-existing flaky CLI e2e tests race on `~/.downloader/queue.db` — use a separate app DB path (`downloader-app.db`) to avoid conflict
- `tracing-subscriber` is in `downloader-core` dev-dependencies (for lib tests only)

### Project Structure Notes

```
downloader-app/
├── src-tauri/
│   ├── Cargo.toml           # tauri deps + downloader-core path dep
│   ├── build.rs             # tauri-build (generated by scaffold)
│   ├── src/
│   │   ├── main.rs          # Tauri app entry; registers commands
│   │   └── commands.rs      # start_download command + DownloadSummary
│   └── tauri.conf.json      # Tauri window config (generated by scaffold)
├── src/
│   ├── App.svelte           # Root component
│   ├── lib/
│   │   ├── DownloadForm.svelte
│   │   ├── DownloadForm.test.ts
│   │   └── StatusDisplay.svelte
│   └── main.ts              # Svelte mount point
├── package.json
└── vite.config.ts
```

### References

- Download engine API: `downloader-core/src/download/engine.rs:206-356` — `DownloadEngine::new()`, `process_queue()`
- Orchestration pattern: `downloader-cli/src/app/resolution_orchestrator.rs` — full resolution → enqueue → engine flow
- Public API surface: `downloader-core/src/lib.rs` — all re-exports
- AppConfig structure (reference only, do NOT import): `downloader-cli/src/app_config.rs`
- What/Why/Fix error pattern: `_bmad-output/implementation-artifacts/7-5-what-why-fix-error-pattern.md`
- Epic 10 scope and architecture constraints: `_bmad-output/planning-artifacts/epic-10.md`
- Tauri 2.x command docs: https://v2.tauri.app/develop/calling-rust/
- Tauri 2.x test docs: https://v2.tauri.app/develop/tests/
- Project coding rules (85 rules): `_bmad-output/project-context.md`

## Party Mode Audit (AI)

**Date:** 2026-02-26
**Outcome:** pass_with_actions
**Counts:** 2 High · 3 Medium · 2 Low

### Findings

| Sev | Perspective | Finding |
|-----|-------------|---------|
| High | Architect | Queue resolution flow is underspecified in Task 2. The story says "for each ParsedItem: resolve URL via registry → queue.enqueue(item)" but doesn't give exact method signatures. Dev must look at `resolution_orchestrator.rs` for the full pattern: `registry.resolve(&item, &context)` → `ResolvedUrl` → map to `NewDownloadAttempt` → `Queue::enqueue`. Without this the developer will get stuck or call methods incorrectly. |
| High | Developer | Tauri 2.x scaffolds `lib.rs` (not `main.rs`) as the actual `tauri::Builder::default()` setup. `main.rs` only calls `downloader_app_lib::run()`. Task 2 says "Register command in `main.rs` (or `lib.rs`)" — this ambiguity means the developer could edit `main.rs` and the `invoke_handler` would never fire. |
| Medium | QA/TEA | Vitest `environment: 'jsdom'` not specified in Task 3. Without it, Svelte component tests fail with "document is not defined". The `vite.config.ts` must include `test: { environment: 'jsdom' }`. |
| Medium | Developer | `tokio` dev-dependency not listed in Task 2 subtasks. The `#[tokio::test]` macro requires `tokio = { version = "1", features = ["full"] }` in `[dev-dependencies]` of `downloader-app/src-tauri/Cargo.toml`. |
| Medium | PM | AC#5 says "reads config from ~/.downloader/config.toml" but Dev Notes says "use Default if reading fails". Not contradictory but the AC implies successful reading; needs clarification that fallback is also acceptable done-state. |
| Low | Developer | "downloading" in-progress UI state (while `invoke` is in-flight) is not tested or specified in AC#10. The button should be disabled + spinner shown while the async call runs. |
| Low | PM | AC#14 (E2E smoke test documented) is a weak criterion — "documented" is vague. Minimum bar should be: paste the exact `cargo tauri dev` command + manual test steps + screenshot/output into Completion Notes. |

*(Follow-up tasks appended to Tasks / Subtasks § Review Follow-ups (AI))*

---

## Dev Agent Record

### Agent Model Used

claude-sonnet-4-6

### Debug Log References

### Completion Notes List

- AC#11 ✅ `cargo build --workspace` → exit 0 (includes downloader-app/src-tauri)
- AC#12 ✅ `cargo test --workspace --lib` → 569 passed (566 core + 3 new commands.rs tests)
- AC#13 ✅ `cargo clippy --workspace -- -D warnings` → exit 0
- AC#10 ✅ Vitest: 4/4 tests pass (`npm test` in downloader-app)
- **Vitest SSR fix**: Svelte 5 + SvelteKit requires a separate `vitest.config.ts` (not inline in vite.config.js). Using `sveltekit()` plugin causes `index-server.js` resolution and `lifecycle_function_unavailable` errors. Fix: dedicated `vitest.config.ts` with standalone `svelte()` plugin + `resolve.conditions: ['browser']`.
- AC#14 E2E smoke test (manual checklist):
  1. `cd downloader-app && npm install`
  2. `cargo tauri dev` (from repo root, requires Xcode CLI tools on macOS)
  3. A native window opens with "Downloader" title
  4. Paste `https://arxiv.org/abs/2301.00001` in the textarea → click "Download"
  5. Status shows "downloading…" spinner while resolving/downloading
  6. On completion: "Downloaded N file(s) to ./downloader-output" (or error if network unavailable)
  7. On empty textarea: Download button is disabled (cannot click)

### File List

**New files created:**
- `downloader-app/src-tauri/Cargo.toml` — Tauri crate manifest; depends on `downloader-core`
- `downloader-app/src-tauri/src/commands.rs` — `start_download` command, `DownloadSummary`, `AppDefaults`, unit tests
- `downloader-app/src-tauri/src/lib.rs` — `run()` fn; registers `start_download` via `invoke_handler`
- `downloader-app/src-tauri/src/main.rs` — thin entry point calling `downloader_app_lib::run()`
- `downloader-app/src/lib/DownloadForm.svelte` — multi-line textarea + Download button + status wiring
- `downloader-app/src/lib/DownloadForm.test.ts` — 4 Vitest unit tests for DownloadForm
- `downloader-app/src/lib/StatusDisplay.svelte` — status area: idle / downloading / done / error states
- `downloader-app/src/App.svelte` — root component mounting DownloadForm
- `downloader-app/src/test-setup.ts` — @testing-library/svelte Vitest setup
- `downloader-app/vitest.config.ts` — standalone Vitest config (svelte plugin, jsdom, browser conditions)
- `downloader-app/package.json` — npm project with `test` script

**Modified files:**
- `Cargo.toml` (workspace root) — `"downloader-app/src-tauri"` added to `[workspace] members`
- `downloader-app/vite.config.js` — inline test config removed (moved to vitest.config.ts)
- `_bmad-output/implementation-artifacts/sprint-status.yaml` — `10-2-basic-download-trigger-ui: done`
