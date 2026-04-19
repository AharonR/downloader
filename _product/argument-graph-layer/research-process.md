# Argument Graph Layer — Research Process

Skill: `_product/skills/pain-discovery.md`

---

## Phase A Results (completed 2026-04-12)

Full report: `_product/argument-graph-layer/pain-discovery-report.md`

| Question | Status | Answer |
|----------|--------|--------|
| Reasoning gap vs retrieval gap | **Resolved** | Reasoning gap is real and distinct. Manifests systemically (12% post-replication citation correction rate; 153–300x citation rate for non-replicable papers) rather than as personal workflow narratives. |
| PKM typed edge demand | **Resolved** | High. Obsidian typed-link request: 183 votes, 3 independent plugins. Logseq explicit `providesEvidenceFor` request. Zettelkasten.de: Helen_Shepherd explicitly lists "supporting evidence" and "contradicting evidence" link types. Workaround is inline prose — not queryable. |
| Manual workaround evidence | **Resolved** | Whiteboards, Miro, mindmaps, separate documents used pre-writing. Maintained outside reading/library workflow. Tool must eliminate this manual step. |
| Scite gap analysis | **Resolved** | Scite = corpus-level (how others cited a paper). Not personal-library-level. Does not connect to researcher's own claims/draft. H19 is the personal-argument layer Scite doesn't touch. |
| Competitor gap (Argdown, Kialo, IBM Debater) | **Resolved** | All either corpus-scale (IBM Debater — now sunset), collaborative/public (Kialo), or disconnected from personal libraries (Argdown). Gap confirmed. |
| Field specificity | **Resolved** | Sharpest in medicine (32% clinical research contradicted), psychology (replication crisis), economics (identification debates), philosophy. Lead with medicine/social science. |
| Trust in LLM-generated edges | **Open** | No community signal on whether researchers would trust automated `contradicts` edges. Requires Phase B. |
| Surprise property of argument graph | **Open** | Whether typed edges surface connections researchers didn't consciously plan cannot be determined from corpus mining. Phase B must show a manually constructed graph. |

---

## Phase A: Automated Pain Discovery

Run the pain discovery skill with the context below before any interviews or building.

---

### Context

**Pain hypothesis:**
AI research tools are information engines, not insight engines. They help researchers find and
summarize papers, but they cannot trace the logical structure between claims: what supports what,
what contradicts what, what only holds under specific conditions. Researchers building original
arguments cannot use their reading as a structured evidential base — only as a search index.

**Target user:**
Researchers writing an argumentative paper (not a survey). Fields where contradictory evidence
is the norm: medicine (RCT vs observational), economics (identification debates), philosophy of
science, social science (replication crisis context). PhD students preparing for thesis defense.

**Competitors:**
- Argdown — structured argument notation; no LLM integration; not connected to a personal library
- Kialo — collaborative argument mapping; debate-oriented; not personal research wikis
- IBM Debater — argument mining at corpus scale; not personal
- A-MEM, MAGMA, PRIME, Zettelgarden — agent memory systems; similarity-based linking only; no typed edges
- Connected Papers, Litmaps — citation graph tools; structural not logical relationships

---

### Signal Targets

Classify each collected item using these gap types:

| Gap type | What it looks like for this product |
|----------|-------------------------------------|
| `reasoning_gap` | "I can't trace why I believe what I believe", "I don't know what the counterargument is" |
| `synthesis_gap` | "I know the papers but can't see how the arguments fit together" |
| `trust_gap` | "I cited two papers that contradict each other and a reviewer caught it" |
| `workflow_gap` | "I build argument maps manually in a separate tool that doesn't connect to my library" |
| `abandonment` | "I tried argument mapping tools but maintaining them was too much work" |

Strong signal: a researcher describes discovering a contradiction between two papers they were both citing — and not finding it themselves. Or: describes building an argument map manually outside their reading workflow.

Weak signal: general agreement that "AI doesn't reason, it summarizes." (Too abstract to drive build decisions.)

**The critical distinction to make:**
Is the complaint a *retrieval gap* (can't find what I have) or a *reasoning gap* (can't evaluate the logical structure of what I have)?

Forum posts from researchers who say "I can't find the paper that supports my claim" → retrieval gap → H13 solves this.
Forum posts from researchers who say "I don't know if my chain of evidence actually holds" → reasoning gap → H19 is the right intervention.

The skill must separate these two populations. Only the reasoning gap validates H19.

**Key discriminant question:**
Do researchers describe a *logical structure* problem — knowing what supports, contradicts, or qualifies their claims — or only a *retrieval* problem?

---

### Search Tools

**Forums:**
- r/AskAcademia — search: "counterargument", "contradictory evidence", "argument structure", "thesis defense objection"
- r/PhD — search: "contradiction", "evidence chain", "strongest objection", "claims evidence"
- r/philosophy — search: "argument mapping", "claim structure", "typed edges", "formal argumentation"
- r/medicine + r/science — search: "contradictory studies", "RCT vs observational", "evidence hierarchy"
- r/economics — search: "identification debate", "contradictory findings", "causal claim"
- PubPeer — browse comments for contradiction dispute patterns (these are typed-edge relationships in the wild)
- Zotero Forums — search: "argument", "claim", "contradiction", "logic"

**GitHub:**
- Argdown issues — what users wish Argdown did; integration requests with personal libraries
- Kialo feedback — what collaborative argument mapping misses for academic solo work
- Search: "argument graph" OR "claim extraction" in academic tool repos

**Blogs and writeups:**
- Zettelkasten.de forum — search: "typed links", "relationship types", "meaningful connections" (practitioners who already think in typed edges)
- The 100% CI (Andrew Gelman's blog) — methodological contradiction discussions; surfaces the reasoning gap in the wild
- Replication crisis coverage (Retraction Watch, PubPeer) — contradiction as a discovered phenomenon
- HN threads: search "argument map", "claim graph", "typed links knowledge management"
- PKM blogs: search "typed links" OR "relationship types" site:obsidian.md OR site:substack.com

**Argument mapping community specifically:**
- argumentation.io community
- Search Twitter/X: "argument map research", "#PKM typed links", "Argdown", "Kialo academic"

---

### Expected Insights

The skill should surface:

1. **Reasoning vs retrieval split** — what percentage of pain signals are about logical structure (reasoning gap) vs finding papers (retrieval gap)? If retrieval dominates, H13 is the product; H19 is premature.
2. **Contradiction discovery moment** — when do researchers discover contradictions? During writing, at peer review, post-publication? (Earlier = more acute pain)
3. **Manual workarounds** — are researchers already maintaining argument maps manually (whiteboard, separate Notion doc, mental model)? Workarounds reveal the pain and also reveal what the tool must replace.
4. **Typed edge demand** — do researchers in Zettelkasten or PKM communities explicitly complain about similarity-based links vs meaningful typed links? (Confirms the Luhmann framing resonates with the target community)
5. **Field specificity** — is the reasoning gap concentrated in specific fields (medicine, economics, philosophy) or broad? (Determines whether to position as domain-specific or general)
6. **Trust in LLM edge classification** — is there any community signal that researchers would trust an automated `contradicts` edge, or would they demand manual review of all edges?

---

## Phase B: Human Validation (3–4 interviews)

Phase B is stimulus-only. Phase A answers whether the reasoning gap exists and how it's described.
Phase B answers two things corpus mining cannot: (1) does the argument graph produce *surprise*
(connections the researcher didn't consciously plan), and (2) do researchers trust typed edges
enough to act on them when writing.

**Prepare before interviews:**
Manually construct an argument graph for 10–15 papers in the researcher's field.
Extract 3–5 claims per paper. Classify relationships by hand. Build `_argument_graph.md`.
This is the stimulus. Do not show an automated output — the manual graph is more accurate and
the point is to test whether the *structure itself* produces value, not whether the automation works.

**Who to interview:**
Researchers writing an argumentative paper who have had a reviewer catch a contradiction they missed.
Or: researchers who maintain a manual argument map or claim list outside their reading tool.
These are people who have already felt the pain and invented a workaround — the highest-signal interviewees.

**Interview agenda (45 minutes):**

*10 min — one story:*
- "Tell me about the last time you discovered a contradiction between two papers you were both citing."
- "How did you find out? What did you do?"
- If no contradiction story: "Tell me how you figure out what the strongest objection to your argument is."

*15 min — stimulus: show the manual argument graph:*
- "Here's what an argument graph looks like for 15 papers in your area."
- "Point at an edge — does this relationship look right to you?"
- "Is there a connection here that surprised you — something you wouldn't have noticed just from reading the papers individually?"
- "What's wrong or missing? What edge type would you want that isn't here?"

*10 min — writing application:*
- "If you could ask 'what's the strongest objection to claim X', how often would you use that while writing?"
- "Would 'build an argument path from my premise to my conclusion' be useful for a specific section you're working on?"
- "Would you trust a `contradicts` edge enough to remove a citation based on it — or would you always verify manually?"

*10 min — trust calibration:*
- "If the edge was labelled `confidence: medium`, would you still act on it?"
- "What would make you trust the graph enough to use it when writing — not just as a curiosity?"

**Go signal:** Researcher has a specific contradiction story + argument graph produces at least one surprising connection + they describe a specific writing task where a `supports`/`contradicts` query would change their behavior.

**No-go signal:** Researcher only has retrieval stories (H13 is sufficient); or finds the edge taxonomy interesting but says they'd verify every edge manually before using it (trust cost exceeds reasoning gain); or the manual graph produces no surprise ("I already knew all of this").
