# Complexity Refactor Roadmap Status

Date: 2026-02-19

## Todo status

| ID | Status | Notes |
| --- | --- | --- |
| `phase0-governance` | completed | Owner roles, approvals, and escalation path documented. |
| `phase0-artifacts` | completed | Owner matrix, gate checklist template, and cadence protocol added under `docs/complexity-refactor/`. |
| `baseline-tests` | completed | Existing integration coverage retained; added failed-history output contract in `tests/cli_e2e.rs` and validated targeted suites. |
| `main-runtime-shell` | completed | Runtime orchestration moved from `src/main.rs` into `src/app/runtime.rs`. |
| `extract-low-risk-helpers` | completed | Config/runtime/terminal/validation helpers extracted to `src/app/`. |
| `extract-self-contained-flows` | completed | Cookie runtime flow extracted to `src/auth/runtime_cookies.rs`; dry-run flow extracted to `src/commands/dry_run.rs`. |
| `split-reporting-project` | completed | Summary/history rendering moved to `src/output/mod.rs`; project artifact rendering moved to `src/project/mod.rs`. |
| `decompose-download-engine` | completed | Engine execution flow split into focused modules: `src/download/engine/task.rs`, `src/download/engine/persistence.rs`, `src/download/engine/error_mapping.rs`. |
| `introduce-db-seams` | completed | Added repository trait seam via `src/queue/repository.rs` with `QueueRepository` implementation for `Queue`; engine internals now depend on repository-bound helpers. |
| `harden-error-model` | completed | Queue DB errors now expose typed kinds in `src/queue/error.rs`; history failure rendering/suggestions in `src/failure/mod.rs` now prioritize typed error categories with message fallback only. |
| `nonfunctional-regression-gates` | completed | Added non-functional regression gate suite in `tests/nonfunctional_regression_gates.rs` and calibration/runbook in `docs/complexity-refactor/nonfunctional-regression-gates.md`. |
| `rollback-checkpoints` | completed | Rollback and freeze criteria documented in `docs/complexity-refactor/rollback-strategy.md`. |
| `finalize-kpi-thresholds` | completed | Baseline thresholds accepted and tracked as gate criteria. |
| `phase-rollout-gates` | in_progress | Audit follow-up updated (2026-02-19): formatting drift resolved, `download::client` proxy-panic fallback hardened, and non-strict local gates (`fmt`/`clippy`/`cargo test --all-targets`) are green. Phase remains `HOLD` pending strict socket-gate pass on a bind-capable CI runner; evidence in `docs/complexity-refactor/phase-rollout-gates-audit-followup.md`. |
