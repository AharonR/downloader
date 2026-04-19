---
date: 2026-04-03
author: fierce + pm-agent
status: draft
type: strategic-analysis
inputs:
  - "_bmad-output/planning-artifacts/product-brief-Downloader-2026-03-08.md"
  - "_bmad-output/planning-artifacts/strategic-roadmap-Downloader-2026-03-08.md"
  - "_bmad-output/planning-artifacts/current-state-and-90-day-plan-Downloader-2026-03-09.md"
  - "conversation: stress-test session 2026-04-03"
---

# Agentic Pivot Analysis: Downloader as Agent Infrastructure

## Context

All 11 epics complete. 864 tests passing. NFR gate PASS. The product is shipped and working.

The question: **is Downloader actually better than manual intake for any user?**

After honest evaluation, the answer for human users is: marginally, in a narrow band. For agents, the answer flips entirely.

---

## Part 1: Why the Human-Facing Tool Is Marginal

### The competitive reality

| Scenario | Downloader | Alternative | Winner |
|----------|-----------|-------------|--------|
| 50 open-access DOIs | CLI pipeline, sidecars, index | Zotero "Add by Identifier" — one click | Toss-up |
| 50 paywalled DOIs | Downloads OA, flags rest as NeedsAuth | Browser + institutional login | Manual/Zotero |
| Mixed URLs + DOIs | Single command, resolves both | Split into Zotero + manual | Downloader |
| Building an LLM corpus | Structured output, sidecars, exportable | Manual folder organization | Downloader |
| One-off paper download | `downloader <url>` | Click "Download PDF" in browser | Browser |

### Core issues

1. **Paywalled content (majority of academic papers)** — tool surfaces NeedsAuth and stops. Cookie capture workflow adds friction vs. just downloading from browser.
2. **Zotero already does the easy cases** — for OA papers, Zotero's batch DOI import is easier with better UX.
3. **The "walk away" promise is undermined** — if 30% comes back NeedsAuth/Failed, you still have manual cleanup.
4. **File naming is basic** — uses Content-Disposition/URL path, not the Author_Year_Title pattern from the product brief.
5. **manifest.json doesn't exist** — described in product brief but not implemented.

### Where value is real but narrow

- Batch downloading 20+ mixed inputs where meaningful fraction is open-access
- LLM corpus preparation (sidecars, structured output, export) — Zotero doesn't target this
- CLI/automation workflows — Zotero is GUI-first

---

## Part 2: The Agentic Thesis

### The value calculus flips

**For a human**: Downloader saves ~10 minutes vs. browser tabs. Marginal.

**For an agent**: Without something like Downloader, the agent literally cannot acquire evidence. No browser, no institutional login, no way to navigate publisher quirks.

### Why Downloader's existing design fits agents better than humans

| Feature | Human value | Agent value |
|---------|-------------|-------------|
| CLI/API-first interface | Friction vs GUI | Exactly what agents need |
| JSON-LD sidecars | Nice-to-have | Essential (machine-readable metadata) |
| Structured error types (NeedsAuth, Failed) | Informational | Decision signals for next action |
| Robots.txt + rate limiting | Responsible but invisible | Critical — agents need compliance without understanding etiquette |
| Deterministic resolution | Expected | Required for reproducibility |
| Cookie/auth management | More friction than browser | Agents can orchestrate programmatically |

### The positioning shift

**Before:** "Turn your source list into a research-ready corpus" (for researchers)

**After:** "Give your research agent a reliable way to turn citations into a verified local corpus" (for agents and agent builders)

---

## Part 3: Angle Clustering and Ranking

### Cluster A: Distribution & Interface Shape

#### Angle 1 — MCP Server

**Thesis:** Wrap existing resolver pipeline as MCP tool server. Agents in Claude Code, Cursor, etc. call `resolve_doi()`, `download_batch()`, `search_corpus()` directly.

**Rebuttal:** MCP is early. Adoption growing but not guaranteed as the standard. Building for one protocol risks being stranded.

**Counter-rebuttal:** Cost is days, not months. Resolver logic works regardless of protocol. MCP is just the first surface.

| Dimension | Score (1-5) | Rationale |
|-----------|-------------|-----------|
| Validity | 5 | Shape already fits. Lib/bin split, structured errors, tool-like interfaces |
| Applicability | 5 | Use Claude Code daily. Own design partner on day one |
| Urgency | 5 | MCP adoption accelerating now. "Paper acquisition tool" position is open |
| **Implied Impact** | **4** | Proves thesis fast. Doesn't scale alone — validation vehicle, not a business |

#### Angle 6 — "What Ships Fast"

**Thesis:** MCP server → library crate → REST API is the effort-ordered sequence.

**Rebuttal:** Shipping fast ≠ shipping right. Quick MCP wrapper might create API debt.

**Counter-rebuttal:** Lib crate already has clean interfaces. Wrapper is thin. API debt risk low.

| Dimension | Score (1-5) | Rationale |
|-----------|-------------|-----------|
| Validity | 4 | Correct sequencing. "Fast" only valuable if you measure something |
| Applicability | 5 | Directly actionable this week |
| Urgency | 4 | Supports Angle 1 |
| **Implied Impact** | **3** | Tactical, not strategic. Enables impact but isn't impact itself |

**Cluster A verdict:** Execution path. MCP server is first move. Low cost, high signal.

---

### Cluster B: Market Position & Moat

#### Angle 2 — Picks and Shovels

**Thesis:** Every AI research startup built bespoke paper acquisition. Downloader-core as shared library crate is the infrastructure they all need.

**Rebuttal:** Incumbents already built their pipelines. Won't rip out for external dependency from solo developer. Their pipelines handle more (full-text extraction, citation graphs).

**Counter-rebuttal:** Fair for incumbents. But new entrants (agent startups shipping weekly) haven't built this yet. Resolver registry encodes months of publisher-specific domain knowledge.

| Dimension | Score (1-5) | Rationale |
|-----------|-------------|-----------|
| Validity | 3 | True in theory. Hard to sell to companies that already have it. Better for new entrants |
| Applicability | 3 | Requires crates.io publish, API stability, docs, trust-building |
| Urgency | 2 | Window is quarters, not weeks |
| **Implied Impact** | **5** | If it works: largest outcome. Downloader becomes ecosystem infrastructure |

#### Angle 5 — Market Timing

**Thesis:** Research agents exploding. None have good paper acquisition. "Standard tool agents call for papers" position is open right now.

**Rebuttal:** "Window closing" is the oldest pitch in product. Windows usually wider than they look. Well-funded startup could replicate resolver logic in weeks with LLM assist.

**Counter-rebuttal:** Resolver logic is the easy part. The trust layer — robots.txt, rate limiting, structured failures, responsible-use guardrails — is what makes an agent tool shippable in production. Harder to replicate because it requires caring about it.

| Dimension | Score (1-5) | Rationale |
|-----------|-------------|-----------|
| Validity | 4 | Window is real. "Standard paper acquisition for agents" genuinely unoccupied |
| Applicability | 4 | Applies if MCP server ships and gets visible in agent tooling ecosystem |
| Urgency | 4 | Not weeks, but next 6 months matter |
| **Implied Impact** | **5** | Defines the product category |

**Cluster B verdict:** Strategic bet. Picks-and-shovels is long game; market timing is reason to start now. Moat isn't resolvers — it's the trust/compliance layer.

---

### Cluster C: Demand Validation & Counter-Arguments

#### Angle 3 — "Agents Don't Read Papers" (Skeptical)

**Thesis:** Agents need content, not files. APIs already provide abstracts/metadata. "Download PDF to disk" is a human pattern agents might skip.

**Rebuttal:** Falls apart for RAG pipelines (need documents), verification workflows (need artifacts), offline/governed environments (can't call APIs at inference time), legal/compliance (need proof of document).

**Assessment:** Rebuttal is valid but narrow. Describes enterprise-adjacent use cases, not power-user. Typical agent today calls S2, gets abstract, moves on.

| Dimension | Score (1-5) | Rationale |
|-----------|-------------|-----------|
| Validity | 4 | Real risk. Most agent workflows today don't need local files |
| Applicability | 5 | Directly challenges core thesis. Must be answered |
| Urgency | 3 | Not urgent to resolve, but urgent to design around |
| **Implied Impact** | **4** | If true, narrows TAM significantly. If addressed, becomes feature gap |

**Critical design implication:** MCP server shouldn't just expose `download_batch()`. Must also expose `get_metadata()`, `get_abstract()`, `check_access()` — lightweight operations agents want more often than full downloads.

#### Angle 4 — Zotero-for-Agents

**Thesis:** Zotero has a web API. Agents could use Zotero as acquisition layer.

**Rebuttal:** Zotero requires user accounts, synced library, GUI setup. Opaque failure modes. Proprietary database, not portable corpus. Designed for human library management, not agent tooling.

**But also:** Zotero's distribution (millions of users) means agents will be built on top regardless. Play isn't to compete — it's to be complementary.

| Dimension | Score (1-5) | Rationale |
|-----------|-------------|-----------|
| Validity | 3 | Real competitor for humans, weak for agents |
| Applicability | 3 | Zotero integration is nice-to-have, not priority |
| Urgency | 1 | Zotero isn't moving into agent tooling |
| **Implied Impact** | **2** | Complementary positioning is smart but won't drive adoption |

**Cluster C verdict:** Angle 3 (skeptical) is the most important finding. It tells you the product surface for agents is metadata and access checks first, downloads second.

---

### Ranked Summary

| Rank | Angle | Cluster | Validity | Applicability | Urgency | Impact | Action |
|------|-------|---------|----------|---------------|---------|--------|--------|
| 1 | MCP Server | A: Distribution | 5 | 5 | 5 | 4 | **Ship this week. Be own first user** |
| 2 | Market Timing | B: Moat | 4 | 4 | 4 | 5 | **Position now. Agent-tool ecosystem visibility** |
| 3 | Agents Don't Read Papers | C: Validation | 4 | 5 | 3 | 4 | **Design around it. Metadata-first API** |
| 4 | Picks & Shovels | B: Moat | 3 | 3 | 2 | 5 | **Long game. Publish crate after API stabilizes** |
| 5 | What Ships Fast | A: Distribution | 4 | 5 | 4 | 3 | **Execution sequencing. Follows from #1** |
| 6 | Zotero-for-Agents | C: Validation | 3 | 3 | 1 | 2 | **Park. Revisit if hybrid use case emerges** |

---

## Part 4: Stress Test Results

### What survived

| Claim | Survived? | Nuance |
|-------|-----------|--------|
| Agents need paper acquisition tooling | Yes, but narrower than hoped | Metadata lookup is high-frequency; full download is lower-frequency |
| Resolver pipeline is the moat | Partially | Real value for full downloads; less relevant for metadata-only |
| MCP server is right first move | Yes | But design metadata-first, download-second |
| Market timing is urgent | Soft yes | Window is quarters, not weeks |
| Downloader beats manual for agents | Yes, clearly | This is where the comparison flips entirely |
| Solo crate can win B2B trust | No, not yet | Good for awareness; B2B requires track record |

### Additional stress-test findings

**Test: Does an agent need this tool, or just an HTTP client?**

Downloader adds value specifically when: (a) agent needs actual PDF file not just metadata, (b) PDF isn't behind a direct URL (needs resolver logic), (c) doing this at volume (needs rate limiting, dedup, queue). For "look up what this paper says" — Semantic Scholar API alone is enough.

**Test: How often do agents build corpora today?**

Corpus construction is real but infrequent. Common need is metadata lookup + access checking. High-frequency operation is `resolve_metadata()`, not `download_batch()`.

**Test: Competitive response time?**

Hard to replicate (months): 10 site-specific resolvers, publisher quirk knowledge, rate limiting tested against real publishers, cookie/auth flow. Easy to replicate (days-weeks): calling S2/Crossref APIs, basic DOI resolution, direct URL download. Moat is real but thin — 2-3 months for well-funded team.

**Test: Does MCP server actually validate the thesis?**

Strong signal: you call resolve_metadata from Claude Code during real research. Weak signal: you use it once and go back to browser. Kill signal: you never reach for it.

---

## Part 5: How This Maps to BMAD Process

### What BMAD completed

All 4 phases (Analysis → Planning → Solutioning → Implementation) ran to completion. Product shipped. But BMAD validated delivery, not product-market fit.

### The gap

BMAD assumes the product thesis is correct. The product brief and PRD were created without external user validation. The strategic roadmap (March 8) acknowledged "validation behind implementation" but still assumed the direction was right.

### What's needed now (not a full BMAD re-run)

```
1. Retrospective on product-market fit       ← DONE (this document)
2. Pivot brief                                ← New product brief, not from scratch
   - Same core technology
   - New customer (agents, not humans)
   - New interface (MCP, not CLI)
   - New success metrics
3. Lightweight PRD delta                      ← What's new, what's unchanged
4. Architecture amendment                     ← MCP server layer on existing core
5. Validation epic                            ← Ship MCP server, be own design partner
6. Kill/continue gate                         ← Did the agent actually use it?
```

---

## Part 6: The Refined Thesis

Downloader's value as a human-facing CLI tool is marginal. Downloader's value as agent infrastructure is real, with two distinct tiers:

**Tier 1 — High-frequency: Metadata + access resolution**
"What is this DOI? Can I access the PDF? What are the authors/title/year?"
Lightweight. No disk I/O, no queue, no database.

**Tier 2 — Low-frequency: Corpus construction**
"Download these 50 papers, organize them, generate sidecars."
Premium operation. Full pipeline.

### The moat

Not the resolvers (replicable in months). The trust layer: robots.txt compliance, rate limiting, structured failure modes, responsible-use guardrails. This is what makes an agent tool shippable in production.

### Validation approach

Don't recruit 10 design partners. Be the design partner. Ship the MCP server, use it yourself for 2 weeks, see if you reach for it or skip it.

---

## Part 7: Technical Readiness Assessment

The codebase is ~95% ready for agent consumption.

**Already in place:**
- Async/await throughout
- Structured error types (thiserror)
- Clean lib/bin split — CLI and Tauri are both thin consumers of core
- All core modules publicly exported: resolver, parser, queue, download, sidecar, auth, export
- Resolver trait is pluggable and well-abstracted
- Tauri app already proves the library is consumable by non-CLI callers (10 IPC commands)
- 864 tests, extensive mocking

**What's needed for MCP server:**
- MCP server wrapper (~200-400 lines) exposing core functions as tools
- Session state management (Database + Queue instance across invocations)
- Type serialization (mostly done — serde derives already present)

**Estimated effort:** ~500-1000 LOC. Thin wrapper, not a refactor.

---

## Open Questions

1. Which downstream handoff validates the thesis fastest: metadata resolution, corpus construction, or both?
2. Should the MCP server expose queue/history, or just stateless resolve + download?
3. Does the auth/cookie problem look different for agents (API keys, institutional tokens) vs. humans (browser cookie export)?
4. Is Rust the right language for the MCP server, or should it be a thin Python/TypeScript wrapper calling the core?
5. What does "visibility in the agent-tool ecosystem" actually mean — MCP registry listing, blog post, GitHub discoverability?

---

## Decision Required

This document is for consideration. No commitment to the agentic pivot has been made. Next steps depend on fierce's assessment of whether the thesis is worth a 4-week validation sprint.
