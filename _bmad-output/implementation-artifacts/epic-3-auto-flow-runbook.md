# Epic 3 Auto Flow Runbook

## Purpose

Run Epic 3 stories automatically through:

`create-story -> party-mode-audit -> dev-story -> code-review`

using the `epic-auto-flow` workflow.

## Command

Run:

`/bmad:bmm:workflows:epic-auto-flow`

Default target epic is `3`.

## Preconditions

1. `_bmad-output/implementation-artifacts/sprint-status.yaml` exists and is valid.
2. Epic 3 stories are present in `development_status`.
3. Story files are in `_bmad-output/implementation-artifacts/`.
4. `code-review` supports `review_mode=auto-fix-high-medium` (already configured).

## Runtime Behavior

1. Select first non-done Epic 3 story with priority:
`in-progress`, `review`, `ready-for-dev`, `backlog`.
2. If selected story is `backlog`, run `create-story`.
3. Run `party-mode-audit` for the selected story.
4. Run `dev-story` for the selected story.
5. Run `code-review` with auto-fix policy.
6. Verify story status becomes `done`.
7. Repeat until all Epic 3 stories are `done`.

## Failure Mode (Fail-Fast)

On stage failure, pipeline stops immediately and writes a failure report:

`_bmad-output/implementation-artifacts/epic-3-auto-flow-failure-<date>.md`

Report includes:

1. failing story key
2. failing stage
3. reason
4. resume action

## Resume Procedure

1. Open the latest failure report and resolve the blocking issue.
2. Confirm affected story and sprint status are consistent.
3. Re-run:
`/bmad:bmm:workflows:epic-auto-flow`

The flow resumes from current sprint/story state.

## Operational Notes

1. Scope is restricted to Epic 3 stories only.
2. Stories already marked `done` are skipped.
3. `party-mode-audit` writes findings and `[AI-Audit]` follow-up tasks into the story file.
4. `code-review` runs non-interactively in auto-fix mode for High/Medium findings.

## Verification Checklist

1. All `3-*` stories in `sprint-status.yaml` are `done`.
2. `epic-3` status is updated appropriately by your normal sprint process.
3. No unresolved Epic 3 failure report remains.
