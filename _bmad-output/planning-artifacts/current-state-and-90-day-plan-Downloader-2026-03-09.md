---
date: 2026-03-09
author: codex
status: draft
inputs:
  - "_bmad-output/planning-artifacts/product-brief-Downloader-2026-03-08.md"
  - "_bmad-output/planning-artifacts/strategic-roadmap-Downloader-2026-03-08.md"
  - "README.md"
  - "_bmad-output/implementation-artifacts/sprint-status.yaml"
  - "docs/complexity-refactor/roadmap-status.md"
---

# Downloader: Current-State Conclusions and 90-Day Plan

## Conclusion Summary

The March 8 product brief and strategic roadmap are directionally right, but the current codebase is more mature than those documents imply on product delivery and less proven than those documents imply on market validation.

Downloader is no longer just a wedge concept. It is already a shipped evidence-intake product with a real CLI, a real desktop app, shared project artifacts, history/search, authenticated-download support, and meaningful testing and CI infrastructure. At the same time, the local status artifacts do not show equivalent progress on design-partner validation, workflow benchmarks against real alternatives, retention evidence, pilot-team proof, or first-class downstream integrations.

The resulting strategic read is:

1. Downloader's product direction is correct: trusted evidence intake and corpus preparation remains the strongest position.
2. Downloader's implementation maturity is ahead of its March 8 framing.
3. Downloader's market-validation maturity is behind its product maturity.
4. Downloader should not broaden into suite behavior, generic AI, or enterprise platform scope yet.
5. The next 90 days should focus on alignment, validation, thin handoff surfaces, and hardening rather than major new surface area.

## Current-State Assessment

### What Is True Now

- The core wedge is real and substantially implemented.
- The product already supports mixed intake, resolvers, authentication, project organization, sidecars, history, search, and desktop usage.
- The product already presents as a serious workflow tool rather than a prototype.
- The user-facing story has improved, but it still does not fully express the strategic positioning in the March 8 documents.

### What Is Not Yet Proven

- Which user segment will adopt Downloader first and repeatedly.
- Whether Downloader materially reduces cleanup time versus current real-world workflows.
- Which downstream handoff or integration surfaces matter enough to drive retention.
- Whether team/governed packaging is a real next market or only a plausible adjacency.

### Strategic Implication

Downloader should be managed as a product that is ahead on implementation and behind on evidence. That means the main job now is not inventing the next large feature set. The main job is proving where the current product wins, tightening the positioning around that win, and selectively adding the smallest downstream surfaces that strengthen the wedge.

## 90-Day Priorities

- Re-baseline the strategy and messaging around the shipped product.
- Validate the wedge with real users and benchmark evidence.
- Add only thin downstream handoff improvements.
- Close the remaining engineering credibility gaps that could undermine adoption.

# Plan

This plan turns the current-state conclusions into a 90-day execution sequence. The goal is to align strategy with the shipped product, prove the wedge with external evidence, add the minimum downstream surfaces needed for workflow fit, and close the remaining hardening gaps without diffusing into a broader suite roadmap.

## Scope
- In: strategy-to-product realignment, positioning updates, design-partner validation, benchmark corpus definition, workflow outcome measurement, thin handoff improvements, targeted engineering hardening, 90-day decision gates.
- Out: generic AI features, full reference-manager UX, broad enterprise control-plane work, large team-collaboration scope, speculative integrations beyond the first 1-2 validated handoff targets.

## Action items
[ ] Re-baseline the March 8 strategy docs into a single current-state view that separates shipped capability, unproven assumptions, and next bets.
[ ] Tighten user-facing positioning across core docs and product surfaces around "trusted evidence intake and corpus preparation" for academic power users and research-heavy analysts.
[ ] Recruit 10-15 design partners from the primary wedge segment and define a lightweight interview and usage-review cadence for the full 90-day window.
[ ] Define benchmark corpus sets and comparison workflows against manual intake and common adjacent tools so Downloader can be measured on completion, cleanup burden, and downstream usability.
[ ] Instrument or manually track the core wedge metrics: corpus completion rate, manual cleanup reduction, repeat project usage, downstream handoff success, and hard-case resolver performance.
[ ] Ship only thin downstream handoff improvements that reduce friction for real users, such as stronger manifests, clearer export packaging, or one validated handoff target, without expanding into suite behavior.
[ ] Close the remaining engineering trust gaps, especially the strict rollout gate follow-through, desktop path/DB consistency issues, and benchmark or instrumentation gaps that weaken product credibility.
[ ] Run a 6-week evidence review to cut low-signal work, confirm segment fit, and decide whether the first handoff improvements are increasing repeat use.
[ ] Run a 90-day strategy review to decide whether Downloader should continue deepening the solo-user wedge, invest in one or two workflow integrations, or begin limited team-packaging discovery.

## Open questions
- Which downstream handoff target should be the first one validated: citation workflow, local corpus workflow, or AI-ingestion workflow?
- How much repeat-usage evidence can realistically be collected in 90 days from the current network and distribution base?
- Should team/governed packaging remain strictly deferred until after the first validation cycle, or should one pilot be explored in parallel?

## Suggested 90-Day Cadence

### Days 1-15

- Align the narrative with the shipped product.
- Finalize the primary target segment and rejection criteria for non-core asks.
- Set up the design-partner list, benchmark source sets, and metric definitions.

### Days 16-45

- Run interviews and live workflow observations.
- Benchmark Downloader against real current-state alternatives.
- Fix the highest-friction wedge blockers exposed by those runs.

### Days 46-75

- Ship the smallest validated handoff improvements.
- Review whether repeat usage is improving.
- Close the remaining engineering credibility issues that could block recommendation or reuse.

### Days 76-90

- Synthesize the evidence.
- Decide whether Downloader is primarily:
  - a deepened solo power-user wedge,
  - an intake layer ready for selective integrations,
  - or still too early for expansion beyond core wedge hardening.

## 90-Day Decision Gates

At the end of this plan, Downloader should be able to answer these questions with evidence rather than inference:

- Do real users return for a second and third project?
- Does Downloader reduce manual corpus-preparation work in a measurable way?
- Which downstream handoff surface matters most to retention?
- Are team and governed-workflow needs showing up as repeatable demand or only as speculative future positioning?

If those questions remain unanswered after 90 days, the correct move is to continue sharpening and validating the wedge rather than broadening the product.
