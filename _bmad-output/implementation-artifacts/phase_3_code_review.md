# Phase 3 Code Review: resolution_orchestrator, download_orchestrator, runtime

## Summary

The implementation matches the plan: resolution and download are in separate orchestrators, runtime is a thin coordinator, ordering and contracts are preserved. Fixes applied: URL redaction in resolution logs, enqueue debug no longer logs URL, and logging consistency in download_orchestrator.

---

## resolution_orchestrator.rs

**Correctness**
- `run_resolution` returns zeros when `ctx.input_text` is `None`; otherwise parses, logs counts, builds registry/topics, resolves each item, enqueues with metadata. Logic matches the original runtime block.
- `ResolutionOutcome` has the three required fields; runtime uses them correctly for the "all failed" bail and early return.
- Empty `parse_result` returns early with `parsed_item_count` and zero resolution counts.

**Security (plan: no cookie/session-correlatable URLs in logs) — fixes applied**
- Module doc and implementation avoid logging `cookie_jar`.
- Duplicate-URL skip logs only `"Skipping duplicate URL already in queue"` (no URL).
- Resolver-metadata debug logs only `metadata_fields = resolved.metadata.len()` (no input/URL).
- **Fixed:** "Skipped unresolved parsed item" `warn!` no longer logs raw URLs: when `input_type == InputType::Url` we log `"(url redacted)"`; for DOI/Reference/BibTex we log a truncated preview (80 chars) to avoid huge logs while keeping debuggability.
- **Fixed:** "Enqueued parsed item" `debug!` no longer logs `value = %queue_value` (resolved URL); it logs only `input_type` and `source_type`.

**Tests**
- `run_resolution_with_no_input_returns_zeros` asserts all three outcome fields; uses `RunContext` with `input_text: None`. Satisfies the required contract test.

---

## download_orchestrator.rs

**Correctness**
- Client (with optional cookie jar and timeouts), retry policy, rate limiter (disabled / jitter / plain), engine, and robots cache match original runtime.
- `process_queue_interruptible_with_options` is called with the same arguments; `EngineError` is mapped with `.context(...)` so the chain is preserved for CI.

**Consistency — fix applied**
- Uses `use tracing::debug` and `debug!(...)` (same pattern as resolution_orchestrator and runtime).

---

## runtime.rs

**Ordering**
- `history_start_id` right after queue creation.
- `run_resolution` then bail when all parsed items failed resolution, then `pending_items` / `total_queued` / `uncertain_references_in_run`, then early return when `total_queued == 0`, then `completed_before`.
- `run_download` after interrupted/spinner setup; `progress_stop.store(true)` and join after `run_download`.
- `completed_before` and `history_start_id` are used correctly for sidecars and project append.

**Behavior**
- Same dry-run, quick-start, output dir, state dir, and queue creation as before.
- Completion summary, sidecar generation, interrupt warning, project append, and exit outcome use the same inputs and conditions.

**Inspection notes**
- Progress spinner: `let _ = handle.await` swallows JoinError if the spinner task panics; acceptable (spinner is best-effort). No change made.
- Imports: only `Database`, `Queue`, `QueueStatus` from `downloader_core`; no unused imports.

---

## Verification

- `cargo build --bin downloader`: ok
- `cargo test --bin downloader`: 227 tests passed

No further follow-ups required for Phase 3.
