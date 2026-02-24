# Complexity Refactor Meeting Cadence

Date: 2026-02-19

## Weekly checkpoint (30 minutes)

- Attendees: DRI, QA Owner, Architect Reviewer
- Inputs:
  - Phase gate checklist status
  - KPI trend deltas (performance, reliability, complexity)
  - Open risks and dependency violations
- Outputs:
  - Proceed/hold decision for next phase
  - Assigned action items with owners and dates

## Stop-the-line protocol (ad hoc)

Trigger immediately when any of the following is true:

- Required test/clippy gate fails and cannot be fixed in-phase.
- KPI threshold is exceeded (runtime, throughput, lock incidence, flaky rate, retry ratio).
- Production risk is identified in queue lifecycle or interrupt/retry invariants.

Stop-the-line flow:

1. DRI opens incident thread and links failing evidence.
2. Engineering Manager is paged for escalation ownership.
3. Team decides `rollback` or `hold + remediation plan`.
4. Next phase work remains frozen until explicit unfreeze sign-off.

## Reporting format

- Weekly update fields:
  - Current phase
  - Completed gate items
  - KPI deltas vs baseline
  - Risks, mitigations, and owners
  - Go/hold recommendation
