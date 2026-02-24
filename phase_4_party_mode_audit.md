# Party Mode Audit (AI): Phase 4 Plan — progress_manager + queue_manager

**Audit date:** 2026-02-19  
**Target:** Phase 4 implementation plan (queue_manager, progress_manager)  
**Outcome:** pass_with_actions  

**Summary counts:** High: 0 | Medium: 3 | Low: 4  

---

## Parties invited (7)

1. **Product / PM** — Scope and user impact  
2. **Architect** — Boundaries, dependencies, and layering  
3. **QA / TEA** — Testability and regression  
4. **Developer** — Implementation clarity and sequencing  
5. **DevOps / CI** — Build and test stability  
6. **Security** — Sensitive paths and logging  
7. **Tech writer** — Naming and documentation  

---

## Findings

### High

- None.

### Medium

- **[Architect] progress_manager Role vs Notes mismatch**  
  **Evidence:** Section 2 "What Phase 4 Does" says progress_manager "decides whether to show it (using terminal conditions)". Section 2 "Notes" say runtime calls `terminal::should_use_spinner(...)` and passes the result as `use_spinner`; "terminal remains the authority".  
  **Risk:** Implementer might move the "should we use spinner?" decision into progress_manager and duplicate or bypass `terminal::should_use_spinner`.  
  **Follow-up:** In the plan, clarify in Section 2 "Role" that progress_manager does **not** decide: it receives `use_spinner: bool` from runtime (runtime calls `terminal::should_use_spinner`). One-line fix: "Provide a single entry point to spawn the progress UI when **requested** (runtime decides via terminal::should_use_spinner)."

- **[Developer] Return order of spawn_progress_ui**  
  **Evidence:** Section 2 Public API returns `(Option<JoinHandle<()>>, Arc<AtomicBool>)`. Section 3 step 3 uses `let (progress_handle, progress_stop) = progress_manager::spawn_progress_ui(...)`. So order is (handle, stop).  
  **Risk:** If the implementation returns (stop, handle) by mistake, the caller would store(true) on the join handle and await the AtomicBool.  
  **Follow-up:** In the plan, state explicitly: "Return order is (handle, stop). Caller uses: progress_stop.store(true); if let Some(h) = progress_handle { h.await; }."

- **[QA / TEA] queue_manager: no test for create_queue failure paths**  
  **Evidence:** Plan Section 5 suggests unit test for create_queue with temp dir and non-negative history_start_id. No mention of testing failure cases (e.g. output_dir not writable, db_path on read-only fs).  
  **Risk:** Refactors might break error propagation; no test would catch it.  
  **Follow-up:** Add to Section 5: "Optional: test that create_queue returns an error when output_dir is not writable (or skip and rely on integration tests)." So the plan explicitly allows skipping failure-path tests but notes the option.

### Low

- **[Tech writer] Naming: queue_manager vs progress_manager**  
  **Evidence:** queue_manager "creates" the queue; progress_manager "spawns" progress UI.  
  **Risk:** None; names are clear.  
  **Follow-up:** None.

- **[Security] state_dir and db_path in logs**  
  **Evidence:** queue_manager logs "Recovered interrupted queue items"; no plan to log state_dir or db_path.  
  **Risk:** If someone adds debug!(path = %db_path) later, paths could leak.  
  **Follow-up:** Add one line to queue_manager notes: "Do not log state_dir or db_path in debug; they can reveal user layout."

- **[PM] output_dir existence**  
  **Evidence:** Plan says "Caller is responsible for ensuring output_dir exists". Runtime already creates output_dir before any queue creation.  
  **Risk:** None.  
  **Follow-up:** None.

- **[DevOps] runtime still needs AtomicBool and Ordering**  
  **Evidence:** Section 3 step 5 says "Arc, AtomicBool, Ordering remain where still used". After refactor, runtime uses progress_stop.store(true) and interrupted for ctrl_c; progress_handle.await. So AtomicBool and Ordering stay in runtime.  
  **Risk:** If imports are over-trimmed, build will fail.  
  **Follow-up:** In Section 3 step 1, list explicitly: "Keep: Arc, AtomicBool, Ordering (for progress_stop and interrupted)."

---

## Review follow-ups (AI)

Tasks to apply to the plan or implementation:

- [ ] [AI-Audit][Medium] Clarify in Section 2 (progress_manager) that runtime decides spinner via terminal::should_use_spinner; progress_manager only implements when requested.
- [ ] [AI-Audit][Medium] State explicitly in plan that spawn_progress_ui return order is (handle, stop) and show caller usage.
- [ ] [AI-Audit][Medium] In Section 5 (Testing), add optional note for queue_manager failure-path test or explicit skip.
- [ ] [AI-Audit][Low] In queue_manager notes, add: do not log state_dir or db_path in debug.
- [ ] [AI-Audit][Low] In Section 3 step 1 (runtime imports), list "Keep: Arc, AtomicBool, Ordering".

---

**Audit complete.** Outcome: **pass_with_actions**.  

The Phase 4 plan (`.cursor/plans/phase_4_progress_and_queue_managers.plan.md`) has been updated to incorporate the above follow-ups: progress_manager role/notes alignment, explicit (handle, stop) return order and caller usage, optional queue_manager failure-path test note, queue_manager “do not log state_dir/db_path” note, and runtime imports (Arc, AtomicBool, Ordering) kept explicit.
