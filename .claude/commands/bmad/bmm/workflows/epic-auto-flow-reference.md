# Epic Auto-Flow - Standalone Execution Reference

## 1. epic-auto-flow (auto-epic)

Run BMAD workflow `epic-auto-flow` from `_bmad/bmm/workflows/4-implementation/epic-auto-flow/workflow.yaml`.

**Target epic**: `<EPIC_KEY>` (default: Epic 3)

**Pipeline stages** (executed in order for each story):
1. **create-story** → Generate story from backlog (if status = backlog)
2. **party-mode-audit** → Multi-agent risk assessment with follow-up actions
3. **dev-story** → Implement all tasks/subtasks with tests
4. **code-review** → Adversarial review with auto-fix (high/medium severity)
5. **bug-test-writing** → Generate regression tests for bugs found in code-review
6. **context-compression** → Compress conversation (after code-review & bug-test-writing)

**Expected output**: Stage-by-stage progress for each story, showing:
- ✅ Story transitions (backlog → ready-for-dev → in-progress → review → done)
- 🔍 Audit findings appended to story file
- 💻 Implementation summary with changed files
- 🐛 Code review findings (auto-fixed or manual action needed)
- 🧪 Regression tests generated for discovered bugs
- 📦 Context compressed (2x per story to manage token usage)
- Final: `Stories processed: <COUNT>` and `All stories in Epic <EPIC_KEY> are now complete! 🎉`

**Configuration**:
- `epic_num`: "3" (change to target different epic)
- `review_mode`: "auto-fix-high-medium" (auto-fixes high/medium severity issues)
- `failure_mode`: "fail-fast" (stops on first failure, generates failure report)
- `compress_after_story`: "true" (enables compression)
- `compress_after_phases`: "code-review,bug-test-writing" (when to compress)

**Failure handling**:
- Generates failure report at `_bmad-output/implementation-artifacts/epic-<EPIC_KEY>-auto-flow-failure-<DATE>.md`
- Report includes: story key, failed stage, failure reason, resume instructions
- Resume by fixing the issue and re-running epic-auto-flow

**Prerequisites**:
- sprint-status.yaml exists (run `sprint-planning` first)
- Epic has stories in backlog/ready-for-dev/in-progress/review status
- All sub-workflows installed in `_bmad/bmm/workflows/`

**Usage**:
```
Run epic-auto-flow for Epic 3
```

---

## 2. create-story

Run BMAD workflow `create-story` from `_bmad/bmm/workflows/4-implementation/create-story/workflow.yaml`.

**Purpose**: Create the next story for epic `<EPIC_KEY>` and mark it ready-for-dev when valid.

**Expected output**:
- Created story file path (e.g., `_bmad-output/implementation-artifacts/3-2-resolver-trait-registry.md`)
- Story key (e.g., `3-2`)
- sprint-status.yaml updated with new story in ready-for-dev status

**Usage**:
```
Run create-story for Epic 3
```

---

## 3. dev-story

Run BMAD workflow `dev-story` from `_bmad/bmm/workflows/4-implementation/dev-story/workflow.yaml`.

**Target story**: `<STORY_KEY>` (e.g., "3-2")

**Purpose**: Implement all tasks/subtasks, add/adjust tests, and update story status/doc fields as required.

**Expected output**:
- Summary of changed files (implementation + tests)
- Test results (all passing)
- Story file updated with implementation notes
- sprint-status.yaml updated to review/done status

**Usage**:
```
Run dev-story for story 3-2
```

---

## 4. code-review

Run BMAD workflow `code-review` from `_bmad/bmm/workflows/4-implementation/code-review/workflow.yaml`.

**Target story**: `<STORY_KEY>` (e.g., "3-2")

**Purpose**: Perform adversarial review, list concrete findings by severity, and apply safe auto-fixes when appropriate.

**Expected output**:
- List of findings categorized by severity (high/medium/low/info)
- Report of what was auto-fixed vs what needs manual decisions
- Updated story file with review notes
- sprint-status.yaml updated if all critical issues resolved

**Review modes**:
- `auto-fix-all`: Auto-fix all severities (risky)
- `auto-fix-high-medium`: Auto-fix high/medium only (recommended)
- `auto-fix-low`: Auto-fix low/info only (safe but minimal)
- `manual`: No auto-fix, report only

**Usage**:
```
Run code-review for story 3-2 with auto-fix-high-medium mode
```

---

## 5. party-mode-audit

Run BMAD workflow `party-mode-audit` from `_bmad/bmm/workflows/4-implementation/party-mode-audit/workflow.yaml`.

**Target story**: `<STORY_KEY>` (e.g., "3-2")

**Purpose**: Run structured multi-agent style risk audit on a story before development and append actionable follow-ups.

**Expected output**:
- Audit section appended to story file with:
  - Security risks
  - Architecture concerns
  - UX/usability issues
  - Performance considerations
  - Testing gaps
- Prioritized action list from PM, Architect, Dev, and QA perspectives

**Usage**:
```
Run party-mode-audit for story 3-2
```

---

## Flow Examples

### Execute full epic automation
```
Run epic-auto-flow for Epic 3, showing stage-by-stage progress.
```

### Execute single story (manual flow)
```
1. Run create-story for Epic 3
2. Run party-mode-audit for story 3-2
3. Run dev-story for story 3-2
4. Run code-review for story 3-2 with auto-fix-high-medium
```

### Resume after failure
```
# If epic-auto-flow failed at story 3-4, code-review stage:
1. Check failure report: _bmad-output/implementation-artifacts/epic-3-auto-flow-failure-2026-02-17.md
2. Fix the identified issue manually
3. Run epic-auto-flow for Epic 3 (will resume from next pending story)
```

---

## Context Compression Details

**What gets preserved**:
- Current story key and status
- Bugs/issues list from code-review
- Story processing count
- Pipeline active state
- Epic progress summary

**What gets compressed**:
- Detailed conversation history
- Tool call results from previous stories
- Verbose output from workflows

**Compression triggers**:
- After code-review phase (first compression)
- After bug-test-writing phase (second compression)
- Runs 2x per story automatically

**Benefits**:
- Manages token usage for epics with many stories
- Prevents context window overflow
- Preserves critical state for pipeline continuity
- Allows processing of large epics in single session

---

## Customization Variables

Edit `_bmad/bmm/workflows/4-implementation/epic-auto-flow/workflow.yaml`:

```yaml
variables:
  epic_num: "3"                          # Target epic
  review_mode: "auto-fix-high-medium"    # Code review policy
  failure_mode: "fail-fast"              # Stop on first error
  compress_after_story: "true"           # Enable compression
  compress_after_phases: "code-review,bug-test-writing"  # When to compress
```

**Common customizations**:
- Change `epic_num` to "4" to run Epic 4
- Change `review_mode` to "manual" for no auto-fixes
- Change `compress_after_story` to "false" to disable compression
- Add more phases to `compress_after_phases`: "party-mode-audit,code-review,bug-test-writing"
