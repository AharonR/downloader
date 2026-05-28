# Story 11-1 — HTML → PDF Conversion in the Tauri UI

## Context

`downloader convert` was added as a CLI subcommand (see `downloader-cli/src/commands/convert.rs`). It walks a corpus directory, converts eligible `.html` files to `.pdf` using headless Chrome, and skips known-useless hosts (ScienceDirect paywall stubs, PubMed abstract pages, doi.org redirects).

This story wires the same feature into the desktop UI. The natural trigger point is the `CompletionSummary` screen, which is already shown after every download run and already holds `summary.output_dir`.

---

## User Flow

1. User runs a download. Some items land as `.html` (no PDF link was resolvable).
2. The Completion Summary screen appears. It shows the existing metrics (Completed, Needs attention, etc.) and the output folder path.
3. A new **"Convert HTML → PDF"** button appears next to the existing "Open output folder" button (inside `.output-block`) — but only when `output_dir` is set.
4. User clicks the button. It enters a loading state ("Converting…") and is disabled.
5. A Tauri IPC call (`convert_html_files`) runs in the background, emitting progress events as each file is processed.
6. A live counter updates: "Converting… 2 / 5".
7. When complete, the button area is replaced by a result line: "Converted 4 files (1 skipped)" — or an error message if Chrome was not found.
8. If the user clicks "Download more" and starts a new run, state resets.

**Edge cases:**
- No `.html` files in the corpus → "Nothing to convert (0 HTML files found)" — not an error.
- Chrome not found → "Chrome not found. Install Google Chrome or set `DOWNLOADER_CHROME_BINARY`." — shown inline as an error, not a crash.
- All files already have a sibling `.pdf` → "Nothing to convert (all already have PDFs)".
- One or more files fail → "Converted 3 files (1 failed, 1 skipped)" — non-zero failures are shown but not treated as an error state.

---

## Backend: New Tauri Command

### File: `downloader-app/src-tauri/src/commands.rs`

Add two items:

**1. Result type** (returned to frontend):

```rust
#[derive(serde::Serialize, Clone, Debug)]
pub struct ConvertResult {
    pub converted: usize,
    pub skipped: usize,
    pub failed: usize,
    pub total: usize,
}
```

**2. Progress event payload** (emitted per file during conversion):

```rust
#[derive(serde::Serialize, Clone, Debug)]
pub struct ConvertProgress {
    pub file: String,       // basename only, for display
    pub converted: usize,   // running count so far
    pub total_eligible: usize,
}
```

**3. Command signature:**

```rust
#[tauri::command]
pub async fn convert_html_files(
    corpus_dir: String,
    no_skip: bool,
    app_handle: tauri::AppHandle,
) -> Result<ConvertResult, String>
```

- `corpus_dir`: absolute path string (from `summary.output_dir`)
- `no_skip`: whether to bypass the known-useless-host filter (pass `false` from the UI; a future "advanced" option could expose it)
- `app_handle`: used to emit `convert-progress` events to the frontend

**4. Implementation steps** (inside the command body):

1. Validate `corpus_dir` exists as a directory — return `Err(...)` if not.
2. Locate Chrome binary using the same resolution order as the CLI:
   - `DOWNLOADER_CHROME_BINARY` env var
   - macOS default: `/Applications/Google Chrome.app/Contents/MacOS/Google Chrome`
   - PATH: `google-chrome`, `google-chrome-stable`, `chromium`, `chromium-browser`
   - If none found: return `Err("Chrome not found. Install Google Chrome or set DOWNLOADER_CHROME_BINARY.")`
3. Collect `.html` files (single-level `read_dir`, case-insensitive `.html` extension, sorted).
4. For each file, apply the same skip logic as the CLI:
   - Skip if sibling `.pdf` exists.
   - Skip if sidecar `.json` URL host matches `["sciencedirect.com", "pubmed.ncbi.nlm.nih.gov", "doi.org"]` and `no_skip` is false.
5. For each eligible file, spawn headless Chrome via `tokio::process::Command`:
   ```
   <chrome> --headless=new --disable-gpu --no-sandbox
            --print-to-pdf-no-header
            --print-to-pdf=<sibling.pdf>
            file://<absolute path>
   ```
   Treat non-zero exit as a per-file failure (increment `failed`, continue).
6. After each eligible file (whether converted or failed), emit a `convert-progress` event:
   ```rust
   app_handle.emit("convert-progress", ConvertProgress {
       file: basename,
       converted: running_converted_count,
       total_eligible: eligible_count,
   }).ok();
   ```
7. Return `Ok(ConvertResult { converted, skipped, failed, total })`.

**Note on code sharing:** The Chrome binary detection logic is ~20 lines. It is intentionally duplicated from `downloader-cli/src/commands/convert.rs` rather than extracted to `downloader-core`, because the app and CLI are separate crates and don't currently share helper utilities. If this duplication becomes a maintenance burden, move `find_chrome_binary` to a new `downloader-core::convert` module exposed as a pub fn.

**5. Register the command** in `downloader-app/src-tauri/src/lib.rs`:

```rust
.invoke_handler(tauri::generate_handler![
    // ... existing commands ...
    commands::convert_html_files,
])
```

---

## Frontend: `CompletionSummary.svelte` Changes

### File: `downloader-app/src/lib/CompletionSummary.svelte`

#### New state variables (in `<script>`):

```ts
type ConvertState = 'idle' | 'running' | 'done' | 'error';

let convertState = $state<ConvertState>('idle');
let convertResult = $state<{ converted: number; skipped: number; failed: number; total: number } | null>(null);
let convertError = $state<string | null>(null);
let convertProgress = $state<{ converted: number; totalEligible: number } | null>(null);
```

#### New event listener setup:

Listen for `convert-progress` events from Tauri using `listen` from `@tauri-apps/api/event`:

```ts
import { listen } from '@tauri-apps/api/event';

// set up in onMount / $effect, tear down on destroy
const unlisten = await listen<{ converted: number; total_eligible: number }>('convert-progress', (event) => {
    convertProgress = {
        converted: event.payload.converted,
        totalEligible: event.payload.total_eligible,
    };
});
// call unlisten() on component destroy
```

#### New handler function:

```ts
async function handleConvertHtml() {
    convertState = 'running';
    convertError = null;
    convertResult = null;
    convertProgress = null;
    try {
        const result = await invoke<{ converted: number; skipped: number; failed: number; total: number }>(
            'convert_html_files',
            { corpusDir: summary.output_dir, noSkip: false }
        );
        convertResult = result;
        convertState = 'done';
    } catch (err) {
        convertError = typeof err === 'string' ? err : 'Conversion failed';
        convertState = 'error';
    }
}
```

#### Reset on `onReset`:

When the user clicks "Download more", reset all convert state:
```ts
function handleReset() {
    convertState = 'idle';
    convertResult = null;
    convertError = null;
    convertProgress = null;
    onReset();
}
```

#### Template changes (inside `.output-block`):

The existing `.output-block` has two children: `.output-copy` (label + path) and the "Open output folder" button. Add the convert button as a third element, shown only when `output_dir` exists:

```svelte
{#if summary.output_dir && (summary.completed > 0 || skippedDuplicates > 0)}
  <div class="output-block">
    <div class="output-copy">
      <p class="output-label">Project output</p>
      <code class="output-dir">{summary.output_dir}</code>
    </div>
    <div class="output-actions">
      <button class="open-folder-btn" onclick={handleOpenFolder} type="button">
        Open output folder
      </button>

      {#if convertState === 'idle'}
        <button class="convert-btn" onclick={handleConvertHtml} type="button">
          Convert HTML → PDF
        </button>
      {:else if convertState === 'running'}
        <button class="convert-btn convert-btn--running" disabled type="button">
          {#if convertProgress}
            Converting… {convertProgress.converted} / {convertProgress.totalEligible}
          {:else}
            Converting…
          {/if}
        </button>
      {:else if convertState === 'done'}
        <p class="convert-result">
          {#if convertResult && convertResult.converted === 0}
            Nothing to convert
          {:else if convertResult}
            Converted {convertResult.converted} file{convertResult.converted !== 1 ? 's' : ''}
            {#if convertResult.skipped > 0}({convertResult.skipped} skipped){/if}
            {#if convertResult.failed > 0}· {convertResult.failed} failed{/if}
          {/if}
        </p>
      {:else if convertState === 'error'}
        <p class="convert-error" role="alert">{convertError}</p>
      {/if}
    </div>
  </div>
{/if}
```

#### New CSS classes to add:

```css
.output-actions {
  display: flex;
  flex-direction: column;
  gap: 0.5rem;
  align-items: flex-end;
  flex-shrink: 0;
}

.convert-btn {
  background: rgba(255, 255, 255, 0.7);
  color: var(--accent-primary);
  border: 1px solid rgba(53, 91, 70, 0.32);
  border-radius: 999px;
  padding: 0.58rem 1rem;
  font-size: 0.88rem;
  font-weight: 600;
  cursor: pointer;
  white-space: nowrap;
}

.convert-btn:hover:not(:disabled) {
  background: rgba(255, 255, 255, 0.92);
}

.convert-btn--running {
  opacity: 0.65;
  cursor: default;
}

.convert-result {
  margin: 0;
  color: var(--state-success);
  font-size: 0.85rem;
  font-weight: 600;
  text-align: right;
}

.convert-error {
  margin: 0;
  color: var(--state-error);
  font-size: 0.82rem;
  text-align: right;
}
```

---

## Tests

### Backend unit tests — `downloader-app/src-tauri/src/commands.rs`

The Tauri command itself is hard to unit-test directly (requires an AppHandle). Test the two pure helpers extracted from the command body:

- `find_chrome_binary_in_app(None)` with macOS Chrome present → returns the macOS path.
- `find_chrome_binary_in_app(Some("/custom/path"))` → returns the custom path unchanged.
- `collect_html_files_for_convert(&dir)` with a mix of `.html`, `.pdf`, `.json` files → returns only `.html`, sorted.
- `useless_host_for_convert(&json_path)` with a ScienceDirect URL → returns `Some(host)`.
- `useless_host_for_convert(&json_path)` with an openreview.net URL → returns `None`.

These helpers should be extracted as private `pub(crate)` functions, same pattern used in `downloader-cli/src/commands/convert.rs`.

The stubbed-Chrome convert test (writing a shell script stub, asserting the PDF is created) from the CLI tests is the clearest pattern — replicate it here as an async `#[test]` on the inner logic, bypassing the Tauri AppHandle by extracting a `run_convert_inner(corpus_dir, no_skip, chrome, progress_fn: impl Fn(ConvertProgress))` helper that the command calls and the test calls directly.

### Frontend tests — `CompletionSummary.test.ts`

The existing test file uses Svelte testing library. Add:

- Render with a summary that has `output_dir` set → "Convert HTML → PDF" button is visible.
- Render with `output_dir` absent → button is not rendered.
- Click the button → button transitions to "Converting…" and is disabled (mock `invoke` to return a pending promise).
- `invoke` resolves with `{ converted: 3, skipped: 1, failed: 0, total: 4 }` → result text "Converted 3 files (1 skipped)" appears.
- `invoke` resolves with `{ converted: 0, skipped: 0, failed: 0, total: 0 }` → "Nothing to convert" appears.
- `invoke` rejects with "Chrome not found…" → error text appears.
- Clicking "Download more" after a completed conversion resets convert state → button returns to "Convert HTML → PDF".

---

## Critical Files

| File | Change |
|---|---|
| `downloader-app/src-tauri/src/commands.rs` | Add `ConvertResult`, `ConvertProgress`, `convert_html_files` command, helper fns |
| `downloader-app/src-tauri/src/lib.rs` | Register `commands::convert_html_files` in `generate_handler!` |
| `downloader-app/src/lib/CompletionSummary.svelte` | Add convert button, progress state, result/error display |
| `downloader-app/src/lib/CompletionSummary.test.ts` | Add convert button tests |

**Not changed:** `downloader-cli/src/commands/convert.rs` — the CLI command is complete and not modified by this story.

---

## Verification

```bash
# 1. Build and start the app
cd downloader-app
npm run tauri dev

# 2. Download a corpus that includes HTML results
#    (or point it at an existing corpus dir with HTML files)

# 3. On the Completion Summary screen:
#    - Confirm "Convert HTML → PDF" button is visible
#    - Click it; confirm it transitions to "Converting… N / M"
#    - Confirm it resolves to "Converted N files (S skipped)"
#    - Confirm the PDFs exist next to the HTML files
#    - Confirm original .html files are preserved

# 4. Click "Download more", re-trigger — confirm button is back to idle state

# 5. Test the Chrome-not-found error path:
DOWNLOADER_CHROME_BINARY=/nonexistent ./target/debug/downloader-app
# → clicking Convert should show the "Chrome not found" error inline

# 6. Run tests
cargo test -p downloader-app
npm run test --prefix downloader-app
```

---

## Tasks / Subtasks

### Review Follow-ups (AI)

- [ ] [AI-Audit][High] Verify `tokio` in `downloader-app/Cargo.toml` explicitly enables the `process` feature (e.g. `tokio = { version = "1", features = ["full"] }` or `features = ["process", ...]`). Without this, `tokio::process::Command` won't compile in the app crate even if tokio is available transitively.
- [ ] [AI-Audit][High] Fix the `listen` / unlisten pattern in `CompletionSummary.svelte` for Svelte 5 runes. `listen(...)` returns `Promise<UnlistenFn>` — you cannot `await` inside `$effect` (cleanup must be synchronous). Use `onMount(async () => { const unlisten = await listen(...); return () => unlisten(); })` or store the promise and resolve in cleanup. Verify no listener leak occurs across re-renders.
- [ ] [AI-Audit][Medium] Add a test in `run_convert_inner` that verifies the `on_progress` callback fires exactly `N` times (once per eligible file, skipped-by-sibling-PDF files excluded). The existing CLI test pattern with stubbed Chrome can be extended with a counter closure.
- [ ] [AI-Audit][Medium] Fix the "Nothing to convert" display condition. Currently `convertResult.converted === 0` triggers "Nothing to convert" even when `failed > 0`. Change to `convertResult.converted === 0 && convertResult.failed === 0` (or use `convertResult.total === 0`). Make the `total` field load-bearing in the template.
- [ ] [AI-Audit][Medium] Add a frontend test: render with `completed=0, failed=2, skipped_duplicates=0, output_dir` set → the convert button (and the entire `.output-block`) must NOT be rendered. The existing test only checks the `output_dir` absent case.
- [ ] [AI-Audit][Medium] Specify (in a comment or a doc note) the "convert running, user clicks Download more" behavior. If the background Tauri task should be cancellable, add a cancel path. If not, consider disabling "Download more" while `convertState === 'running'`, or accepting the silent continue-to-completion behavior and document it.

---

## Senior Developer Review (AI)

**Review date:** 2026-05-05
**Status:** ✅ Approved — done
**Issues found:** 0 High · 2 Medium · 4 Low
**Fixed:** 2 Medium (M-1 tracing, M-2 reset test assertion)
**Action items created:** 0

### Fixes applied

**M-1 fixed** (`commands.rs` `run_convert_inner`): Added `warn!` logging for both non-zero Chrome exit and spawn failures, matching the CLI pattern. Users can now diagnose per-file conversion failures via tracing output.

**M-2 fixed** (`CompletionSummary.test.ts`): Strengthened the reset test — after clicking "Download more", the test now asserts the "Convert HTML → PDF" button is visible again, confirming `convertState` returned to `idle`.

### Low issues deferred (no action required)

- L-1: Blocking I/O in async context (`which`, `read_dir`) — consistent with CLI; acceptable for desktop app
- L-2: `--no-sandbox` Chrome flag — consistent with CLI; acceptable for first implementation
- L-3: `listen` unlisten race window — negligible in practice; documented in code comment
- L-4: `Cargo.toml`/`Cargo.lock` not listed in Critical Files — documentation only

---

## Party Mode Audit (AI)

**Audit date:** 2026-05-05
**Outcome:** pass_with_actions
**Experts:** PM/Product · Architect · QA/Dev

**Findings: 2 High, 4 Medium, 2 Low**

| # | Severity | Area | Finding |
|---|---|---|---|
| 1 | High | Arch | `tokio::process::Command` requires the `process` tokio feature — must be explicit in the app Cargo.toml or build fails |
| 2 | High | QA | Svelte 5 `$effect` cleanup is synchronous; `await listen(...)` inside it is invalid — listener leak or silent no-op |
| 3 | Medium | QA | `on_progress` callback call-count is untested — skipped files (sibling PDF) must not trigger it |
| 4 | Medium | QA | "Nothing to convert" shown even when `failed > 0` — `total` field unused in template despite being in `ConvertResult` |
| 5 | Medium | QA | Missing frontend test for "completed=0, failed=2 → no convert button" |
| 6 | Medium | PM | No specified behavior when user clicks "Download more" during an in-progress conversion (background task continues orphaned) |
| 7 | Low | Arch | Three logic blocks duplicated from CLI (`find_chrome_binary`, `collect_html_files`, `useless_host` + constant) — no divergence guard |
| 8 | Low | PM | Doc inconsistency: test case `converted:3, skipped:1` but example result text says "Converted 4 files (1 skipped)" |
