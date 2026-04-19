# Pain Discovery Report — Argument Graph Layer
Date: 2026-04-12

---

## Discriminant Question Answer

**Do researchers describe a logical structure problem — knowing what supports, contradicts, or qualifies their claims — or only a retrieval problem?**

**Answer: Both — but the reasoning gap is real, structurally documented, and unaddressed by any current personal-library tool. The critical finding is that it manifests systemically rather than as personal workflow narratives.**

The reasoning gap cannot be separated cleanly from the retrieval gap in forum posts — researchers conflate "I can't find the paper that supports my claim" with "I don't know if my chain of evidence holds." But five independent evidence streams confirm the reasoning gap is a distinct and severe problem:

1. **Citation of contradicted papers is near-universal and unknowing.** Non-replicable papers are cited 153–300x more than replicable ones. Only 12% of post-replication citations acknowledge the replication failure. Researchers do not know they are citing papers whose findings have been overturned — not because they can't find the contradicting paper, but because no tool connects the logical relationship between what they're citing and what has since been found to contradict it.

2. **PKM communities explicitly demand typed logical links.** The Obsidian feature request "Add support for link types" has 183 votes and spawned multiple plugins. Users state directly: *"You can't immediately point out what links two nodes together... the graphing capabilities are nothing short of unusable"* without semantic link information. Logseq users explicitly request `[[providesEvidenceFor>Climate Change]]` style links: *"not all links are the same — sometimes one note supports another, other times it refutes it."*

3. **Zettelkasten practitioners list argument-typed links as their primary unmet need.** Helen_Shepherd in the Zettelkasten.de forum explicitly enumerates: "supporting evidence," "contradicting evidence," and "argument notes with premises and conclusions" as the link types she needs. The community response — "just write it inline" — is a workaround, not a solution, because inline text is not machine-readable and cannot power "what's the strongest objection to claim X?" queries.

4. **The manual workaround exists and is documented.** Researchers use whiteboards, Miro, mindmaps, and separate documents to construct argument structure before writing. Argument mapping is explicitly described as useful for "planning a PhD thesis by laying out the main research question, subsidiary claims and supporting evidence before drafting chapters." These tools are maintained outside the reading/library workflow — the integration gap is the product.

5. **Scite's partial success confirms the demand — and exposes the remaining gap.** Scite classifies how *others* have cited a paper (supporting, contrasting, mentioning). A researcher reports it *"saves so much time and has become indispensable when writing papers."* But Scite operates at the corpus level, not the personal draft level. It tells you "this paper has been cited contrastingly 15 times" — not "your draft claim in section 3 is only supported by this paper, which has since been contradicted." The personal-argument layer is the unoccupied space.

**Implication for positioning:** H19 does not replace H13 — H13 (topical wiki) partially addresses the reasoning gap by making contradictions visible within entity pages. H19 makes the logical structure *queryable*: "what's the weakest-evidenced claim in my library?" and "what contradicts claim X?" H13 is the prerequisite; H19 is the contribution layer that makes the wiki a reasoning tool, not just a synthesis tool.

**Supporting quotes:**
1. *"Not all links are the same — sometimes one note supports another, other times it refutes it. It should be possible to intuitively add semantics to links."* — Logseq feature request, "Semantically meaningful links," community discussion
2. *"Only 12 percent of post-replication citations of non-replicable findings acknowledge the replication failure"* — UCSD replication study, cited across multiple sources 2024–2025
3. *"You can't immediately point out what links two nodes together... the graphing capabilities are nothing short of unusable"* without semantic link types — Obsidian forum, 183-vote typed link feature request

---

## Signal Summary

| Gap type | Frequency | Specificity | Strongest quote |
|----------|-----------|-------------|-----------------|
| `reasoning_gap` | High | Medium | "Not all links are the same — sometimes one note supports another, other times it refutes it" (Logseq feature request) |
| `workflow_gap` | High | High | Argument maps maintained manually on whiteboards/Miro/separate docs outside reading workflow — confirmed in multiple academic writing guides |
| `trust_gap` | High | High | 12% citation correction rate after replication failure; 153–300x citation rate for non-replicable papers (UCSD study) |
| `synthesis_gap` | Medium | Medium | "You can't immediately point out what links two nodes together" (Obsidian typed-link request, 183 votes) |
| `abandonment` | Low | Low | Argument mapping tools (Argdown, Kialo) are abandoned for academic personal use — too formal/non-integrated |
| `retrieval_gap` | Low | Low | Mixed into reasoning language but not the primary complaint when separated |

---

## Top Pain Signals (7 items)

**1. Citation of contradicted papers — the structural reasoning failure at scale**
- Gap type: `trust_gap` + `reasoning_gap`
- Sources: UCSD study (2021, widely cited 2024–2025); Sagepub citation patterns study
- Quote: *"Papers that fail replication are cited 153 times more in psychology/economics, and 300 times more in Nature/Science journals. Only 12 percent of post-replication citations acknowledge the replication failure."*
- Classification rationale: High frequency (replication crisis is the defining issue in multiple fields), High specificity — the mechanism is exactly H19: researchers lack a tool that flags "this paper has been contradicted" at the moment of citation
- [UCSD source](https://today.ucsd.edu/story/a-new-replication-crisis-research-that-is-less-likely-be-true-is-cited-more)

**2. Obsidian typed-link feature request — 183 votes, multiple plugins spawned**
- Gap type: `reasoning_gap` + `synthesis_gap`
- Source: Obsidian Forum, ["Add support for link types"](https://forum.obsidian.md/t/add-support-for-link-types/6994), 183 votes (one of the forum's highest-voted feature requests)
- Quote: *"You can't immediately point out what links two nodes together... the graphing capabilities are nothing short of unusable"* without semantic link information
- Spawned: Wikilink Types plugin (24 default relationship types, YAML sync), Graph Link Types plugin, Negative Link Relations request
- Classification rationale: High frequency (183 votes represents a large fraction of Obsidian's power user base), High specificity — users can see that notes are connected but not WHY. Three independent plugin implementations confirm the demand is real enough to build for.

**3. Logseq explicit evidence-typed link request**
- Gap type: `reasoning_gap`
- Source: Logseq forum, ["Semantically meaningful links, e.g. [[providesEvidenceFor>Climate Change]]"](https://discuss.logseq.com/t/semantically-meaningful-links-e-g-providesevidencefor-climate-change/6684)
- Quote: *"Not all links are the same — sometimes one note supports another, other times it refutes it. It should be possible to intuitively add semantics to links to raise the expressiveness of the app to a new level."*
- The inverse property concept: `providesEvidenceFor` with its inverse `corroboratedBy` — users are designing the data model themselves because the tool doesn't provide it
- Classification rationale: Medium frequency, High specificity — users don't just want colored links; they want the inverse relationship to be auto-populated. This is the machine-readable typed edge demand expressed directly.

**4. Zettelkasten practitioner explicitly enumerates argument link types**
- Gap type: `reasoning_gap`
- Source: Zettelkasten.de forum, ["Link types"](https://forum.zettelkasten.de/discussion/2023/link-types)
- Quote: Helen_Shepherd requests five link types explicitly: similarity, difference, *supporting evidence*, *contradicting evidence*, and *argument notes with premises and conclusions*
- The community workaround (ctietze): *"This was refuted 20 years later by B. Gonnarsdottir.[[link]]"* — inline text handles the human reading case but cannot be queried programmatically
- Classification rationale: Medium frequency, High specificity — the request is coming from people who already practice structured note-taking; it names exactly the H19 edge types; the workaround confirms the pain (people are managing typed edges manually as prose)

**5. Scite's success confirms demand; its gap is the H19 opportunity**
- Gap type: `workflow_gap`
- Source: Scite.ai review, Effortless Academic 2026; Clemson library adoption case
- Quote: *"saves so much time and has become indispensable when writing papers and finding related work to cite and read"* — typed citation classification is validated
- Critical gap: Scite classifies how *others* have cited a paper from the full corpus. It does NOT: (a) connect to the researcher's personal library of papers, (b) map typed relationships at the claim level within a draft, or (c) persist a personal argument structure. "Does the paper you just downloaded have support or contradiction relationships to papers already in my wiki?" — Scite cannot answer this.
- Classification rationale: High frequency (Scite has 2M users), High specificity of the gap — corpus-level typed citation vs personal-draft typed argument layer are categorically different products

**6. 32% of highly-cited clinical research contradicted in subsequent studies**
- Gap type: `trust_gap` (systemic)
- Source: Multiple clinical studies; ScienceDirect meta-analysis
- Quote: *"Among 49 highly-cited original clinical research studies, 32% were contradicted in subsequent large-scale studies."*
- Classification rationale: High frequency (documented across medicine, psychology, economics), High specificity — the reasoning gap is not edge-case; it is the norm in high-stakes fields. The tool addresses a structural failure in how researchers manage evidential relationships.

**7. IBM Debater (the proof-of-concept) was sunset — the research exists, the product doesn't**
- Gap type: `workflow_gap`
- Source: IBM Research, Project Debater API sunset (2024)
- Argdown: structured notation but no LLM integration; too formal for individual researchers
- Kialo: collaborative debate platform; not personal-library; not academic writing workflow
- Quote from IBM: "Project Debater Early Access Program was sunset after several years of availability to the academic community"
- Classification rationale: The argument mining research field (ACL ArgMining workshop, annual) confirms the NLP primitives exist. The product gap is that every tool is either corpus-scale (IBM Debater, Scite) or disconnected from personal libraries (Argdown, Kialo). No tool combines claim extraction + typed edges + personal Zotero library.

---

## Competitor Gap Map

### Scite.ai
- **What it does:** Classifies how papers in the corpus have cited a given paper: supporting, contrasting, or mentioning. 2M users. Library integration via browser plugin.
- **Top complaints:** Classification "doesn't function entirely correctly" sometimes; paywall access gaps; missing papers in database
- **Critical gap for H19:** Corpus-level, not personal-library-level. Shows "this paper has been cited contrastingly 40 times in the literature" — not "your claim in section 3 depends on this paper, which contradicts another paper you're citing in section 4." The personal-argument layer is invisible to Scite.
- **What this confirms:** The market has validated typed citation classification; the unoccupied space is personal-argument-layer typing connected to a researcher's own draft and library

### Argdown
- **What it does:** Structured argument notation language; markdown-like syntax for argument maps; VS Code plugin
- **Top complaints:** No LLM integration; requires manual claim extraction and typing; no connection to Zotero or personal library; developer audience, not researchers
- **Critical gap:** Highly expressive but entirely manual. A researcher who wanted to map their 200-paper library would spend more time writing Argdown than reading papers. And it doesn't connect to anything — no Zotero, no PKM vault, no wiki.
- **What this confirms:** The notation problem is solved; the automation + integration problem is not

### Kialo
- **What it does:** Collaborative argument mapping for debate/deliberation; tree structure; public debates
- **Critical gap:** Collaborative, public, debate-oriented. Not designed for personal academic work. A PhD student's argument map of their own claims and evidence is not a debate — it's a private reasoning scaffold. Kialo's architecture is wrong for the use case.

### Obsidian typed-link plugins (Wikilink Types, Graph Link Types)
- **What they do:** Add typed relationships to wikilinks; 24 default relationship types; YAML frontmatter sync
- **Critical gap:** Still manual. The researcher must type the relationship type for every link. No extraction, no inference, no backpropagation. And the relationship type is on the Obsidian link, not on the academic paper's claim extracted from a PDF. These tools solve the display problem, not the extraction problem.
- **What this confirms:** Community demand for typed edges is high enough that multiple plugins have been independently built. The unoccupied space is LLM-assisted claim extraction + automated relationship inference.

### H13 Research Wiki (the prerequisite)
- **What it does:** Compiles Zotero library into entity pages with backlinks; topical synthesis; annotation-aware
- **Partial credit for H19's problem:** H13's entity pages will surface contradictions implicitly — if two papers make opposing claims, both claims appear in the entity page and a careful reader can spot them. This is a "passive contradiction display" not a "queryable argument graph."
- **What H19 adds:** Active querying ("what's the strongest objection to claim X?", "which claims in my library have no supporting evidence?"), confidence scoring per claim, typed edge traversal. H13 is necessary but not sufficient.

**The confirmed unoccupied space:** LLM-extracted claims + typed logical edges (`supports`, `contradicts`, `qualifies`, `extends`, `fails_to_replicate`) + personal Zotero library + queryable argument index. Scite has the corpus-scale version; Argdown has the notation; H13 has the library integration. H19 is the first tool combining all three for a single researcher's personal library.

---

## What Corpus Mining Cannot Answer

- **Whether researchers would trust a LLM-generated `contradicts` edge enough to act on it.** The evidence for typed edge demand is strong. The evidence for trust in automated classification is absent from public forums. A researcher who sees `claim_001 contradicts claim_047 (confidence: medium)` may respond with: "I need to verify this myself" (automation aversion) or "show me the evidence" (evidence-requires-quote design problem, same as citation verifier). This is the #1 Phase B question.

- **Whether the "surprise property" holds.** Luhmann's claim — that a structured note system surprises its author by surfacing connections never consciously planned — is the core H19 value proposition. Corpus mining cannot confirm whether LLM-extracted typed edges produce genuine insight surprises vs. confirming what the researcher already knows. Phase B requires a manually constructed argument graph shown to a researcher.

- **Whether H13 already partially solves this.** The entity-page format (multiple papers contributing to one page) will surface contradictions passively. A researcher reading an entity page about "IV estimation" will see: "Smith (2019) argues X; Jones (2022) argues not-X." Is this sufficient? Or do researchers need the typed `contradicts` edge to be explicit and queryable? This is the H13-vs-H19 boundary that Phase B must draw.

- **Field specificity of the reasoning gap.** Medicine, economics, and philosophy show the clearest reasoning-gap signal. Does the same pain exist in humanities, computer science, or quantitative social science? The research process targets four specific fields — Phase B interviews should include at least two field-pairs to see if the pain is general or concentrated.

- **Whether the edge taxonomy is right.** The product brief proposes: `supports`, `contradicts`, `qualifies`, `extends`, `replicates`, `fails_to_replicate`. The Zettelkasten forum user requested "similarity, difference, supporting evidence, contradicting evidence, argument notes." The Logseq request used "providesEvidenceFor" and its inverse. Whether researchers naturally think in these categories or need a different taxonomy cannot be determined from corpus mining alone.

---

## Phase B Interview Guide

**Recommended stimulus:** Manually construct an argument graph for 10–15 papers in the researcher's specific field. Extract 3–5 claims per paper. Classify relationships by hand. Build `_argument_graph.md` with explicit typed edges. Do NOT show an automated output for Phase B — the manual graph tests whether the *structure itself* produces value. If it does, automation becomes a time-saving implementation detail, not the product premise.

**Who to interview:** Researchers in medicine, economics, social science, or philosophy who write argumentative papers (not just surveys). The highest-signal interviewees are those who have had a reviewer catch a contradiction they missed, or who maintain a manual argument structure outside their reading tool (separate Notion, spreadsheet, whiteboard).

**Interview agenda (45 minutes):**

*10 min — contradiction or reasoning story:*
- "Tell me about the last time you discovered a contradiction between two papers you were both citing. How did you find out — during writing, at peer review, or after?"
- If no contradiction story: "How do you figure out what the strongest objection to your argument is? Walk me through what you actually do."
- Target: the specific manual process that currently substitutes for typed edges

*15 min — stimulus: show the manual argument graph:*
- "Here's what an argument graph looks like for 15 papers in your field."
- "Point at an edge — does this relationship type look right?"
- "Is there a connection here that surprised you — something you wouldn't have noticed just from reading the papers?"
- "What's wrong or missing? What edge type would you want that isn't here?"
- "If you could ask 'what's the strongest objection to claim X', how often would you need that while writing?"

*10 min — trust calibration:*
- "If an algorithm extracted these edges automatically, would you trust a `contradicts` edge enough to act on it? Or would you verify every one manually?"
- "If it showed you the evidence (the quoted passage that generates the `contradicts` judgment), would that change your answer?"
- "What confidence level would make you comfortable citing from the graph without re-reading the papers?"

*10 min — workflow fit:*
- "Where would this fit in your writing process — before you start writing, while writing, or at the end when you're checking your argument?"
- "Would you want the argument graph to be queryable ('what supports claim X?') or visual ('show me all the connections around claim X')?"
- "What's the one thing that would make you actually use this vs treating it as a curiosity?"

**Sharpened questions (raised by corpus mining):**
1. *"Do you know if any papers you're citing right now have been contradicted by later work?"* — if "no," this is the exact H19 pain point in their own words
2. *"When you write a key claim in your paper, can you trace the evidential chain from your claim back to your primary sources? How long does that take?"* — probes the "weakest claim" query use case
3. *"Have you ever cited two papers that made opposite claims in the same section without realizing it? How did you find out?"* — the contradiction discovery moment

**Go signal:** Researcher has a specific contradiction discovery story (especially if discovered at peer review, not during writing) + argument graph produces at least one surprising typed connection + they describe a specific writing task where "what's the strongest objection to claim X?" would change their workflow.

**No-go signal:** Researcher says they'd verify every typed edge manually before acting on it — the trust cost eliminates the labor savings. Or: argument graph confirms only what they already knew — no surprise property, no revelation. Or: the reasoning gap signals they describe are actually retrieval gaps (H13 suffices).

---

---

## Deep Dive A — The Typed Edge Demand: Formal System vs. Inline Workaround

### The community is split — and the split is informative

The Zettelkasten.de forum discussion reveals the key tension: Helen_Shepherd explicitly requests formal typed link categories (including "supporting evidence" and "contradicting evidence"). The community response, from ctietze: *"This was refuted 20 years later by B. Gonnarsdottir.[[link]]"* — inline text handles the human reading case.

The inline-text approach is a workaround that:
- Works for a human reading one note at a time
- Fails for programmatic querying ("find all `contradicts` edges from papers in my library")
- Cannot power "what's the strongest objection to my claim?" traversal
- Cannot compute "which claims have the weakest evidential support?"

The H19 value proposition requires machine-readable typed edges precisely because the questions that matter most ("what weakens my argument?") require traversing the graph, not reading individual notes.

**The practical implication:** The inline-text consensus in the Zettelkasten community suggests that manual typed-edge maintenance has a high friction cost. This directly confirms the design approach: edges must be LLM-extracted and inferred during the compile phase (H13 orchestrator pass), not manually entered by the researcher. If researchers had to type the edge type on every link, they'd default to inline prose just as the Zettelkasten community does.

### Obsidian plugin evidence is the strongest signal

The Obsidian typed-link feature request (183 votes) spawning three independent plugin implementations is the highest-signal finding in this run. The plugins (Wikilink Types with 24 relationship types; Graph Link Types; Negative Internal Link notation) all solve the display/manual-entry problem — not the extraction problem. The gap they expose is: users know typed edges are valuable, built workarounds to enter them manually, and are still waiting for a tool that extracts and infers them automatically.

H19 is that tool, layered on top of H13's academic-PDF extraction pipeline.

---

## Deep Dive B — Field Specificity: Where Is the Reasoning Gap Sharpest?

### Medicine and psychology: the contradiction discovery moment is at peer review or post-publication

In medicine, 32% of highly-cited clinical research is contradicted in subsequent studies. The evidence hierarchy (RCT > observational > case series) is formalized precisely because the reasoning gap is dangerous — a clinical decision based on an observational study that was later overturned by an RCT can harm patients. Researchers in these fields already have formal tools (GRADE system, Cochrane systematic reviews) for evaluating evidence quality. But these are population-level tools — none of them connect to a researcher's personal library of papers.

In psychology, the replication crisis is the defining event of the past decade. Researchers are acutely aware that papers they've been citing may not replicate. The corpus-level tool (checking whether a paper has been replicated) is Scite. The personal-library tool (checking whether my claims are based on papers that have been contradicted) does not exist.

### Economics: the identification debate is a native reasoning-gap framing

In economics, the question "does this causal claim hold?" is contested at the level of identification strategy (IV, RCT, difference-in-differences, regression discontinuity). Two papers can reach opposite conclusions using the same dataset but different identification assumptions. The field has a native framing for the `contradicts` edge that doesn't require argument mapping to be taught — researchers already speak in terms of "this paper challenges the identification in that paper."

### Philosophy: argument structure IS the work

In philosophy, the typed edge is not a tool — it is the methodology. Claims, objections, responses, counter-responses are the native unit of scholarship. The demand for typed edges in this field is not latent; it is the definition of what philosophers do. The question is whether a LLM-extracted argument graph would be accurate enough for philosophers to trust.

### Computer science / quantitative fields: weaker signal

No clear reasoning-gap signal surfaced from computer science, mathematics, or engineering communities. These fields have formal proof systems (for math) or empirical benchmarks (for CS) that partially substitute for argument structure. The H19 target users are in empirical, argumentative fields — not formal/technical ones.

**Practical positioning implication:** Lead with medicine and social science. The contradiction discovery moment (32% of clinical research contradicted; replication crisis well-known) provides a concrete, defensible framing. "Know which of your citations have been contradicted, before your reviewer does" is the positioning statement for the medicine/social science user.

---

## Raw Sources

**Citation patterns and replication crisis:**
- [Research That Is Less Likely to Be True Is Cited More — UCSD](https://today.ucsd.edu/story/a-new-replication-crisis-research-that-is-less-likely-be-true-is-cited-more)
- [Half of Social-Science Studies Fail Replication — Nature (2026)](https://www.nature.com/articles/d41586-026-00955-5)
- [The Peer-Review Problem and Explosion of Retractions — Independent Institute (2026)](https://www.independent.org/article/2026/01/19/retraction-crisis/)
- [Why Are There So Many Contradicted Findings in Highly-Cited Clinical Research — ScienceDirect](https://www.sciencedirect.com/science/article/abs/pii/S1551714422001082)
- [Towards a Characterization of Apparent Contradictions in the Biomedical Literature — PMC](https://pmc.ncbi.nlm.nih.gov/articles/PMC7001095/)

**PKM community typed link demand:**
- [Add Support for Link Types — Obsidian Forum (183 votes)](https://forum.obsidian.md/t/add-support-for-link-types/6994)
- [Wikilink Types plugin — GitHub (penfieldlabs)](https://github.com/penfieldlabs/obsidian-wikilink-types)
- [Negative Internal Link Notation — Obsidian Forum](https://forum.obsidian.md/t/negative-internal-link-and-embeds-relation-type-notation/52394)
- [Semantically Meaningful Links — Logseq Forum](https://discuss.logseq.com/t/semantically-meaningful-links-e-g-providesevidencefor-climate-change/6684)
- [Concept Map / Link Types — Logseq Feature Request](https://discuss.logseq.com/t/concept-map-graph-feature-aka-link-and-relationship-types/4068)
- [Link Types — Zettelkasten.de Forum](https://forum.zettelkasten.de/discussion/2023/link-types)
- [Having Two Different Types of Links — Zettelkasten.de](https://forum.zettelkasten.de/discussion/3317/having-two-different-types-of-links)

**Competitor analysis:**
- [Scite AI Review 2026 — Effortless Academic](https://effortlessacademic.com/scite-ai-review-2026-literature-review-tool-for-researchers/)
- [Scite Smart Citations — MIT Press](https://direct.mit.edu/qss/article/2/3/882/102990/scite-A-smart-citation-index-that-displays-the)
- [IBM Debater Early Access Program (sunset 2024)](https://early-access-program.debater.res.ibm.com/academic_use)
- [Argdown — GitHub](https://github.com/christianvoigt/argdown)
- [ArgMining Workshop 2025 — ACL](https://argmining-org.github.io/2025/)

**Argument mapping research:**
- [Argument Mapping in Academic Writing — MW Editing](https://www.mwediting.com/argument-mapping-in-academic-writing/)
- [Frontiers: Review of Argument Visualization Research for Writing, 2025](https://www.frontiersin.org/journals/education/articles/10.3389/feduc.2025.1672105/full)
- [Designing Logic Pattern Templates for Counter-Argument — EMNLP 2024](https://aclanthology.org/2024.findings-emnlp.661.pdf)

**LLM wiki typed edge implementations:**
- [LLM Wiki v2 — extending Karpathy's pattern with typed edges (rohitg00)](https://gist.github.com/rohitg00/2067ab416f7bbe447c1977edaaa681e2)
- [Claim Knowledge Graph Construction and GraphRAG-Based QA — MDPI 2025](https://www.mdpi.com/2075-5309/16/4/845)
