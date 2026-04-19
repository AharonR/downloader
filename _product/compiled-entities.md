# Compiled Entity Reference

> Compiled 2026-04-13 from 71 memory files, 11 _product files, and 10 docs files.
> This is a point-in-time snapshot. All Phase A research completed 2026-04-12; Phase B pending for all three products.
>
> **Post-compilation additions (same session):** 4 new entity files (Zotero MCP, SemanticCite, PaperQA2, Trust Break pattern), 4 new spec/process files (compile prompt, session-start report, lint phase, Phase A→B handoff), 3 stale files fixed, 3 entity files updated with reflected findings, hypothesis scoring table updated.

---

## Table of Contents

1. [The Downloader Project](#1-the-downloader-project)
2. [Products Being Built](#2-products-being-built)
3. [Hypotheses (H1–H20)](#3-hypotheses-h1h20)
4. [New Findings (N1–N12)](#4-new-findings-n1n12)
5. [Dependency Map & Build Order](#5-dependency-map--build-order)
6. [People & Intellectual Influences](#6-people--intellectual-influences)
7. [Concepts & Frameworks](#7-concepts--frameworks)
8. [Competitors & Market Landscape](#8-competitors--market-landscape)
9. [Technologies & Infrastructure](#9-technologies--infrastructure)
10. [Research Papers & Benchmarks](#10-research-papers--benchmarks)
11. [Design Principles & Constraints](#11-design-principles--constraints)
12. [Go-to-Market & Distribution](#12-go-to-market--distribution)
13. [Open Questions & Next Steps](#13-open-questions--next-steps)

---

## 1. The Downloader Project

**What it is:** A Rust-based academic paper downloader with 10 site-specific resolvers (Wiley, ACM, Springer, Elsevier, MDPI, arXiv, PMC, S2, Crossref, bare-ID), a Tauri desktop app (Svelte 5 frontend), and a CLI.

**Current state (2026-03-29):** All 11 epics complete. 864 tests passing. NFR gate: PASS. Coverage ~89.5%.

**What it has that others don't:**
- 10 site-specific resolvers encoding months of publisher quirk knowledge
- Trust layer: robots.txt compliance, rate limiting, structured error types
- Auth/cookie flow for paywalled content (Wiley, ACM via browser-like headers)
- Bare-ID resolution (PMC, PMID, arXiv plain-text input)
- Clean lib/bin split, 864 tests

**Strategic role:** Downloader is the PDF acquisition layer that feeds any tool needing full-text access to paywalled papers. 64% of scholarly literature is paywalled. Without Downloader, tools fall back to abstracts, degrading accuracy by ~16pp.

**Key files:**
| Purpose | Path |
|---|---|
| Resolver registry | `downloader-core/src/resolver/mod.rs` |
| Shared S2 helpers | `downloader-core/src/resolver/semantic_scholar.rs` |
| Resolver error enum | `downloader-core/src/resolver/error.rs` |
| Critical regression tests | `downloader-cli/tests/critical.rs` |
| Tauri app commands | `downloader-app/src-tauri/src/commands.rs` |
| CI quality gates | `.github/workflows/phase-rollout-gates.yml` |
| Sprint status | `_bmad-output/implementation-artifacts/sprint-status.yaml` |

---

## 2. Products Being Built

Three candidate products, all in `_product/`. All completed Phase A (automated pain discovery) on 2026-04-12. Phase B (stimulus interviews) pending for all three.

### 2.1 Citation Verifier (H4)

| Attribute | Value |
|---|---|
| Layer | Independent (no prerequisites) |
| Pain / Effort / Comp | 5 / 2 / 3 |
| Build estimate | 2 weeks |
| Distribution | Claude Code skill |

**Problem:** 20-25% citation error rate. 80% of authors don't read the full text of papers they cite.

**Solution:** Given a draft paragraph + Zotero item IDs, verify each citation against full text. Return per-citation verdict with evidence quote and page number.

**MVP scope:** 3-class verdict: `supported` / `contradicted` / `not_found`. If full text unavailable: return `unverifiable` (never classify from abstract). `partially_supported` deferred to Phase 2 (requires rewrite prompt).

**Moat:** Downloader's resolver pipeline. Without full-text access, classification degrades from 91-95% F1 (gold retrieval) to 63% (open deployment). Competitors without paywalled PDF retrieval degrade on exactly the papers that matter most.

**Compliance framing:** ICMJE 2025 now requires authors to ensure "summaries correctly represent the original research findings." First major style body with claim-to-source accuracy in writing.

**Positioning:** Pre-submission QA tool, not a drafting aid. The "night before submission" pattern is the primary use case.

### 2.2 Research Wiki Compiler (H13 + H17)

| Attribute | Value |
|---|---|
| Layer | Foundation — everything downstream depends on this |
| Pain / Effort / Comp | 5 / 1.5-2 / 2 |
| Build estimate | 4 weeks to MVP (30 papers) |
| Distribution | Claude Code skill pack |

**Problem:** Researchers accumulate papers but cannot synthesize across them. PKM systems collapse at 50-200 papers due to manual maintenance burden.

**Solution:** Compile pipeline: Zotero library → LLM-maintained personal research wiki with entity pages, backlinks, and annotation integration.

**Architecture (4-phase cycle):**
1. **Ingest** — Zotero MCP → raw/ staging (metadata + fulltext + annotations per paper)
2. **Compile** — Sonnet subagents extract claims, methods, concepts; annotations as first-class input (H17)
3. **Orchestrate** — Opus creates/updates entity pages, adds backlinks, resolves conflicts
4. **Lint** — periodic audit for contradictions, gaps, broken links, stale articles

**Phase A key finding:** Discriminant resolved — it's a structure problem, not retrieval. "Most researchers already know how to find papers. The hard part is making sense of them."

**Field scope:** MVP targets empirical fields (biology, social science, medicine). Formula-heavy content flagged as "quality uncertain" (MinerU CDM 66.9 on math equations).

**Scale ceiling:** ~200 papers before lint phase becomes mandatory.

**Unlocks:** H2 (Research Memory), H6 (Backlog Triage), H11 (Context Agent), H14 (Draft Audit), H16 (Fine-tune), H18 (Epistemic Versioning), H19 (Argument Graph).

### 2.3 Argument Graph Layer (H19)

| Attribute | Value |
|---|---|
| Layer | Built on Research Wiki Compiler |
| Pain / Effort / Comp | 5 / 3 / 2 |
| Build estimate | 1 week after wiki MVP |
| Distribution | Claude Code skill + research paper |

**Problem:** #1 unsolved pain: "AI tools aggregate, don't reason — information engines, not insight engines." Current tools connect pages by topic. None answer "what's the strongest counterargument to my claim in section 3?"

**Solution:** Semantic layer where claims are nodes and logical relationships are typed edges: `supports`, `contradicts`, `qualifies`, `extends`, `replicates`, `fails_to_replicate`.

**Phase A key finding:** Reasoning gap is real and systemic. 183 Obsidian votes for typed links. Non-replicable papers cited 153-300x more. Manual workarounds (Miro, whiteboards) confirm the pain. Scite (2M users) validates typed-citation demand but operates at corpus level only — H19 fills the personal-library gap.

**Research contribution:** Publishable as systems paper: *"Argument-Aware Personal Research Wikis: LLM-Compiled Claim Graphs from Academic PDF Annotations."*

**Luhmann grounding:** Typed edges implement what Luhmann called the Kommunikationspartner property — the system's ability to surprise by surfacing connections never consciously planned.

---

## 3. Hypotheses (H1–H20)

### Scoring Key
Pain 1-5 (depth of frustration), Effort 1-5 (1=easy), Competition 1-5 (1=nobody, 5=saturated)

| # | Name | Pain | Effort | Comp | Status | Type |
|---|---|:---:|:---:|:---:|---|---|
| H1 | Glue Layer (Zotero+Obsidian+Overleaf) | 4 | 2 | 3 | **Dead** — MCP ecosystem saturated | Config |
| H2 | Research Memory | 5 | 2 | 2 | Falls out of H13 query+enhance phase | Usage pattern |
| H3 | Systematic Review Autopilot | 5 | 3 | 5 | **Dead** — Elicit + ASReview own this | Contribution |
| **H4** | **Citation Verifier** | **5** | **2** | **3** | **Phase A complete; ships first** | **Product** |
| H5 | Research Ops for Labs | 2 | 4 | 2 | **Killed** | — |
| H6 | Reading Backlog Triage | 4 | 1 | 3 | Trivial add-on after H13 | Usage pattern |
| H7 | Living Literature Review | 4 | 2 | 3 | Lint add-on to H13 | Add-on |
| H8 | Methodology Finder | 3 | 2 | 2 | Low priority | PR to S2/Elicit |
| H9 | Research Onboarding | 3 | 1 | 3 | Low priority | Config |
| H10 | Negative Results Platform | 4 | 5 | 2 | **Killed** — incentive problem | — |
| H11 | Research Context Agent | 5 | 1 | 2 | Nearly free after H13 | Usage pattern |
| H12 | Source-Grounded Writing Copilot | 5 | 2 | 3 | Builds on H13 wiki | Product |
| **H13** | **Personal Research Wiki** | **5** | **1.5-2** | **2** | **HIGHEST — substrate for everything** | **Infrastructure** |
| H14 | Draft Audit Agent | 5 | 2 | 2 | Builds on H13 + H19 | Product |
| H15 | Field Radar + Personal Context | 4 | 4 | 1 | Future — technically hardest | Research |
| **H16** | **Wiki-as-Behavior-Training** | **4** | **3** | **2** | **Go/no-go probe after H13 MVP** | **Research paper** |
| **H17** | **Annotation-Centric Wiki** | **4** | **2** | **1** | **Built into H13 from day 1** | **Compile mode** |
| **H18** | **Epistemic Versioning** | **4** | **1** | **1** | **2-day add-on to H13** | **Query mode** |
| **H19** | **Argument Graph Layer** | **5** | **3** | **2** | **Primary research contribution** | **Research paper + product** |
| **H20** | **Companion Researcher Agents** | **5** | **2.5** | **1** | **After H19; strongest viral mechanism** | **Research paper + product** |

### Detailed Hypothesis Summaries

**H13 — Personal Research Wiki:** Zotero → LLM-maintained markdown wiki → queryable by any agent. Replaces GraphRAG with Karpathy's LLM wiki pattern. At 50-200 papers, markdown wiki outperforms GraphRAG (inspectable, git-versioned, no vector DB). Every downstream hypothesis depends on this.

**H16 — Wiki-as-Behavior-Training:** Use compiled wiki as training data to fine-tune a model that reasons like a researcher in your field — not facts (RAG handles that) but field reasoning style (gap identification, methodological skepticism). Three-layer separation: facts→wiki (RAG), task adaptation→memory (Memento), field reasoning style→weights (H16). Competition raised to 2 after Memento (Aug 2025) proved task adaptation solvable via memory alone.

**H17 — Annotation-Centric Wiki:** Your Zotero annotations (highlights, notes) as the primary compile unit, not paper summaries. Builds into H13 compile prompt from day one. Annotations are the most token-efficient compile input (pre-curated by researcher salience judgment). H17 is the viral mechanism — without it, output is "just another LLM summary" (qaadika criticism); with it, output carries personal intellectual signature worth sharing.

**H18 — Epistemic Versioning:** git history of wiki = queryable record of intellectual development. "How did my understanding of X change over 18 months?" Unique temporal lane no memory framework covers (Mem0/Letta/Hindsight all overwrite/append/decay — none track deltas). Prototype opportunity: this project's own memory wiki already has the git history to test H18.

**H19 — Argument Graph Layer:** Claims as nodes, typed logical edges (supports/contradicts/qualifies/extends). The primary research contribution candidate. Directly attacks #1 pain point. Competitive gap confirmed: no system has typed edges (A-MEM, MAGMA, PRIME, Zettelgarden all use similarity-based linking). NLP primitives now available (FEVERFact for claim extraction, Counterclaims for contradicts detection, Contradiction Detection in RAG for validation).

**H20 — Companion Researcher Agents:** Instantiate an expert peer from a researcher's public corpus (papers + citation graph + methodological stances). Cross-KB synthesis between your wiki and theirs reveals knowledge gaps. Effort dropped to 2.5 (persona = context file + S2 API). H20 + H19 = argument graph with external expert nodes. Strongest viral mechanism: "what would [famous researcher] say about your work?"

---

## 4. New Findings (N1–N12)

From two discovery rounds (2026-04-06, 2026-04-11).

| # | Finding | Pain | Effort | Comp | Relationship |
|---|---|:---:|:---:|:---:|---|
| N1 | Extend Better Notes with LLM compile | 5 | 2 | 2 | Hands win to BN maintainer — worst moat |
| N2 | CHI 2026 Tools for Thought paper (H17+H18) | 4 | 3 | 2 | Workshop venue |
| **N3** | **Counterclaim-aware argument graph** | **4** | **4** | **1** | Best research contribution; primitives ready |
| N4 | Wiki-Memvid bridge for academic corpora | 3 | 3 | 2 | Adjacent |
| N5 | Cold-start hybrid compilation | 4 | 2 | 1 | Handles light-annotator users |
| N6 | Legal personal-precedent wiki | 4 | 4 | 2 | Adjacent market (see framing) |
| N7 | vitaLITy 2 hybrid (discovery → wiki) | 3 | 3 | 2 | Discovery-first then compile |
| **N8** | **Density Threshold Characterization** | **4** | **3** | **1** | Empirical study; independently publishable |
| N9 | Collaborative Argument Graph | 4 | 4 | 1 | Social layer on H19; multi-user infra heavy |
| **N10** | **Calibrated Claims (Bayesian propagation)** | **4** | **3** | **1** | Extends H19; AGM grounding |
| **N11** | **Anti-Library (structural gap mapping)** | **5** | **2** | **1** | Falls out of H19 topology; highest pain |
| N12 | Argument Path Navigation | 4 | 2 | 2 | Sharpens H19 Phase 4 + H12 |

**Top picks:** N11 (highest pain, lowest effort, falls out of H19), N10 (unique formal grounding), N3 (best pure research contribution), N8 (independently publishable empirical study).

---

## 5. Dependency Map & Build Order

### Layer Structure

```
LAYER 0 — Independent
  H4 (Citation Verifier) → ship now

LAYER 1 — Foundation
  H13 (Personal Research Wiki) — everything below depends on this

LAYER 2 — Built into H13 (zero additional build)
  H17 (Annotation-Centric Compile) — in compile prompt from day 1
  H2  (Research Memory), H6 (Backlog Triage), H11 (Context Agent) — usage patterns

LAYER 3 — After H13 is running
  H18 (Epistemic Versioning) — 2 days; unique temporal lane
  H14 (Draft Audit) — needs provenance + entity pages
  H12 (Writing Copilot) — queries wiki
  H16 (Behavior Fine-tune) — 1-week probe, then go/no-go

LAYER 4 — After H13 + H19
  H19 (Argument Graph) ← PRIMARY CONTRIBUTION
  N11 (Anti-Library) — fastest win off H19 (read-only topology)
  N10 (Calibrated Claims) — Bayesian propagation
  N12 (Argument Path Navigation) — upgrades H12
  H20 (Companion Researcher Agents) — cross-KB diff
  N8  (Density Threshold Study) — empirical study
```

### Critical Path

```
H13 → H17(inline) → H19 → { N10, N11, N12, H20 }
```

### Build Order

```
Week 1-2:   H4  (independent; clears the decks)
Week 3-4:   H13 MVP — 30 papers, H17 built in
            → H18 starts accumulating automatically
Week 5:     H14, H12 (quick wins off substrate)
            H16 probe (1-week go/no-go)
Week 6-8:   H19 (primary contribution)
            → N11 first (lowest effort off H19; Pain 5)
            → N12 (upgrades H12)
            → N10 (adds formal epistemology layer)
Post-H19:   H20, N8 (empirical study)
```

### Kill List

H1 (dead — MCP saturated), H3 (Elicit owns), H5 (killed), H8, H9, H10 (incentive problem), H15 (future), N9 (multi-user infra heavy).

---

## 6. People & Intellectual Influences

### Andrej Karpathy

**Role:** Former OpenAI/Tesla. Independent researcher. Most credible voice on practical LLM deployment.

**Key projects:**
- **LLM Wiki (April 2026):** Single markdown idea file (intentionally abstract, not code). Three-layer architecture (raw/ → wiki/ → schema). Four-phase cycle (ingest, compile, query+enhance, lint). "A large fraction of my recent token throughput is going less into manipulating code, and more into manipulating knowledge." 1.7M views, 5+ community implementations within 2 weeks — all Claude Code skill packs.
- **AutoResearch (March 2026):** Agent runs 12 experiments/hour autonomously. ~11% efficiency after 2 days. 21K stars, 8.6M views.
- **microgpt:** Complete GPT in 200 lines, zero dependencies.

**Key positions:**
- "Context engineering" > prompt engineering. The bottleneck is curating what the agent knows, not writing better prompts.
- Phase change Dec 2025: went from 20% → 80% agent-driven coding.
- "Agentic Engineering" (not "Vibe Coding") — rigorous, expertise-required orchestration of agents.
- Dependency minimalism: "I'm more inclined to directly generate functionality using an LLM."
- Future vision: compiled wiki → synthetic training data → fine-tune personal model (= H16).

### Niklas Luhmann

**Role:** German sociologist (1927-1998). Creator of the original Zettelkasten method.

**Key concept — Kommunikationspartner:** The Zettelkasten as a communication partner that surprises its author. The surprise property emerged from structural density of typed, meaningful links — not similarity-based "related to." Without typed edges, you get retrieval. With them, traversal generates implicit reasoning chains the author never made explicit.

**Three note types:** Bibliographic (what he read) → H17 annotations. Permanent (what he thought) → H13 wiki entity pages. Index/outline (what he planned to communicate) → H20 companion outputs.

**Why this matters:** H19's typed edges are the technical implementation of Luhmann's Kommunikationspartner property. The four-layer model (Memory → Insight → Revelation → Communication) is gated at the Revelation layer on typed edges.

### Vannevar Bush

**Role:** American engineer. Director of OSRD during WWII. Author of "As We May Think" (1945).

**Key concept — Memex:** A personal knowledge device with associative trails between documents. Diagnosed the right problem (maintenance burden kills personal knowledge systems) but had no solution. Karpathy's LLM wiki = the Memex finally working. H13+H17+H18 together = the most complete Memex ever: what you read (H13), what you thought (H17), how your thinking changed over time (H18).

**User connection:** The user admires Bush and responds positively to Memex framing.

---

## 7. Concepts & Frameworks

### Context Engineering

**Definition (Karpathy):** "The delicate art and science of filling the context window with just the right information for the next step." More important than prompt engineering. The bottleneck has shifted from "agent doesn't understand what I want" to "agent doesn't have the context it needs."

**Empirical evidence:** Projects with well-maintained context files see 40% fewer agent errors, 55% faster task completion (Anthropic 2026 Agentic Coding Trends Report).

**Paradigm shift:** Imperative → Declarative. Developer's job is now encoding criteria, constraints, and context — agents derive the steps.

**Four-layer tool taxonomy:**
1. **Static files** (CLAUDE.md, LLM wiki) — lowest overhead, highest leverage
2. **MCP connectors** (5,000+ servers, 97M+ monthly SDK downloads) — universal tool-to-agent bridge
3. **Agent memory frameworks** (Mem0, Letta, Zep, Hindsight) — cross-session persistence
4. **Orchestration** (LlamaIndex, LangGraph, Semantic Kernel) — multi-step workflows

**Implication for hypotheses:** Most of H13 pipeline is assembly, not construction. H19 reframed as the primary context engineering contribution for research — typed logical structure gives agents fundamentally richer context than flat semantic similarity.

### PKM Failure Patterns (8 Lessons)

1. **Stratification** — systems fail because accumulated content from multiple eras becomes unnavigable, not because content decays
2. **Episodic crisis cost** — plugin breakage (not daily maintenance) triggers trust erosion and abandonment
3. **Fragile adopters** — academic users exit the category on trust breaks (Mendeley 2018 natural experiment), not migrate to next tool
4. **Compounding is aspirational; retrieval is actual** — lead with retrieval wins, not compounding narrative
5. **Deferral trap** — capture easier than processing → backlog grows → "mausoleum"; LLM compile directly attacks this
6. **Temporal mismatch** — PhD students' incentive structure doesn't fit compounding timeline; target postdocs/recurring-output researchers
7. **Collaborative collapse** — shared wikis default to two-tier (personal used / shared stale); reformat-for-sharing step never happens
8. **Jupyter success factors** — formalized existing behavior, peer-to-peer spread, open access, native shareability, GitHub rendering as viral mechanism

### Research Tool Gap Taxonomy (6 Types)

| Gap | What it implies |
|---|---|
| `retrieval_gap` | Better search/indexing solves it |
| `synthesis_gap` | Wiki compilation needed (H13) |
| `reasoning_gap` | Argument graph needed (H19) |
| `trust_gap` | Evidence display + unverifiable gate |
| `workflow_gap` | Distribution/UX problem, not feature |
| `abandonment` | Reveals maintenance cost — highest value signal |

### Pain Discovery Methodology

**Phase A (automated):** Corpus mining of forums/GitHub/blogs. Answers "does this pain exist?" Replaces the "do you have this problem?" portion of interviews.

**Phase B (3-4 interviews):** Stimulus-only. Shows live artifact. Skips all pain validation (Phase A answered those). Answers what corpus mining cannot: trust threshold, workflow fit, artifact-based surprise.

**Complaint specificity filter:** Only keep posts with specific workflow steps that fail. Discard general frustration.

### "Domain Expertise via Documents" Framing

**Critique:** "Personal academic knowledge management" is the wrong unit. The actual user complaints ("I forget what I read," "I can't see how my thinking changed") are not specific to academia.

**Better unit:** How does a person build domain expertise over time using documents? Includes lawyers (case law), investors (filings + thesis evolution), journalists (beat tracking), policy analysts (regulatory accumulation).

**Academic is the beachhead, not the destination.** Adjacent markets (law, investment) have higher willingness-to-pay and more visible failure costs.

### Four-Layer Agent Memory Model

| Layer | What it does | What enables it |
|---|---|---|
| Memory | Storing notes | Basic indexing |
| Insight | Pattern detection across notes | Dense linking |
| Revelation | Surprise — emergent from structure | Typed edges + density threshold |
| Communication | Output-facing knowledge synthesis | Output-oriented note type |

**Key insight:** The revelation layer is gated on typed edges. This is why H19 is architecturally load-bearing for the full system.

---

## 8. Competitors & Market Landscape

### Direct Competitors

| Competitor | What it does | Critical gap |
|---|---|---|
| **ZotAI** (€39.99) | Q&A over Zotero PDFs + annotations | Session-only; no persistent structure; answers don't accumulate |
| **PaperQA2** (Future-House) | Superhuman Q&A on papers; WikiCrow for gene summaries | No entity pages; no persistent wiki; WikiCrow is population-scale |
| **NotebookLM** (Google) | Q&A over uploaded sources | 50-source cap; notebook silo; "amnesiac by design"; no API/export |
| **Scite** (2M users, acquired by Research Solutions Dec 2023) | Typed citation classification (supporting/contrasting/mentioning) | Corpus-level only; not personal library; not connected to draft |
| **Elicit** ($33M, ~$18-22M ARR) | Structured extraction, systematic reviews | Not claim-level verification against a draft |
| **Atlas** | Auto-builds KG from uploads | Complete silo; no Zotero/Obsidian integration |
| **SemanticCite** (arXiv:2511.16198) | 4-class citation verdict system | Not a user-facing tool; degrades on paywalled content |
| **Argdown** | Structured argument notation | Manual; no LLM; not connected to Zotero |
| **Kialo** | Collaborative argument mapping | Public; not personal academic workflow |
| **IBM Debater** | Corpus-scale argument mining | **Sunset 2024**; not personal scale |

### Closest Threats

**Better Notes (windingwind/zotero-better-notes, ~4,000 GitHub stars):** Already has wiki-style `[[backlinks]]` + annotation capture inside Zotero. If v3 adds LLM compilation, gap #1 closes. Strategic stance: build *with* (contribute LLM compile upstream or build complementary skill pack), not against. Differentiate on H19/H18/H17.

**llmwiki.app (lucasastorian, Show HN April 2026):** First standalone Karpathy-pattern product (not a skill pack). Multi-format upload + Claude MCP. One MCP server away from Zotero ingest. Canary action: check weekly for Zotero integration.

### Agent Memory Systems (Competitive Map)

| System | Memory | Insight | Revelation | Communication | Typed Edges |
|---|---|---|---|---|---|
| A-MEM (NeurIPS 2025) | yes | — | — | — | **no** |
| MAGMA (Jan 2026) | yes | partial | — | — | **no** |
| PRIME (April 2026) | yes | yes | partial | — | **no** |
| Zettelgarden | yes | partial | — | — | **no** |
| **H13+H17+H19+H18+H20** | **yes** | **yes** | **yes** | **yes** | **yes** |

The typed edges column is the discriminator. No existing system has it.

### MCP Research Ecosystem (Crowded)

| Server | Coverage |
|---|---|
| PapersFlow MCP | 474M+ papers (OpenAlex + S2) |
| Zotero MCP | Your library + OA cascade |
| ZotPilot MCP | Zotero library semantic search |
| Scite MCP | 1.4B+ citations |
| ArXiv MCP | arXiv corpus |

### Free Agent Configurations (No Moat)

ARIS (Zotero + Obsidian + S2 + local PDFs), Claude Scholar (25 skills), Agent Client (Obsidian). All are markdown skills + MCP configs — they work, they're free, no moat.

### Moat Analysis

Feature moats are dead (AI compresses dev cycles). The SaaSpocalypse (Feb 2026): AI productivity gains accrue to users and model providers, not SaaS vendors.

Possible moats: data flywheel (only at scale), brand/trust in niche, regulatory/compliance (publisher licensing). Downloader's resolver pipeline is the specific moat for the citation verifier product.

---

## 9. Technologies & Infrastructure

### RAG Approaches (Decision Matrix)

| Corpus size | Query type | Best approach |
|---|---|---|
| 50-200 docs | Any | **LLM Wiki** |
| 200-500 docs | Multi-hop | GraphRAG |
| 500+ docs | Complex relational | GraphRAG |
| Any | High volume, low latency | Vector RAG |

**GraphRAG performance overstated:** Independent eval shows 39% win rate vs reported 66%. Community-GraphRAG with global search hallucinates on QA tasks.

**LLM Wiki wins at 50-200 papers:** No vector DB overhead, human-navigable, git-versioned, incremental compounding, inspectable. Upgrade path: add GraphRAG only if corpus exceeds ~300 papers.

### PDF → Markdown Tools (for H13 ingest)

| Tool | Strength |
|---|---|
| **MinerU** | Academic PDF focus; formula-aware; 86.2/100 OmniDocBench |
| **Docling** (IBM) | Production-grade; tables, figures, equations |
| **Marker** | Fast; good layout preservation |
| **MarkItDown** (Microsoft) | Office docs + PDF |
| **Zerox** | Vision-based (handles scanned papers) |

Formula CDM: MinerU 66.9, Mathpix 71.4 — neither reliable for formula-heavy content.

### Karpathy-Pattern Implementations (all April 2026)

| Repo | Stars | Key strength |
|---|---|---|
| llm-wiki-compiler | 85 | 84-90% token reduction; coverage indicators |
| wiki-skills | 20 | Cleanest 11-step ingest; bidirectional linking as first-class |
| llm-knowledge-bases | 9 | Model routing (Haiku/Sonnet/Opus); lint+evolve |
| karpathy-wiki | 1 | Two skills: research + project/code wiki |
| Understand-Anything | 7,874 | Interactive web dashboard (shows appetite) |

All four are Claude Code skills/plugins — validates "contribution as skill pack" direction.

### Agent Memory Frameworks

| Framework | Architecture | Benchmark |
|---|---|---|
| Mem0 | Vector store + KG | 49% LongMemEval |
| Letta (MemGPT) | 3-tier: core/archival/recall | — |
| Zep | Temporal + semantic hybrid | — |
| **Hindsight** | 4 parallel: semantic+BM25+KG+temporal | **91.4% LongMemEval** |

### NLP Primitives for H19

| Primitive | Paper | Use |
|---|---|---|
| Claim extraction | FEVERFact (arxiv:2502.04955) | 4.4K sentences, 17K claims, 6 metrics |
| Counterclaims | Counterclaims in Causality (arxiv:2510.08224) | "A does not cause B" negations |
| Contradiction detection | Contradiction in RAG (arxiv:2504.00180) | RAG-context validation → wiki-claim |

### Zotero MCP Tools

- `zotero_get_annotations(item_id)` — highlights + notes with page numbers
- `zotero_get_item_fulltext(item_id)` — full text for context
- `zotero_get_recent()` — recent items

---

## 10. Research Papers & Benchmarks

### Key Papers

| Paper | Relevance |
|---|---|
| **Memento** (arxiv:2508.16153, Aug 2025) | Proves task adaptation solvable via memory (87.88% GAIA); H16 must target field reasoning style, not task adaptation |
| **A-MEM** (arxiv:2502.12110, NeurIPS 2025) | Zettelkasten-inspired agent memory; similarity-based linking only |
| **MAGMA** (arxiv:2601.03236, Jan 2026) | Four-graph architecture; 45.5% higher reasoning; no typed argument edges |
| **PRIME** (arxiv:2604.07645, April 2026) | Gradient-free meta-level memory evolution; closest to revelation layer |
| **EvolveR** (arxiv:2510.16079) | Self-evolving LLM agents; same "adapt without weights" cluster as Memento |
| **SemanticCite** (arxiv:2511.16198) | Citation verification baseline; 66%/84% accuracy; no user-facing tool |
| **FEVERFact** (arxiv:2502.04955) | Claim extraction benchmark; use for H19 extraction quality |
| **paper2lkg** (WWW 2025) | PDF → local KG via LLM; potential H13 component |
| **GitEvo** (arxiv:2602.00410) | Git evolution analytics for knowledge bases; directly applicable to H18 |
| **OmniDocBench** (CVPR 2025, arxiv:2412.07626) | Document parsing benchmark; MinerU 86.2/100 |

### Key Benchmarks

| Benchmark | Score | Context |
|---|---|---|
| SciFact (citation verification) | 91-95% F1 (gold retrieval) → 63% (open) | H4 accuracy baseline |
| OmniDocBench (document parsing) | MinerU 86.2 overall, 66.9 formula CDM | H13 ingest quality |
| GAIA (agent tasks) | Memento 87.88% | H16 differentiation |
| LongMemEval (memory) | Hindsight 91.4%, Mem0 49% | Context engineering landscape |
| GraphRAG win rate | 39% (independent) vs 66% (self-reported) | Justifies LLM wiki over GraphRAG |

---

## 11. Design Principles & Constraints

### H13 Design Constraints (from PKM failure lessons)

1. **Minimize integration surface** — Zotero MCP as single integration point; no Better BibTeX or Obsidian Integration dependency
2. **Compile from existing annotation behavior** — accept messy, inconsistent, private-language annotations; never require new annotation discipline
3. **Lead with retrieval wins** — MVP demo delivers retrieval value in first session; compounding is retention story, not acquisition
4. **H17 annotation-centric mode is the viral mechanism** — without it, output is generic LLM summary; with it, output carries personal intellectual signature
5. **Compile output must be immediately useful** — no intermediate artifacts requiring manual second pass (attacks deferral trap)
6. **Don't target PhD students as primary adopters** — temporal mismatch; target postdocs/researchers with recurring synthesis obligations
7. **Don't target lab-scale as initial collaborative use case** — two-tier collapse pattern; single-user first
8. **Observability is first-class** — session-start digest showing what's compiled, what's raw, what's thin, what changed

### AI Verdict Design Principles (for H4)

- **Expertise split:** Novices → automation bias (accept wrong verdicts). Experts → algorithm aversion (re-verify everything after one error).
- **Fix:** Evidence + reasoning alongside verdict. Turns "re-read full paper" into "check one passage."
- **Unverifiable gate:** Confident wrong verdict > no verdict. Return `unverifiable` when inputs insufficient.
- **Intermediate verdicts need rewrite prompts** — bare "partially supported" is not actionable.

### Compile Prompt Constraints (from Gwern/Willison research)

Entity pages that are **link-maps** ("see also X, interesting connection to Y") do NOT travel. Entity pages that are **synthesis** ("Smith 2019 argues X; conflicts with Jones 2021's Y; implication Z") DO travel.

The format that travels: **evaluable claim + inspectable evidence + followable provenance.**

### H19 Design Constraints (from PKM failure analysis)

- Every inferred relationship needs confidence level
- Low-confidence relationships should not surface in default query results
- Inference chain must be visible ("I inferred `contradicts` because...")
- Claim relationships need recency signal — old edges may be superseded
- Build H19 for draft-audit use case (H14 integration) as primary; exploratory graph is secondary

---

## 12. Go-to-Market & Distribution

### Academic Software Buyer Map (3 Personas)

1. **Individual researcher** (PhD/postdoc) — deadline-triggered; $10-15/mo; fastest feedback loop
2. **PI / Supervisor** — past-embarrassment-triggered; lab budget; runs on students' drafts (auditor mode)
3. **Library** (institutional) — trial metrics required (50+ unique users in 4-8 week trial); 6-18 month cycle; librarian is champion

**Adoption pattern (Scite template):** Librarian discovers → library runs trial → measures engagement → purchases if threshold met → promotes via LibGuides.

**Turnitin analogy:** Individual use → evidence accumulates → institutional mandate → library purchases.

**Sequence:** Individual first (fastest feedback, generates usage data) → PI second (lab budget, QA framing) → Library last (requires existing institutional credibility).

### Viral Mechanisms

**Pre-H19:**
1. Reading Trace (Artifact B) — best zero-install encounter; standalone, self-contained; add "Compiled from X annotations" attribution
2. Lit Review Section (Artifact C) — highest ceiling via Twitter/X thread export; requires DOI resolution
3. The idea file path — publish method as blog post; spreads via HN/X independently of tool

**Post-H19:**
H19's navigable argument map is the strongest viral artifact of any hypothesis. A published argument map of a contested research area is immediately legible, hard to reproduce without the tool, and convertible to a research paper figure.

**H17 as viral mechanism:** qaadika's HN criticism resolved: "There's nothing personal about a knowledge base you filled by asking AI questions." H17 (annotation-centric mode) makes the output carry personal intellectual signature — that's the difference between "cleaned-up Wikipedia" and "your intellectual engagement with the papers."

---

## 13. Open Questions & Next Steps

### Phase B Questions (All Three Products)

**Citation Verifier:**
1. Will researchers act on `contradicted` with evidence quote, or re-read anyway?
2. Is the PI/supervisor a stronger buyer than the individual researcher?
3. "As I write" vs "one pass before submit" — which workflow moment dominates?

**Research Wiki Compiler:**
1. Do researchers trust entity pages enough to cite from them without re-reading source papers?
2. Does annotation-based entity page produce visible recognition vs full-text version?
3. At what library size does the wiki become "necessary" — 30? 100? 200?
4. Does the "surprise property" hold?

**Argument Graph Layer:**
1. Would researchers trust LLM-generated `contradicts` edges enough to act on them?
2. Does the argument graph produce surprise (Kommunikationspartner test)?
3. Is {supports, contradicts, qualifies, extends, replicates, fails_to_replicate} the right taxonomy?

### Technical Unknowns (for 30-Paper MVP)

1. Compile quality on academic PDFs (only tested on conversation sessions so far)
2. Annotation signal quality from real Zotero libraries
3. Index behavior at 200 files
4. Entity consolidation across multiple papers covering same concept (hardest unsolved problem)
5. Cross-session query performance without full context load

### Wiki Maintenance Steps

1. **Lint phase** — audit session notes vs entity files; absorb N1-N7 into main table; flag stale files
2. **Apply H19 to hypotheses** — create `hypothesis_argument_graph.md` with typed edges between hypotheses (stress-tests H19)
3. **H13 prototype** — use this memory wiki as template; run compile on 5 Zotero papers
4. **H18 proof-of-concept** — run git diff analytics on this wiki's own history

### Competitive Monitoring

- **Better Notes:** Watch for LLM compile features, v3 milestone, any Karpathy references
- **llmwiki.app:** Check weekly for Zotero MCP integration, annotation parsing, academic metadata
- **Agent memory papers:** Monitor for typed-edge implementations (currently: nobody)

---

*This document compiles knowledge from 62 memory files covering ~375KB of research, analysis, and session notes accumulated over 2 weeks of intensive product discovery and hypothesis evaluation. Primary sources include academic papers, HN/Reddit threads, GitHub repos, and structured pain discovery runs.*
