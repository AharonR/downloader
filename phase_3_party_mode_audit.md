# Party Mode Audit (AI): Phase 3 Plan — resolution_orchestrator + download_orchestrator

**Audit date:** 2026-02-19  
**Target:** Phase 3 implementation plan (RunContext + config_manager + input_processor already done; Phase 3 = resolution + download orchestrators)  
**Outcome:** pass_with_actions  

**Summary counts:** High: 1 | Medium: 3 | Low: 4  

---

## Parties invited (7)

1. **Product / PM** – acceptance and user-value risks  
2. **Architect** – boundaries, dependencies, layering  
3. **QA / TEA** – tests, edge cases, regression  
4. **Developer** – implementation clarity, sequencing  
5. **DevOps / CI** – build and test stability  
6. **Security** – sensitive data and logging  
7. **Tech writer** – naming and maintainability  

---

## Findings

### High

- **[Security] Cookie jar and paths in resolution_orchestrator**  
  **Evidence:** Plan Section 1: resolution_orchestrator receives `ctx: &RunContext`, which includes `ctx.cookie_jar`. The resolution loop uses `build_default_resolver_registry(ctx.cookie_jar.clone(), ...)`. No explicit rule against logging resolver input or metadata that might carry session identifiers.  
  **Risk:** If debug logging is added later in the orchestrator (e.g. per-item resolve), URLs or metadata could be logged; cookie jar itself must never be logged.  
  **Follow-up:** Add one explicit line to the plan: "Do not log cookie_jar contents or URLs that could correlate with authenticated sessions; limit debug logs to counts and non-sensitive metadata."

### Medium

- **[Architect] run_resolution async vs sync**  
  **Evidence:** Plan Section 1: `run_resolution(ctx, queue) -> Result<ResolutionOutcome>`. Current runtime uses `.await` on `resolve_to_url`, `has_active_url`, `enqueue_with_metadata`.  
  **Risk:** If the plan does not state that `run_resolution` is `async fn`, implementers might write it as sync and hit compile errors or deadlocks.  
  **Follow-up:** In Section 1 Public API, state: `run_resolution(ctx: &RunContext, queue: Arc<Queue>) -> impl Future<Output = Result<ResolutionOutcome>>` or explicitly "async fn run_resolution(...) -> Result<ResolutionOutcome>".

- **[QA / TEA] No contract test for ResolutionOutcome**  
  **Evidence:** Plan Section 5: "Unit tests: Optional for Phase 3: a test that run_resolution with input_text: None returns outcome with zeros."  
  **Risk:** Refactors could change the meaning of "no input" (e.g. return non-zero parsed_item_count) and break runtime's "all failed" bail logic.  
  **Follow-up:** Make the "run_resolution with input_text: None returns zeros" test **required** in the plan, not optional, and add one line: "Assert ResolutionOutcome { parsed_item_count: 0, resolution_failed_count: 0, first_resolution_error: None }."

- **[Developer] completed_before and history_start_id placement**  
  **Evidence:** Plan Section 3: runtime refactor keeps "Rest: Keep info Download complete, output::print_completion_summary, sidecar generation ... project append (history_start_id)". Current runtime computes `completed_before` and `history_start_id` **before** the resolution block (lines 307–313 and 101).  
  **Risk:** If the refactor moves or reorders code, `completed_before` must still be computed **before** `run_download` and `history_start_id` before any queue mutation from resolution. Plan does not state this ordering explicitly.  
  **Follow-up:** In Section 3, add: "Preserve ordering: history_start_id = queue.latest_download_attempt_id().await? and completed_before = queue.list_by_status(Completed)... must remain before run_resolution/run_download so sidecar and project append use correct baselines."

### Low

- **[Tech writer] Naming: ResolutionOutcome vs DownloadStats**  
  **Evidence:** Plan uses ResolutionOutcome (new) and DownloadStats (existing).  
  **Risk:** Minor: "Outcome" vs "Stats" is slightly inconsistent; acceptable if documented.  
  **Follow-up:** Add a one-line note in Section 1 or 4: "ResolutionOutcome holds resolution-phase counts only; DownloadStats is from download_core and used for exit and summary."

- **[DevOps / CI] EngineError mapping**  
  **Evidence:** Plan Section 2: "Map EngineError to anyhow::Error so the orchestrator returns Result<DownloadStats>."  
  **Risk:** If mapping loses context (e.g. only .to_string()), CI logs may be harder to diagnose.  
  **Follow-up:** Recommend in plan: "Map via anyhow::Error::msg(e) or .context(...) so engine error variant is preserved in error chain."

- **[PM] No user-visible behavior change**  
  **Evidence:** Plan Section 3: "No behavioral change: Same order of operations, same error messages, same logs."  
  **Risk:** None; this is correct.  
  **Follow-up:** None beyond keeping this line in the plan.

- **[Architect] Queue ownership**  
  **Evidence:** runtime creates queue and passes `Arc::clone(&queue)` to both orchestrators.  
  **Risk:** None if clone is used consistently.  
  **Follow-up:** None.

---

## Review follow-ups (AI)

Tasks to add to the plan or to implementation checklist:

- [ ] [AI-Audit][High] Add explicit security note to plan: do not log cookie_jar or session-correlatable URLs in resolution_orchestrator.
- [ ] [AI-Audit][Medium] Specify in plan that run_resolution is async (async fn or explicit Future).
- [ ] [AI-Audit][Medium] Require (not optional) unit test: run_resolution with input_text None returns ResolutionOutcome with all zeros.
- [ ] [AI-Audit][Medium] Add ordering constraint to plan: history_start_id and completed_before computed before run_resolution/run_download.
- [ ] [AI-Audit][Low] Add one-line naming note: ResolutionOutcome vs DownloadStats.
- [ ] [AI-Audit][Low] Recommend EngineError mapped with context for CI diagnostics.

---

**Audit complete.** Use this plan for implementation only when High/Medium follow-ups are addressed or explicitly accepted.
