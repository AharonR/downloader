---
name: ""
overview: ""
todos: []
isProject: false
---

# Phase 3: Resolution and Download Orchestrators

**Single source of truth for Phase 3 implementation.**  
Includes plan, implementation steps, and audit-derived constraints.  
Last updated: 2026-02-19.

---

## What Phase 3 Does

Phase 3 splits the main download flow in `runtime.rs` into two new modules:

1. **resolution_orchestrator** — Parses user input (URLs, DOIs, references), resolves each item to a download URL, and enqueues them with metadata. It only reports counts and the first error; it does not run downloads.
2. **download_orchestrator** — Builds the HTTP client, retry/rate-limit settings, and download engine, then runs the queue until done. It returns download statistics.

`runtime.rs` stays as the thin coordinator: it creates the queue and DB, calls resolution then download, then handles completion summary, sidecars, project append, and exit code. No change to user-visible behavior.

---

## Conventions and Constraints (from audit)

- **Security:** Do not log `cookie_jar` contents or URLs that could identify an authenticated session. In resolution_orchestrator, limit debug logs to counts and non-sensitive metadata.
- **Async:** `run_resolution` is an **async** function (it awaits resolver and queue calls). The plan and code must use `async fn run_resolution(...)` and `.await` at the call site.
- **Ordering:** In runtime, compute `history_start_id` and `completed_before` in the same place as today: `history_start_id` right after queue creation, `completed_before` after resolution and before starting downloads. These baselines are required for sidecar generation and project append.
- **Naming:** `ResolutionOutcome` holds resolution-phase counts only. `DownloadStats` comes from download_core and is used for exit code and completion summary.
- **Errors:** When mapping `EngineError` to `anyhow::Error` in download_orchestrator, use `.context(...)` or equivalent so the original error variant remains in the chain for CI and debugging.

---

## 1. resolution_orchestrator

**New file:** `src/app/resolution_orchestrator.rs`

### Role

Parse input text, resolve each item to a URL (via the resolver registry), optionally extract topics and load custom topics, enqueue items with metadata. Return how many items were parsed, how many resolution failed, and the first resolution error message (so runtime can bail when *all* fail).

### Types and function

- **ResolutionOutcome**  
  - `parsed_item_count: usize`  
  - `resolution_failed_count: usize`  
  - `first_resolution_error: Option<String>`
- **async fn run_resolution(ctx: &RunContext, queue: Arc) -> Result****
  - If `ctx.input_text` is `None`, return `Ok(ResolutionOutcome { parsed_item_count: 0, resolution_failed_count: 0, first_resolution_error: None })`.
  - Otherwise: parse input, log parse feedback, build resolver registry and resolve context, set up optional topic extractor and custom topics (same logic as current runtime), then for each parsed item resolve to URL and enqueue with metadata (skip if URL already in queue). Fill the three fields of `ResolutionOutcome` and return it.

### Implementation notes (match current runtime.rs ~lines 102–274)

- Use: `downloader_core::{parse_input, build_default_resolver_registry, ResolveContext, build_preferred_filename, extract_reference_confidence, load_custom_topics, match_custom_topics, normalize_topics}`, `TopicExtractor`, `crate::output::log_parse_feedback`, `context::RunContext`, `Arc<Queue>`, and queue methods `has_active_url`, `enqueue_with_metadata`.
- Topic extractor: `if ctx.args.detect_topics { TopicExtractor::new().ok() } else { None }`.
- Custom topics: if `ctx.args.topics_file` is set, call `load_custom_topics(path)` or `bail!(...)` with the same message as today; else `Vec::new()`.
- Per item: resolve via registry; on error increment `resolution_failed_count` and set `first_resolution_error` once; on success build `QueueMetadata` (suggested_filename, title, authors, year, doi, topics, parse_confidence, parse_confidence_factors) and call `enqueue_with_metadata`. Skip when `queue.has_active_url(&queue_value).await?`.
- When `resolution_failed_count > 0`, log the existing "Skipped parsed items that could not be resolved" message.

### Module wiring

In `src/app/mod.rs`, add (alphabetically):  
`pub(crate) mod resolution_orchestrator;`

---

## 2. download_orchestrator

**New file:** `src/app/download_orchestrator.rs`

### Role

Build the HTTP client (with optional cookie jar and timeouts), retry policy, rate limiter, download engine, and optional robots cache; call `engine.process_queue_interruptible_with_options`; return `DownloadStats`. Convert `EngineError` to `anyhow::Error` using `.context(...)` (or equivalent) so CI and logs keep the original error information.

### Function

- **run_download(ctx: &RunContext, queue: Arc, interrupted: Arc) -> Result****
  - Build `HttpClient` from `ctx.cookie_jar` and `ctx.http_timeouts` (same logic as current runtime).
  - Build `RetryPolicy::with_max_attempts(u32::from(ctx.args.max_retries))`.
  - Build `RateLimiter`: disabled if `ctx.args.rate_limit == 0`, with jitter if `ctx.args.rate_limit_jitter > 0`, else plain (same as runtime ~lines 291–303).
  - Build `DownloadEngine::new(usize::from(ctx.args.concurrency), retry_policy, rate_limiter)?`; map `EngineError` to anyhow.
  - If `ctx.args.check_robots`, build `Some(Arc::new(RobotsCache::new()))`; else `None`.
  - Call `engine.process_queue_interruptible_with_options(queue.as_ref(), &client, &ctx.output_dir, interrupted, QueueProcessingOptions { generate_sidecars: ctx.args.sidecar, check_robots: ctx.args.check_robots, robots_cache })?`.
  - Return `Ok(stats)`.

### Module wiring

In `src/app/mod.rs`, add (alphabetically):  
`pub(crate) mod download_orchestrator;`

---

## 3. runtime.rs refactor

### Purpose

Replace the inline resolution block and the inline download block with calls to the two orchestrators. Keep in runtime: queue/DB creation, ctrl_c handling, progress spinner, completion summary, sidecars, project append, and exit logic.

### Required ordering

- Right after queue creation: `history_start_id = queue.latest_download_attempt_id().await?`.
- After resolution, before download: `completed_before = queue.list_by_status(QueueStatus::Completed).await?` (and collect into `HashSet<i64>` as today). These two baselines are used later for sidecar scope and project append; do not move them after `run_download`.

### Steps

1. **Imports**
  Add `resolution_orchestrator` and `download_orchestrator`. Keep only the download_core symbols runtime still needs (e.g. `Database`, `Queue`, `QueueStatus`). Remove resolution/parser/topic imports that now live in resolution_orchestrator.
2. **Up to queue creation**
  No change to dry-run, quick-start, output dir, state_dir, or has_prior_state logic.
3. **Queue and DB**
  Unchanged. Then compute and keep `history_start_id` as above.
4. **Resolution**
  Replace the whole `if let Some(input_text) { ... }` block with:
  - `let resolution = resolution_orchestrator::run_resolution(&ctx, Arc::clone(&queue)).await?;`
  - If `resolution.parsed_item_count > 0` and `resolution.resolution_failed_count == resolution.parsed_item_count`, bail with the same message as today, using `resolution.first_resolution_error`.
  - `let pending_items = queue.list_by_status(QueueStatus::Pending).await?;`  
  `let total_queued = pending_items.len();`  
  Compute `uncertain_references_in_run` from `pending_items` as today.  
  If `total_queued == 0`, log and `return Ok(ProcessExit::Success)`.
5. **Baselines for sidecar/append**
  Compute `completed_before` from `queue.list_by_status(QueueStatus::Completed)` as today (before starting downloads).
6. **Download**
  Replace the client/engine/process_queue block with:
  - Create `interrupted` and spawn ctrl_c (unchanged).
  - Create `progress_stop` and `progress_handle` (spinner) as today.
  - `let stats = download_orchestrator::run_download(&ctx, Arc::clone(&queue), Arc::clone(&interrupted))?;`
  - Set `progress_stop` and await the progress handle (unchanged).
7. **After download**
  Keep: "Download complete" log, `output::print_completion_summary`, sidecar generation (when `ctx.args.sidecar`), interrupt warning, project append (when `ctx.args.project`), and `exit_handler::determine_exit_outcome(stats.completed(), stats.failed())`. Ensure `total_queued` and `uncertain_references_in_run` are still in scope.
8. **Spinner**
  Leave `spawn_spinner` in runtime; Phase 4 will move it to progress_manager.

Behavior, error messages, and logs must remain the same as today.

---

## 4. Dependencies and visibility

- **resolution_orchestrator:** Uses `crate::app::context`, `crate::output`, `downloader_core` (parser, resolver, topics, queue), `std::sync::Arc`, `anyhow`. Does not depend on runtime or download_orchestrator.
- **download_orchestrator:** Uses `crate::app::context`, `downloader_core` (download, queue), `std::sync::Arc`, `std::sync::atomic::AtomicBool`, `anyhow`. Does not depend on runtime or resolution_orchestrator.
- **runtime:** Uses both orchestrators, context, config_manager, command_dispatcher, config_runtime, exit_handler, input_processor, terminal, output, project, commands, and Database/Queue for creation and listing.

All new code is `pub(crate)`; no new public API.

---

## 5. Testing

- **Required:** One unit test for resolution_orchestrator: when `run_resolution` is called with a context that has `input_text: None`, the result must be `Ok(ResolutionOutcome { parsed_item_count: 0, resolution_failed_count: 0, first_resolution_error: None })`. This protects the runtime’s "all failed" and "no items enqueued" logic.
- **Regression:** Run `cargo test --bin downloader` and full `cargo test`; optionally run a quick manual or E2E check. No other new tests are required for Phase 3.

---

## 6. Implementation order

1. Add `src/app/resolution_orchestrator.rs` with `ResolutionOutcome` and `async fn run_resolution`; add `pub(crate) mod resolution_orchestrator;` in `src/app/mod.rs`.
2. In `runtime.rs`, call `run_resolution`.await, remove the inline resolution block, and add outcome handling plus pending/total_queued (and `completed_before`) as above. Run `cargo build` and `cargo test`.
3. Add `src/app/download_orchestrator.rs` with `run_download`; add `pub(crate) mod download_orchestrator;` in `src/app/mod.rs`.
4. In `runtime.rs`, replace the client/engine/process_queue block with `run_download`, keeping ctrl_c and spinner. Run `cargo build` and `cargo test`.
5. Clean up unused imports in runtime and run the full test suite.

---

## 7. Out of scope for Phase 3

- Moving queue/DB creation into queue_manager (Phase 4).
- Moving the progress spinner into progress_manager (Phase 4).
- Changing config_runtime, validation, or other existing modules’ contracts.

