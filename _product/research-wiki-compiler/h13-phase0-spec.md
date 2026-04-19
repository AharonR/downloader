# H13 Phase 0 — Research Wiki Compiler (Open Source Preview)

> **Status:** Ready for development  
> **Date:** 2026-04-13  
> **Goal:** Ship a working Zotero → LLM wiki pipeline as an open source Claude Code skill pack before llmwiki.app adds Zotero integration (~4-6 week window)

---

## 1. What It Is

A Claude Code skill pack that compiles a researcher's Zotero library into a personal markdown wiki: entity pages, paper summaries, backlinks, and claim provenance. Built on the Karpathy LLM wiki pattern, adapted for academic PDFs and Zotero annotations.

**The gap it fills:** Every existing Zotero+LLM tool uses RAG (Q&A per session, no persistent structure). No tool implements LLM compilation for academic papers with annotation awareness. This is the first.

**Distribution:** GitHub + Claude Code skill pack. Open source, MIT licensed. Not a SaaS product — researchers run it on their own Zotero library.

---

## 2. Scope (Phase 0)

### In scope
- Zotero library → markdown wiki (entity pages + paper pages)
- Annotation-aware compilation (H17 built in from day one)
- Parallel compile paths with output diff
- Session-start report (observability)
- 20-50 paper target corpus
- Empirical fields: biology, social science, medicine, computer science prose
- Output: Obsidian-compatible markdown vault (or any plain markdown directory)
- Single user, local execution

### Out of scope (Phase 0)
- Argument graph / typed edges (H19 — Layer 4; requires wiki to exist first)
- Epistemic versioning queries (H18 — accumulates automatically but no query UI yet)
- Collaborative / multi-user (fails at the sharing-step problem)
- Formula-heavy content (math, physics equations — CDM 66.9%, flagged as "quality uncertain")
- Fine-tuning / H16
- GUI / web interface
- Paywalled PDF retrieval UI (Downloader handles this separately; note the gap in README)

---

## 3. Dependencies

| Dependency | Role | Risk |
|---|---|---|
| **Zotero MCP** (`54yyyu/zotero-mcp`) | Single integration point — all paper content, metadata, annotations | Single maintainer; Zotero API changes |
| **Claude API** (Sonnet for compile, Opus for orchestration) | LLM compilation | Cost (manageable at 20-50 papers) |
| **Zotero desktop app** | Must be running locally for MCP server to function | Standard dependency |
| **Downloader** (optional) | PDF acquisition for paywalled papers (64% of scholarly literature) | Not required for Phase 0; document the gap |

**No other dependencies.** Do not add Better BibTeX, Obsidian Integration plugin, or any intermediary that creates additional failure surfaces.

---

## 4. Architecture

```
Zotero Library (PDFs + metadata + annotations)
      │
      ▼  Zotero MCP (single integration point)
  zotero_get_recent()          → detect new/changed papers
  zotero_get_item_fulltext()   → full extracted text
  zotero_get_annotations()     → highlights, notes, colors, page numbers
      │
      ▼  INGEST LAYER
  raw/<slug>/
    metadata.md     ← title, DOI, authors, date, tags, collections
    fulltext.txt    ← full extracted text
    annotations.md  ← highlights + notes with page provenance
  _index.md         ← hash-tracked manifest (skip unchanged papers)
      │
      ▼  COMPILE LAYER (parallel paths, one Sonnet subagent per paper)
  [annotation path]  annotations + fulltext for gap-fill context
  [model path]       fulltext only
  → diff outputs → confidence-weighted paper page
      │
      ▼  ORCHESTRATION (Opus, one pass per compile run)
  create/update wiki/papers/<slug>.md
  create/update wiki/concepts/<slug>.md
  add backlinks (bidirectional)
  entity consolidation (same concept from multiple papers)
  update _index.md and _sources.md
      │
      ▼  OUTPUT VAULT
  wiki/
    papers/          ← one page per paper
    concepts/        ← entity pages: methods, findings, concepts, authors
    _index.md        ← master list with hashes + coverage indicators
    _sources.md      ← source → wiki article mapping
    _session.md      ← session-start report (auto-generated)
    _queue.md        ← papers annotated but not yet compiled
      │
      ▼  LINT (periodic, on demand)
  stale articles, broken backlinks, detected contradictions, orphan pages
```

---

## 5. Output File Formats

### Paper page (`wiki/papers/<slug>.md`)

```markdown
---
title: "Paper Title"
doi: 10.xxxx/xxxxx
authors: [Smith J, Jones A]
year: 2023
zotero_id: XXXXXXXX
compiled: 2026-04-13
compile_mode: annotation-grounded  # or model-summarized
annotation_count: 12
---

# Paper Title

## Key Claims
- **Claim 1** (annotation-grounded, p.4): "IV estimation is biased when instrument is weak"
  - Confidence: high | Type: empirical_finding
- **Claim 2** (model-surfaced, p.12): "..."
  - Confidence: medium | Type: methodological_claim

## Methods
...

## Results
...

## Limitations
...

## Researcher Annotations
<!-- H17: compiled from your Zotero highlights and notes -->
- p.4 (yellow): "..." — [your sticky note if present]
- p.7 (red): "..." — disagreement marker

## Relationships to Cited Work
- Extends: [[jones-2021-iv-methods]]
- Contradicts: [[smith-2019-ols-bias]]

## "You May Have Missed" (model-surfaced only)
- p.9: "..." — model flagged this; not in your annotations

## Compile Conflicts
- [none]
```

### Entity page (`wiki/concepts/<slug>.md`)

```markdown
---
entity: Instrumental Variables
type: method
coverage: high  # high/medium/low based on paper count + annotation density
updated: 2026-04-13
---

# Instrumental Variables

## Definition
...

## Evidence from Papers
- [[smith-2023-iv-bias]]: "IV estimation is biased when instrument is weak" (annotation-grounded, p.4)
- [[jones-2021-iv-methods]]: "..." (model-summarized, p.12)

## Researcher Perspective
<!-- compiled from your annotations across all papers mentioning this concept -->
...

## Open Questions
<!-- from lint phase -->
...

## Backlinks
[[smith-2023-iv-bias]] · [[jones-2021-iv-methods]] · [[chen-2022-weak-instruments]]
```

---

## 6. Compile Paths (Parallel)

Run both paths for every paper that has annotations. Diff the outputs. **The diff is the product.**

### Three input modes

| Mode | Condition | Input | Label |
|---|---|---|---|
| **Annotation-rich** | Annotations exist | annotations + fulltext (gap-fill) | `annotation-grounded` |
| **Annotation-only** | Annotations exist, fulltext unavailable | annotations only | `annotation-grounded` |
| **Cold-start** | No annotations | fulltext only | `model-summarized` |

When annotations exist: run annotation-rich AND cold-start in parallel.  
When no annotations: run cold-start only.

### Diff signals

| Result | Meaning | Action |
|---|---|---|
| Both paths produce same claim | High confidence | Promote; mark `annotation-grounded` if highlighted |
| Annotation path only | Researcher-salient; model missed it | Keep; mark `annotation-grounded` |
| Model path only | Model surfaced; not highlighted | Surface in "You May Have Missed" section |
| Paths contradict | Compile conflict | Surface as conflict; do not silently resolve |

### Output labels propagate

`annotation-grounded` vs `model-summarized` labels attach to individual claims and propagate to entity pages. When H19 (argument graph) is built, these labels weight claim confidence in the graph.

---

## 7. Compile Prompt Constraints

These are non-negotiable constraints on the compile prompt output. Violating any of them produces output that fails one of the known PKM failure modes.

1. **Synthesis, not organization.** Entity pages must be evaluable by a stranger without reading the source papers. "See also X" is not synthesis. "Smith 2019 argues X; conflicts with Jones 2021 because Y; implication Z" is.

2. **Annotations as-is.** Accept raw highlights, inline comments, private-language reactions ("important!", "???"). Never require structured annotation input.

3. **Every claim traces to source.** Each factual claim = source paper reference + page number + compile mode label. No sourceless assertions.

4. **Immediately navigable output.** Entity pages with backlinks are immediately useful. Extracted text requiring a second human synthesis pass is not. Two-step processes where step two is manual default to deferral.

5. **H19 claim schema built in.** Extract 3-7 structured claims per paper:
   ```yaml
   - id: claim_001
     text: "..."
     type: empirical_finding  # or theoretical_claim, methodological_claim
     confidence: high         # high/medium/low
     source_page: 12
     annotation_support: true
     compile_mode: annotation-grounded
   ```
   This costs negligible extra at compile time and is the substrate for H19.

6. **Quality flagging.** Formula-heavy sections (math, chemistry): flag as "quality uncertain." Do not hallucinate equation content.

### Anti-patterns (compile prompt must avoid)

- **Annotation dump**: listing highlights without synthesis → fails constraint 1
- **Paper summary only**: summarizing each paper independently, no cross-paper connections → fails to compound
- **Schema-dependent input**: requiring specific annotation tags or colors → fails constraint 2
- **Intermediate artifact**: raw extraction requiring manual synthesis → fails constraint 4
- **Sourceless claims**: assertions without paper+page provenance → fails constraint 3

---

## 8. Ingest Logic

```
1. Call zotero_get_recent() → list of item IDs
2. For each item:
   a. Compute hash of (fulltext + annotations)
   b. Compare against _index.md entry
   c. If new or changed → add to compile queue
   d. If unchanged → skip
3. For each item in compile queue:
   a. zotero_get_item_fulltext(item_id) → fulltext.txt
   b. zotero_get_annotations(item_id) → annotations.md
   c. Extract metadata (title, DOI, authors, year, tags) → metadata.md
   d. Write to raw/<slug>/
   e. Update _index.md entry with new hash + status: pending_compile
```

**Incremental by design.** Re-running ingest never re-processes unchanged papers. Hash-tracked in `_index.md`.

---

## 9. Session-Start Report (`wiki/_session.md`)

Generated automatically at the start of every session, before any query. Answers: "what is the current state of your wiki?"

```markdown
# Session Report — 2026-04-13 14:32

## New since last session
- 3 papers ingested: [smith-2023-iv-bias], [jones-2021-iv-methods], [chen-2022-weak-instruments]
- 2 entity pages created: [[Instrumental Variables]], [[Weak Instruments]]
- 1 entity page updated: [[Econometrics Methods]]

## Raw queue (annotated but not compiled)
- jones-2024-replication (annotated 2026-04-12, 8 highlights)
- chen-2023-panel (annotated 2026-04-10, 3 highlights)

## Coverage
- High (5+ papers): [[Instrumental Variables]], [[Panel Data]]
- Medium (2-4 papers): [[Fixed Effects]], [[Measurement Error]]
- Stubs (1 paper): 14 concepts

## Staleness
- [[Regression Discontinuity]]: last updated 2026-03-10; 2 new papers reference it

## Open questions (from lint)
- "Does weak IV bias direction depend on correlation sign?" — no wiki entry answers this

## Contradictions (unresolved)
- [[Instrument Validity]]: smith-2023 and chen-2022 make conflicting claims about exclusion restriction testing

## Index health
- 0 broken backlinks
- 0 orphan pages
```

This report converts the wiki from a black box into a trusted tool. Without it, the user loses confidence in the system state and falls back to memory — the mechanism behind PKM abandonment.

---

## 10. Skill Pack Structure

```
zotero-wiki/           ← repository root
  SKILL.md             ← Claude Code skill definition
  README.md            ← user-facing install + quickstart (5 min to first compile)
  prompts/
    ingest.md          ← ingest phase prompt
    compile-annotation.md   ← annotation-path compile prompt
    compile-model.md        ← model-path compile prompt
    diff.md            ← output diff + conflict detection
    orchestrate.md     ← Opus orchestration prompt (entity pages + backlinks)
    lint.md            ← lint phase prompt
    session-report.md  ← session-start report prompt
  schema/
    SCHEMA.md          ← entity page + paper page format spec
    claim-schema.yaml  ← H19-ready claim object format
  wiki/                ← output vault (gitignored or user-owned)
    .gitkeep
```

---

## 11. Design Constraints (Non-Negotiable)

Derived from documented PKM failure modes. Violating these produces a tool that works in demos and fails in practice.

| # | Constraint | Failure mode it prevents |
|---|---|---|
| C1 | Zotero MCP as the **only** integration point | Episodic crisis cost (plugin chain breakage → trust break → abandonment) |
| C2 | Compile from **existing** annotation behavior; never require new habits | Cold-start failure; Zettelkasten/second-brain pattern |
| C3 | Lead with **retrieval wins** in the first session, not compounding narrative | Deferral trap; churn from unmet expectations |
| C4 | H17 (annotation-centric mode) **built in from day one** | Output indistinguishable from NotebookLM; no viral mechanism |
| C5 | **Immediately navigable** compile output; no second manual synthesis pass | Deferral trap; mausoleum failure |
| C6 | **Session-start report** generated before any query | Observability gap → trust failure → abandonment |
| C7 | **Single user** design; no shared wiki for Phase 0 | Collaborative collapse; two-tier failure |

---

## 12. MVP Success Criteria

The Phase 0 prototype is validated when:

1. **Compile runs end-to-end** on 20-30 real Zotero papers without manual intervention
2. **Entity pages pass the stranger test**: a colleague who hasn't read the papers can evaluate the claims
3. **Backlinks are bidirectional**: every paper page links to its concepts; every concept page links back to its papers
4. **Session-start report accurately reflects wiki state** (no manual inspection needed to answer "what's in there?")
5. **Annotation-grounded pages are visibly different** from model-summarized ones (the researcher can tell the difference at a glance)
6. **Cost is acceptable**: 30-paper compile ≤ $1.00 in API tokens (estimated: ~$0.06-0.30 at Sonnet pricing)

---

## 13. Build Sequence

### Phase 0a — Ingest + single compile path (3-5 days)
1. Set up skill pack scaffold (SKILL.md, SCHEMA.md, prompts/)
2. Build ingest layer: Zotero MCP → raw/ staging with hash tracking
3. Build annotation-path compile prompt (H17 built in)
4. Build model-path compile prompt
5. Test on 5 papers; inspect entity page quality

### Phase 0b — Orchestration + parallel paths (3-5 days)
6. Build Opus orchestration pass (create/update entity pages, bidirectional backlinks)
7. Build parallel path diff (confidence weighting, "you may have missed" section, conflict detection)
8. Test on 20-30 papers

### Phase 0c — Observability + release (2-3 days)
9. Build session-start report
10. Build basic lint (broken backlinks, orphan pages, stale articles)
11. Write README (install + quickstart + 5-minute demo)
12. Cut v0.1.0 release on GitHub

**Total: ~2 weeks to working open source preview**

---

## 14. Open Questions

| Question | Blocking? | How to resolve |
|---|---|---|
| Does Zotero MCP handle papers with no stored PDF? | Yes — needed for cold-start path | Test locally; document graceful degradation |
| What is the Sonnet token cost on a 40-page academic PDF? | Informs cost estimate | Run 3-5 test papers before full build |
| Does annotation-rich path produce meaningfully different output from model-only? | Yes — core H17 assumption | Run side-by-side on 5 papers; inspect diff |
| How to handle papers with 0 annotations but the user wants them in the wiki? | No — cold-start covers this | Cold-start path; `model-summarized` label |
| Zotero MCP maintainer risk — what if it breaks? | Long-term | Document fallback: Zotero API directly; note in README |

---

## 15. What This Unlocks

Phase 0 ships a working wiki substrate. Everything downstream follows:

| What | When | Cost |
|---|---|---|
| H18 (Epistemic Versioning) | Starts accumulating in git history automatically | 0 additional build |
| H2/H6/H11 (memory, triage, context agent) | Any agent that reads markdown files | 0 additional build |
| H19 (Argument Graph) | After wiki has 20+ entity pages | ~1 week |
| N11 (Anti-Library) | After H19 | 1 day |
| H4 (Citation Verifier) | Independent; can run in parallel | 2 weeks |
