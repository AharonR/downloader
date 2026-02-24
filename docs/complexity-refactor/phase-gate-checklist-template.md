# Complexity Refactor Phase Gate Checklist

Use this template for every phase PR. A phase cannot be closed until all required checks are complete.

## Header

- Phase ID:
- Date:
- PR link:
- DRI:
- QA sign-off owner:
- Architect reviewer:

## Entry criteria

- [ ] Scope is limited to one refactor phase (no mixed feature work).
- [ ] Owner matrix and escalation path confirmed.
- [ ] Baseline metrics for this phase are captured.
- [ ] Rollback plan for this phase is documented.

## Execution checks

- [ ] `cargo test` passed (required suites).
- [ ] `cargo clippy -- -D warnings` passed.
- [ ] New/updated regression tests included for touched invariants.
- [ ] Public behavior parity validated (CLI output/contracts).

## Evidence (required)

- [ ] Validation command list captured in a phase evidence note.
- [ ] Strict socket-test mode was used for gate runs (`DOWNLOADER_REQUIRE_SOCKET_TESTS=1`).
- [ ] Pass/fail outcomes for each required command are recorded with date/time.
- [ ] Gate decision is traceable to evidence artifact path.

## KPI review

- [ ] p95 runtime regression <= 7% vs baseline.
- [ ] Queue throughput regression <= 5% vs baseline.
- [ ] DB lock/busy incidence <= baseline and < 0.5%.
- [ ] Flaky rate < 1% across reruns.
- [ ] Retry-path success ratio drop <= 3 percentage points.
- [ ] `src/main.rs`/hotspot complexity target met for this phase.

## Exit decision

- Decision: `GO` | `HOLD` | `ROLLBACK`
- Summary:
- Follow-up actions:

## Sign-off

- DRI:
- QA Owner:
- Architect Reviewer:
- Engineering Manager (only for HOLD/ROLLBACK):
