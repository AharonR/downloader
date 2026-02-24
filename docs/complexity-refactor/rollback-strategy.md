# Complexity Refactor Rollback Strategy

Date: 2026-02-19

## Rollback checkpoints

- One phase per PR. Each PR must stay independently revertible.
- Rollback decision owner: Tech Lead, with Engineering Manager escalation for production-impacting failures.
- Rollback candidate list:
  - Runtime shell extraction (`src/app/runtime.rs` + delegating `src/main.rs`)
  - Helper extraction (`src/app/*`, `src/auth/runtime_cookies.rs`, `src/commands/dry_run.rs`)
  - Output/project extraction (`src/output/mod.rs`, `src/project/mod.rs`)

## Freeze criteria

Freeze immediately when any condition is met:

- Required quality gates fail (`cargo test`, `cargo clippy -- -D warnings`).
- Queue lifecycle invariants regress (`pending -> in_progress -> completed/failed`).
- Accounting invariant regresses (`completed + failed != processed`).
- Reliability/performance KPI thresholds are exceeded.

## Rollback procedure

1. Mark the phase as `HOLD` in the phase gate checklist.
2. Revert the phase PR commit set.
3. Re-run required gates on the restored baseline.
4. Document root cause and mitigation before attempting re-entry.
