# Argument Graph Layer — Product Brief

**Hypothesis:** H19 (primary) + N10, N11, N12 (downstream)  
**Layer:** Builds on Research Wiki Compiler (H13 must exist first)  
**Build estimate:** 1 week after wiki MVP  
**Pain:** 5/5 | **Effort:** 3/5 | **Competition:** 2/5

---

## Problem

The #1 unsolved pain across researcher communities: "AI tools aggregate, they don't reason.
They're information engines, not insight engines."

Current tools — including the research wiki (H13) — connect pages by *topic*.
They cannot answer:
- "What's the strongest counterargument to the claim I'm making in section 3?"
- "Trace the chain of evidence behind my conclusion that IV estimation is fragile."
- "Which claims in my wiki have no supporting evidence — only assertions?"
- "Which of my cited papers actually contradicts the position I'm arguing?"

The wiki knows *that* two papers relate to causal inference.
The argument graph knows *why* — one supports, one contradicts, one qualifies under a specific condition.

---

## Solution

A semantic layer over the research wiki where claims are nodes and logical relationships are typed edges.

**Claim types:**
- `empirical_finding` — observed result with stated conditions
- `theoretical_claim` — proposed mechanism or framework
- `methodological_claim` — assertion about how to study something

**Edge types:**
- `supports` — paper B's finding corroborates paper A's claim
- `contradicts` — paper C challenges paper A's conclusion
- `qualifies` — paper D shows A's result only holds under condition X
- `extends` — paper E applies A's framework to a new domain
- `replicates` / `fails_to_replicate` — for empirical work

**What this enables:**
- "What supports claim X?" → traverse all `supports` edges inbound to X
- "What's the weakest claim?" → claims with no `supports` edges and at least one `contradicts` edge
- "Build an argument path for section 2.3" → shortest path through typed edges from premise to conclusion
- "What's the strongest objection?" → highest-confidence `contradicts` edge pointing at my claim

---

## Target User

Same as the research wiki compiler — but specifically researchers who are:
- Writing a paper where they're making a novel argument (not just summarizing literature)
- Working in fields where contradictory evidence is the norm (social science, medicine, psychology)
- Preparing for a thesis defense or oral exam where they must anticipate objections

---

## MVP Scope (1 week after wiki MVP)

1. **Claim extraction** added to compile prompt: 3–7 structured claim objects per paper
   ```
   claims:
     - id: claim_001
       text: "IV estimation is biased when instrument is weak"
       type: empirical_finding
       confidence: high
       source_page: 12
   ```

2. **Relationship inference** orchestrator pass: for each pair of claims on linked entity pages,
   infer relationship type or `none`; prune via backlinks to control O(n²) cost

3. **Argument graph index**: `_argument_graph.md` — human-readable, git-versioned
   ```
   claim_001 → contradicts → claim_047 (source: paper B, confidence: medium)
   claim_001 → qualifies → claim_023 (condition: "when n < 100")
   ```

4. **Two query operations:**
   - "What supports X?" — traverse inbound `supports` edges
   - "What's the weakest claim in this set?" — structural graph analysis

**What is explicitly out of scope for MVP:**
- Visual graph UI (output is markdown and structured text)
- Belief propagation or confidence scoring (N10 — future layer)
- Structural gap mapping (N11 Anti-Library — future layer)
- Collaborative or shared argument graphs (N9 — kills moat)

---

## Competitive Landscape

**General argument mapping (not on personal research wikis):**
- **Argdown** — structured argument notation; markdown-like; no LLM integration; no wiki
- **Kialo** — collaborative argument mapping; not personal library; debate-oriented
- **IBM Debater** — argument mining from large corpora; not personal scale

**Agent memory systems:**
- **A-MEM** (NeurIPS 2025), **MAGMA**, **PRIME**, **Zettelgarden** — all use similarity-based linking; none implement typed argument edges

**The gap:** No tool combines LLM-compiled wiki + claim extraction + typed logical edges + Zotero provenance.
The combination is unoccupied. Competition score 2 because adjacent tools exist (Argdown/Kialo), not because they solve this problem.

---

## Research Contribution Angle

H19 is publishable as a systems paper:
*"Argument-Aware Personal Research Wikis: LLM-Compiled Claim Graphs from Academic PDF Annotations"*

Evaluation:
- Precision/recall on extracted relationships (manually annotated ground truth, 30-paper corpus)
- User study: are argument graph queries more useful than topical queries for specific writing tasks?
- Comparison baseline: H13 topical search without the argument layer

Grounding in Luhmann: the typed edge is the technical implementation of what Luhmann called the
Kommunikationspartner property — the system's ability to surface connections the author never consciously planned.
Without typed edges, a wiki is an information engine. With them, traversal generates implicit reasoning chains.

---

## Downstream Hypotheses Unlocked

- **N10 Calibrated Claims** — Bayesian confidence propagation through the argument graph; formal AGM grounding
- **N11 Anti-Library** — structural gap map: find the shape of your ignorance from graph topology holes
- **N12 Argument Path Navigation** — shortest path + weakest link for writing applications ("build argument for section 2.3")
- **H20 Companion Researcher Agents** — cross-KB argument graph diff between your wiki and a public corpus
- **H14 Draft Audit** — check draft claims against wiki claim_047 with typed-edge evidence chains
