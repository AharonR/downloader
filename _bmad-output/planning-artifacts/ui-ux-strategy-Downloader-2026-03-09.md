---
date: 2026-03-09
author: codex
status: draft
inputs:
  - "_bmad-output/planning-artifacts/product-brief-Downloader-2026-03-08.md"
  - "_bmad-output/planning-artifacts/strategic-roadmap-Downloader-2026-03-08.md"
  - "_bmad-output/planning-artifacts/current-state-and-90-day-plan-Downloader-2026-03-09.md"
  - "_bmad-output/planning-artifacts/ui-scope-decision.md"
  - "_bmad-output/planning-artifacts/ux-design-specification.md"
  - "README.md"
  - "downloader-app/src/routes/+page.svelte"
  - "downloader-app/src/lib/DownloadForm.svelte"
  - "downloader-app/src/lib/ProjectSelector.svelte"
  - "downloader-app/src/lib/ProgressDisplay.svelte"
  - "downloader-app/src/lib/CompletionSummary.svelte"
  - "downloader-app/src/lib/StatusDisplay.svelte"
---

# UI/UX Strategy: Downloader

## Executive Summary

Downloader's UI should reflect the product it actually is now: a trusted evidence-intake and corpus-preparation tool, not a generic downloader, not a broad AI research suite, and not yet a full library-of-record application.

The current desktop app is already a valid narrow wedge: paste sources, choose a project, run a batch, watch progress, inspect results. That is a strong starting point. The next UI/UX job is not to add breadth. It is to make trust, provenance, and workflow readiness more visible while keeping the experience calm, local-first, and operationally clear.

This document therefore defines a UI/UX direction for Downloader that:

- starts from the shipped desktop app, not a greenfield concept
- aligns the interface with the March 8 product and strategy framing
- preserves the CLI as the precision and automation surface
- treats the desktop app as the trust-building and workflow-onramp surface
- focuses the next phase on clarity, reviewability, and handoff readiness rather than new product sprawl

## What This Document Reframes

Earlier UX work explored a broader GUI direction with richer navigation, more speculative result structures, and wider interaction patterns. That work remains useful as an idea bank, but Downloader is now in a different state:

- the product is already shipped in both CLI and desktop forms
- the strategic wedge is clearer: trusted evidence intake and corpus preparation
- the key risk is not "can we build a GUI?" but "can the GUI make the wedge more legible and more repeatable?"

Where this document conflicts with earlier broad GUI assumptions, this document should guide near-term work.

## Product Role in UI Terms

Downloader's interface is not primarily about helping users discover sources or manage an entire reference library. Its job is narrower and more important:

1. help users turn messy source lists into clean runs
2. show what happened in a way they can trust
3. preserve enough structure and evidence that the outputs can move into downstream workflows

That means the UI must optimize for:

- legibility of inputs
- confidence during execution
- inspectable outcomes
- friction-light recovery from partial failure
- easy movement into the user's next tool or folder workflow

## Surface Strategy

### CLI

The CLI remains the power-user and automation surface.

It should continue to optimize for:

- speed
- explicit flags
- scriptability
- repeatability
- dense operational feedback

### Desktop App

The desktop app should be the approachable, trustworthy workflow surface.

It should optimize for:

- clear first-run understanding
- reduced anxiety during long or messy runs
- visible project context
- inspectable completion and failure states
- easy handoff to the local filesystem and downstream tools

### Relationship Between Surfaces

The desktop app should not attempt to replace the CLI's strengths. It should instead expose the same product truth more accessibly:

- same core engine
- same project structure
- same trust model
- same local-first behavior
- different interaction density

## Current UI Audit

### What Exists Today

The current desktop app is a focused single-window workflow:

- a title and subtitle
- a project selector
- a multiline source input area
- a download button and cancel action
- live progress
- a completion summary with failed-item details

This is coherent and appropriately narrow for the current wedge.

### Current Strengths

- The flow is simple and low-friction.
- The app is local-first in feel and behavior.
- Progress and completion states are already visible.
- The project selector helps anchor work in a reusable output structure.
- The failure summary supports recovery better than a generic "something failed" toast would.

### Current Gaps

- The messaging is still too narrow and too generic. The current subtitle frames Downloader as a paper downloader, not as a trusted evidence-intake layer.
- Trust signals are still lighter than they should be. Users can complete a run, but they cannot yet review provenance, normalization, or outcome quality in a strong UI-native way.
- The app currently exposes the run, but not much of the post-run workflow.
- The README describes settings behavior for the desktop app, but the current desktop UI remains visibly narrow and does not yet present those controls in-app.
- The visual system is serviceable but generic. It feels like a utility shell rather than a distinctive research workbench.

### Design Implication

The next UI/UX phase should deepen the current workflow rather than branching into more navigation or more product areas.

## UX Principles

### 1. Make Trust Visible

Downloader wins when users trust the output. The interface must make success inspectable:

- what was fetched
- what failed
- what was skipped
- where files went
- what source each item came from

### 2. Keep The Workflow Narrow

The UI should resist becoming a general-purpose research cockpit. It should stay focused on the handoff from source list to usable local corpus.

### 3. Treat Partial Failure As Normal

Real-world intake is messy. The UI must normalize partial failure without making users feel the run was useless.

### 4. Preserve Momentum

Every screen should answer "what can I do next?" without requiring the user to re-parse the system.

### 5. Stay Calm, Not Hype-Driven

Downloader should not look or sound like an AI toy or a generic SaaS dashboard. The tone should communicate rigor, usefulness, and control.

### 6. Prefer Evidence Over Decoration

When space is constrained, show provenance, status, and structure before adding visual flourish or secondary navigation.

## Experience Goals

The user should feel:

- before run: "This is straightforward."
- during run: "I can see that the tool is working and I know what it is doing."
- after run: "I understand the result and can use it immediately."
- after a partial failure: "I know what failed, why, and what to do next."

The UI should reduce three specific anxieties:

- "Did it actually fetch the right things?"
- "Where did everything go?"
- "How much cleanup do I still have to do?"

## Information Architecture

## Primary Window Model

For the next phase, Downloader should remain a single primary workspace rather than a full multi-page application.

That workspace should contain four stable zones:

1. `Intent`
Purpose, context, and project framing.

2. `Input`
Source entry, project selection, and any lightweight run options.

3. `Run State`
Resolution, progress, in-flight status, and interruption handling.

4. `Outcome`
Completion summary, failed items, output destination, and next-step actions.

## Proposed Near-Term Screen Structure

### Section A: Workbench Header

Purpose:
- state what Downloader does in product terms
- reinforce local-first trust
- keep the window from feeling like a generic form

Recommended content:
- product title
- one-sentence value proposition
- small context note such as "Local-first evidence intake"

### Section B: Intake Panel

Purpose:
- let the user define the run with minimal friction

Contents:
- project selector
- multiline source input
- optional examples or input hints
- a minimal advanced-options affordance later, not by default

### Section C: Active Run Panel

Purpose:
- show that the system is progressing
- communicate whether the run is healthy, stalled, or mixed

Contents:
- overall progress
- failure count
- per-item active list
- cancellation action
- status copy that reflects resolution and download phases distinctly when possible

### Section D: Results Panel

Purpose:
- turn the end of the run into a usable handoff moment

Contents:
- completed count
- failed count
- output directory
- failed-item details
- next actions such as open folder, inspect log, or start another run when those surfaces exist

## Key Flows

### Flow 1: First Run

User goal:
- try Downloader on a real source list with minimal confusion

UX requirement:
- the first-run screen must explain the product in one glance
- examples should be concrete and mixed-format aware
- success should end in a clear "this is where your corpus is" moment

### Flow 2: Repeat Project Use

User goal:
- add new sources to an existing project

UX requirement:
- project recall must be lightweight
- prior project names should be easy to reuse
- the system should feel like it is helping continue work, not starting from zero

### Flow 3: Partial Failure Recovery

User goal:
- salvage useful work from an imperfect run

UX requirement:
- failures should be grouped and inspectable
- error language should remain actionable
- the success path should stay visible even when some items fail

### Flow 4: Auth or Access Friction

User goal:
- understand why specific sources failed and what kind of fix is needed

UX requirement:
- distinguish likely auth/access failure from generic network failure
- preserve the What/Why/Fix language model already used in CLI/backend messaging

### Flow 5: Post-Run Handoff

User goal:
- move the resulting corpus into reading, citation, or AI analysis workflows

UX requirement:
- the app should expose output location and artifacts clearly
- future handoff actions should appear here before they appear elsewhere in the app

## Content Strategy

## Product Language

The UI should consistently describe Downloader as:

- trusted evidence intake
- corpus preparation
- local-first
- project-based

It should use "sources", "project", "results", "failures", "output folder", and "corpus" more often than "download" alone.

## Microcopy Style

The copy style should be:

- precise
- calm
- concrete
- non-promotional
- operational without sounding cold

Good tone:
- "Add sources for a project"
- "Results saved to your project folder"
- "3 items need attention"

Avoid:
- "Let AI organize your research"
- "Your knowledge pipeline is ready"
- "Smart magic"

## Error Language

The What/Why/Fix pattern should be preserved visually in the desktop app where practical. That is one of Downloader's strongest UX assets because it turns failure into diagnosis instead of noise.

## Visual Direction

Downloader should feel like a research workbench, not a glossy AI product and not a corporate admin console.

### Desired Visual Character

- archival rather than futuristic
- precise rather than playful
- warm-neutral rather than sterile white
- structured rather than dashboard-heavy

### Recommended Visual System

- background: soft paper or stone neutrals, not flat bright white everywhere
- primary accent: deep blue or ink tone for progress and primary actions
- success: muted green, not neon
- warning/error: oxide or rust variants with strong contrast
- typography: one readable text family plus one monospace utility family for paths, domains, and technical evidence
- spacing: generous enough to reduce cognitive crowding, but denser than consumer productivity apps

### Visual Anti-Patterns

- generic AI gradients and glowing surfaces
- overuse of cards and dashboard widgets
- chat-style layouts
- decorative motion unrelated to system state

## Accessibility and Interaction Standards

The desktop app should meet practical accessibility standards from the start:

- full keyboard operability for the primary flow
- visible focus states
- semantic labels on inputs and progress regions
- color not being the only failure/success indicator
- readable error blocks with preserved line breaks
- mobile-style touch targets are not necessary, but click targets should remain generous

The current use of native patterns such as `datalist` for project suggestions is directionally correct where it reduces custom accessibility burden.

## Desktop-Specific Guidance

- Respect local filesystem expectations.
- Show paths clearly and in copyable form.
- Prefer direct actions like "Open folder" over abstract links when those hooks are added.
- Do not hide important run information behind hover-only interactions.
- Keep interruption and completion states explicit.

## Near-Term Feature Priorities For UI/UX

### Priority 1: Reframe The App

Update the visible product story so the window clearly says what Downloader is for:

- trusted intake
- organized project output
- local-first evidence workflow

### Priority 2: Strengthen Trust Signals

Add or emphasize the information that helps users believe the output:

- clearer result breakdown
- stronger output artifact visibility
- provenance and source visibility where feasible
- explicit distinction between downloaded, failed, skipped, and blocked states

### Priority 3: Improve Post-Run Utility

The end of a run should feel like the start of the next task:

- expose project folder
- expose artifacts
- provide better failure review
- prepare the UI for lightweight handoff actions

### Priority 4: Delay Broad Navigation

Do not add settings pages, library views, dashboards, or multi-pane exploration until the current wedge flow proves that those surfaces solve real problems.

## Out of Scope

For this phase, the UI should explicitly avoid:

- full citation-manager behavior
- note-taking and annotation surfaces
- AI chat or summarization as the main UI frame
- multi-user collaboration UX
- enterprise admin consoles
- broad dashboard navigation designed to imply product breadth that does not yet exist

## Phased UI/UX Roadmap

### Phase A: Clarify and Tighten

Goal:
- make the current one-window workflow feel unmistakably aligned with the product strategy

Deliverables:
- improved header and product framing
- stronger intake hints
- cleaner progress and completion hierarchy
- better status language

### Phase B: Make Results Reviewable

Goal:
- help users trust and act on the run outcome

Deliverables:
- richer completion panel
- clearer failure groupings
- stronger artifact visibility
- lightweight result inspection patterns

### Phase C: Add Thin Handoff Surfaces

Goal:
- support the most important downstream workflow without broadening the app unnecessarily

Deliverables:
- one or two minimal handoff actions
- improved manifest or export visibility
- stronger project continuation patterns

## UX Success Metrics

The UI is succeeding if it improves:

- first-run comprehension
- completion-to-next-step confidence
- repeat project usage
- recovery after partial failure
- successful handoff into downstream workflow

Practical signals:

- fewer abandoned runs
- fewer support-style questions about where outputs went
- higher reuse of project names
- better tolerance of partial-failure runs
- stronger repeat use after first completion

## Final Direction

Downloader's UI should behave like a calm evidence workbench.

It should not try to look bigger than the product is. It should make the current wedge easier to trust, easier to repeat, and easier to carry into the user's next workflow. If the interface does that well, it will strengthen the product more than any broad navigation or speculative AI surface would.
