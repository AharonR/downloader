---
date: 2026-03-08
author: fierce
status: draft
inputs:
  - "_bmad-output/planning-artifacts/product-brief-Downloader-2026-03-08.md"
  - "_bmad-output/planning-artifacts/research/market-downloader-future-strategy-research-2026-03-08.md"
v2_date: 2026-03-09
v2_audit: audit-10-expert-product-strategy-2026-03-09.md
---

# Strategic Roadmap: Downloader

## Strategic Thesis

Downloader should win a narrow but valuable layer of the research workflow: trusted evidence acquisition and corpus preparation. It should not try to become a full reference manager or a broad AI research suite. The roadmap should therefore sequence from `wedge` to `workflow layer` to `governed team offering`, with each phase gated by evidence rather than ambition.

## North Star

Turn source chaos into reusable evidence corpora faster, more reliably, and more transparently than manual workflows or generalized tools.

**User-facing positioning:** *"Turn your source list into a research-ready corpus. Every file named, traced, and ready to cite."*

## Strategy Rules

- win the acquisition-and-preparation step before expanding upward
- integrate into existing research stacks instead of replacing them
- prioritize determinism, provenance, and corpus hygiene over feature breadth
- validate enterprise-adjacent demand only after the power-user wedge is durable
- do not add generic AI surface area unless it directly strengthens trust or workflow utility
- suite expansion deferred until Phase 4 decision gates are met, or earlier if 3+ pilot teams independently request integrated suite capabilities

## What Downloader Should Not Do

- do not compete head-on with Zotero, EndNote, Mendeley, or Paperpile on library-of-record UX
- do not build generic chat, summarization, or “AI copilot” experiences as the main product story
- do not move into heavy enterprise control-plane scope before wedge validation
- do not let GUI ambitions outrun proof that the intake layer is valuable on its own

## Phase 1: Validate the Wedge

**Timing:** March 2026 to May 2026

**Goal:** Prove that Downloader is materially better than manual intake for academic power users and research-heavy individual users.

### Product Priorities

- harden mixed input handling for URLs, DOIs, and bibliography-style inputs
- improve deterministic success/failure reporting
- make provenance visible in outputs by default
- improve file naming, normalization, and basic deduplication
- continue hardening authenticated and difficult-source workflows

### Market and User Priorities

- recruit 10-15 design partners from the academic power-user segment
- run structured interviews on current intake workflows and failure points
- benchmark Downloader against manual + Zotero/Mendeley-assisted intake on real source sets
- refine the positioning statement around `trusted intake layer` and `corpus readiness`

### User Acquisition Plan

Channels for design partner recruitment (see `gtm-acquisition-plan-Downloader-2026-03-09.md` for full playbook):

- **Academic social media** (Bluesky, Twitter/X): Find researchers sharing workflow frustrations; engage authentically; demonstrate concrete problem-solving
- **Reddit** (r/GradSchool, r/bioinformatics, r/PhD, r/MachineLearning): Answer existing questions; share workflow comparisons
- **Zotero forums**: Position as complementary — "Downloader acquires, Zotero manages"
- **Library science listservs** (Code4Lib, ACRL): Emphasize compliance, provenance, institutional compatibility
- **Hacker News**: One well-timed Show HN post after Phase 1 hardening
- **Conference outreach**: Lightning talks at library tech and research computing events

Approach: Find researchers already complaining about corpus preparation. Offer to solve their specific problem. Convert satisfied users into design partners.

### Decision Gates

- at least 60% of design partners use Downloader on more than one real project
- benchmark runs show clear reduction in manual cleanup time
- provenance/completion output is trusted enough that users feed results into downstream workflows
- hard-case acquisition performance improves over baseline rather than plateauing

### Deliverables

- stable wedge feature set
- benchmark corpus test sets
- validated positioning language
- evidence-backed list of top remaining workflow blockers

## Phase 2: Become the Preferred Intake Layer

**Timing:** June 2026 to September 2026

**Goal:** Make Downloader easy to route into the tools users already trust.

### Product Priorities

- add stronger export and handoff surfaces for downstream workflows
- improve metadata enrichment and duplicate handling
- add automation-friendly interfaces for repeatable runs
- package outputs for common citation and AI-ready corpus workflows
- improve observability of what was fetched, skipped, blocked, or normalized

### Market and Distribution Priorities

- publish workflow guides for academic and analyst use cases
- position Downloader as complementary to Zotero, Mendeley, Papers, NotebookLM, and AI pipelines
- target communities where evidence handling pain is acute rather than broad top-of-funnel awareness
- collect public case studies or internal references from design partners

### Decision Gates

- a meaningful share of active users export or hand off results into downstream tools
- repeated workflow usage grows faster than one-time trials
- user feedback shifts from “can it download this?” to “can it fit my existing workflow?”
- demand appears for automation hooks and repeatable team-friendly usage

### Deliverables

- integration-ready export surfaces
- workflow playbooks
- clearer segmentation of solo versus team needs
- stronger proof that Downloader belongs in the middle of the stack

## Phase 3: Package for Team and Governed Workflows

**Timing:** October 2026 to January 2027

**Goal:** Test whether Downloader can become a serious workflow component for teams building governed corpora.

### Product Priorities

- introduce team-friendly configuration and reproducibility features
- add better manifests, run records, and audit-friendly output conventions
- improve deployment patterns for internal shared use
- define the minimum viable API or automation surface for non-interactive workflows

### Market Priorities

- recruit 3-5 pilot teams in labs, research groups, or knowledge-ops settings
- test team adoption requirements without committing to full enterprise scope
- validate whether governance, repeatability, and packaging are strong enough to justify a deeper move

### Decision Gates

- at least two teams sustain use beyond a short pilot
- Downloader proves compatible with shared or governed workflow expectations
- team pilots reveal repeatable product requirements rather than one-off custom asks
- the product can support larger-scale ingestion without collapsing trust

### Deliverables

- pilot packaging for team workflows
- team-grade requirements list
- evidence on whether enterprise-adjacent expansion is justified
- clearer separation between product core and deployment layer

## Phase 4: Decide the Next Product Shape

**Timing:** February 2027 to March 2027

**Goal:** Make an explicit strategy choice based on evidence, not momentum.

### Strategic Options

**Option A: Double down on the power-user wedge**

- remain a specialist tool with best-in-class acquisition and corpus preparation
- grow through community adoption and workflow depth

**Option B: Expand into workflow infrastructure**

- invest in APIs, team packaging, and governed deployment
- target research-heavy teams and knowledge operations

**Option C: Stop broadening and refine**

- tighten the product around the most reliable, most defensible use cases
- reduce surface area and optimize for excellence in the wedge

### Required Inputs for the Decision

- retention and repeat usage data
- benchmark quality outcomes
- export/integration usage patterns
- pilot-team feedback
- evidence on willingness to operationalize Downloader inside shared workflows

## Cross-Phase Product Principles

### Technical Moat: Resolver Architecture

Downloader's resolver pipeline is a defensible technical asset that requires sustained per-source investment to replicate:

| Resolver | Domain Knowledge Encoded |
|----------|-------------------------|
| arXiv | PDF URL construction from abs/paper IDs, version handling |
| Crossref | DOI → metadata via content negotiation, mailto-based polite pool |
| IEEE | IEEE Xplore document ID extraction, `10.1109/*` DOI routing |
| PubMed | PMID/PMC resolution, PubMed Central full-text access |
| ScienceDirect | Elsevier PII extraction, `10.1016/*` DOI routing |
| Springer | Chapter/article URL pattern recognition |
| YouTube | oEmbed metadata extraction + transcript retrieval |
| Direct | URL passthrough fallback for any unrecognized source |

Standard metadata keys across all resolvers: `title`, `authors`, `doi`, `year`, `source_url` (from `STANDARD_METADATA_KEYS` in `resolver/mod.rs`).

The registry uses priority-ordered resolution (Specialized → General → Fallback) with graceful degradation when individual resolvers are unavailable.

**Future extensibility:** Community-contributed resolver plugins via the `Resolver` trait. Any crate implementing `name()`, `priority()`, `can_handle()`, and `resolve()` can be registered.

### Core Principles

- every release should improve trust, not just capability
- every new feature should reduce downstream cleanup or increase reuse
- every integration should make Downloader harder to remove from the workflow
- every expansion should be tested against the rule: does this strengthen the intake layer or distract from it?

## Concrete Metrics by Phase

### Phase 1

- corpus completion rate
- manual cleanup reduction
- provenance completeness
- repeat usage among design partners

### Phase 2

- downstream handoff success
- integration usage rate
- repeat workflow automation rate
- retention among heavy users

### Phase 3

- team pilot continuation rate
- governed workflow fit
- shared-run reliability
- larger-ingestion quality and trust

## Roadmap Risks

- incumbents improve import/acquisition fast enough to narrow the wedge
- the product remains useful but invisible, limiting adoption growth
- team requirements arrive before the wedge is strong enough
- effort diffuses into AI features or UI surface area that do not strengthen the core
- research confidence remains directional rather than decision-grade in some adjacent areas

## Competitive Intelligence Cadence

Quarterly refresh of the competitive landscape to ensure strategy stays calibrated to actual market movement.

- **Scope:** Track feature releases, funding, acquisitions for top 12 competitors (Zotero, Mendeley, EndNote, Paperpile, ReadCube/Papers, Elicit, Consensus, Scite, NotebookLM, Perplexity, Semantic Scholar, Unstructured)
- **Cadence:** Quarterly (next refresh: June 2026)
- **Action triggers:** Defined events that prompt immediate strategy review rather than waiting for quarterly refresh
- **Tracker:** See `research/competitive-velocity-tracker-2026-03-09.md` for per-competitor cards, refresh process, and trigger definitions

## Immediate Next Moves

1. Use the March 8 product brief as the strategic source of truth.
2. Turn the wedge into explicit product requirements and validation tests.
3. Define the benchmark corpus sets and design-partner program for Phase 1.
4. Map the first downstream handoff targets that matter most after wedge validation.
5. Review roadmap progress against decision gates every 6-8 weeks.

## Final Position

Downloader’s best path is disciplined expansion. First become indispensable at trusted evidence intake. Then become the preferred handoff layer into the rest of the research stack. Only after that should Downloader try to become a broader workflow platform.
