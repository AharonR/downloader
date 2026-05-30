# Compile Prompt Specification

Constraints consolidated from: `h13_design_constraints.md`, `viral_mechanism_research.md`, `h13_pilot_learnings.md`, `entity_personal_research_wiki.md`, `wiki_compiler_research.md`, `entity_h17_annotation_wiki.md`.

## Input

Per paper, from Zotero MCP:
- `zotero_get_item_fulltext(item_id)` → full extracted text
- `zotero_get_annotations(item_id)` → highlights, notes, colors, page numbers, types

## Output Types

### Paper page (`papers/<slug>.md`)
One per paper. Structured summary with:
- Metadata (DOI, authors, publication date, source)
- Key claims (3-7 structured claim objects — see H19 schema below)
- Methods summary
- Results summary
- Limitations
- Relationships to cited work
- **Researcher annotations section** (H17) — highlights and notes with page provenance

### Entity page (`concepts/<slug>.md`)
One per concept/method/finding that appears across papers. Contains:
- Definition / description
- Evidence from papers (with backlinks to paper pages)
- **Researcher perspective section** (H17) — compiled from annotations, not paper text
- Coverage indicator (high/medium/low based on number of source papers + annotation density)
- Backlinks to all papers that mention this concept

## Core Constraints

### 1. Synthesis, not organization (from Gwern/Willison research)

Entity pages that are **link-maps** do NOT travel:
> "see also paper X, interesting connection to Y"

Entity pages that are **synthesis** DO travel:
> "Smith 2019 argues X; this conflicts with Jones 2021's finding Y; implication for field Z"

**The format that travels**: evaluable claim + inspectable evidence + followable provenance.

**Test**: Can a stranger read an entity page and evaluate the claims without reading the source papers? If no, the compile prompt needs redesign.

### 2. Annotations as first-class input (H17, Constraint 2)

Accept annotations as-is — raw highlights, inline comments, margin notes, private-language reactions ("important!", "???", ":("). The compile phase must extract signal from messy, inconsistent annotations.

**Never require** researchers to annotate differently before the tool works. No "use these tags" or "restructure your annotation practice" preconditions.

**Annotation types to handle:**
| Type | Signal | Compile action |
|---|---|---|
| Text highlights | Researcher salience judgment | Primary evidence for entity pages |
| Sticky notes | Direct expression of researcher thinking | "Researcher commentary" sections |
| Color coding | User-defined conventions (yellow=key, red=disagree) | Map to claim confidence if user defines scheme |
| Tags on annotations | Categorical markers | Map to entity page categories |

### 3. Every claim traces to source (from trust break pattern)

Every factual claim in an entity page must include:
- Source paper reference (backlink)
- Page number
- Whether derived from full text or from researcher annotation

This is the primary defense against the "bad wiki article becomes a prior that poisons future generations" failure mode.

### 4. Immediately useful output (Constraint 5, deferral trap)

Compile output must be navigable without further manual curation. Entity pages with backlinks are immediately useful. Flat summaries per paper that require a separate synthesis pass are not.

**Two-step processes where the second step is manual will default to deferral.**

### 5. H19 claim extraction (built into compile from start)

For each paper, extract 3-7 key claims as structured objects:
```yaml
claims:
  - id: claim_001
    text: "IV estimation is biased when instrument is weak"
    type: empirical_finding  # or theoretical_claim, methodological_claim
    confidence: high
    source_page: 12
    annotation_support: true  # researcher highlighted this
```

This costs negligible extra effort at compile time and provides the substrate for H19's argument graph layer.

### 6. Annotation density as salience signal

Papers with dense annotations → entity pages get richer "researcher perspective" sections. Papers with no annotations → entity pages based on full-text extraction only, marked as "not annotated."

This gives the researcher immediate visibility into which parts of the wiki reflect their active reading vs. automated extraction.

## Parallel Compile Paths (2026-04-13)

Run both paths in parallel for every paper; diff the outputs. **The diff is the product.**

### Three input paths

| Path | Input | Output label | Cost |
|---|---|---|---|
| **Annotation-rich** | annotations + full text | `annotation-grounded` | low |
| **Annotation-only** | annotations alone | `annotation-grounded` | very low |
| **Cold-start** | full text only | `model-summarized` | high |

If annotations exist, run annotation-rich AND cold-start in parallel. If no annotations, run cold-start only, label output `model-summarized`.

### Diff outputs → three signals

| Diff result | Meaning | Action |
|---|---|---|
| Both paths agree | High-confidence claim | Promote with confidence weight |
| Annotation path only | Researcher-salient; model missed it | Keep; flag as "annotation-surfaced" |
| Model path only | Model surfaced; you didn't highlight it | Surface as "you may have missed this" per paper |
| Paths contradict | Compile conflict | Surface immediately; do not silently resolve |

### Output labels matter downstream

`annotation-grounded` claims are weighted higher in H19 (argument graph). A claim the researcher explicitly highlighted is higher-confidence evidence than one the model autonomously extracted. Claims inherit their label through to entity pages and graph edges.

### H18 extension

The per-paper divergence rate (how often annotation judgment differs from model read) is itself a queryable metric: "How idiosyncratic is my research perspective on X?" Track it in the git history of wiki pages.

### At scale

Running both paths doubles full-text token cost. At 30-paper MVP: negligible. At scale: expose as user setting — "thorough mode" (both paths) vs "fast mode" (annotation-only with cold-start fallback).

## Compile Workflow

```
Per paper (Sonnet subagent):
1. Read fulltext + annotations
2a. Annotation path: compile from annotations (+ full text for gap-fill context)
2b. Model path: compile from full text only
3. Diff outputs: surface agreements (high confidence), annotation-only (salient), model-only ("missed"), contradictions (flag)
4. Identify entity candidates (concepts/methods that should have their own pages)
5. Output: structured paper page (with provenance labels) + entity update instructions

Orchestration pass (Opus):
1. For each entity update instruction:
   a. If entity page exists → merge new evidence, update coverage indicator
   b. If entity page doesn't exist → create new page
   c. Add backlinks in BOTH directions (paper→entity and entity→paper)
2. Bidirectional backlink audit (wiki-skills step 7):
   Scan ALL existing pages for mentions of new entities; add backlinks where missing
3. Update _index.md and _sources.md
4. Flag potential entity consolidation (same concept from different papers)
```

## Quality Constraints

| Field type | Compile quality | Action |
|---|---|---|
| Biology, social science, medicine | Acceptable (WikiCrow validates) | Full compile |
| Computer science | Good for prose; variable for code/benchmarks | Full compile with code-block passthrough |
| Mathematics, physics | Formula CDM ~67% (MinerU/Mathpix) | Flag as "quality uncertain" on equation-heavy sections |

## Anti-Patterns

- **Annotation dump**: listing highlights without synthesis → fails Constraint 1
- **Paper summary only**: summarizing each paper independently without cross-paper connections → fails to compound
- **Schema-dependent input**: requiring specific annotation tags or colors → fails Constraint 2
- **Intermediate artifact**: producing raw extraction that needs manual synthesis → fails Constraint 4
- **Sourceless claims**: assertions without paper+page provenance → fails Constraint 3
