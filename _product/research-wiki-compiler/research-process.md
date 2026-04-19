# Research Wiki Compiler — Research Process

Skill: `_product/skills/pain-discovery.md`

---

## Phase A Results (completed 2026-04-12)

Full report: `_product/research-wiki-compiler/pain-discovery-report.md`

| Question | Status | Answer |
|----------|--------|--------|
| Retrieval vs structure discriminant | **Resolved** | Structure problem. "Most researchers already know how to find papers. The hard part is making sense of them." Five independent evidence lines; retrieval is not the primary complaint. |
| Annotation-awareness (H17) signal | **Partially resolved** | Latent signal — Better Notes popularity + ZotAI annotation-context confirm direction; but explicit "compile my annotations first" request does not appear. Requires Phase B side-by-side probe. |
| Abandonment patterns | **Resolved** | Roam, second-brain systems, Obsidian manual maintenance: all collapse at 50–200 papers. Specific moment: maintenance cost exceeds synthesis return. |
| Trust in AI compilation | **Partially resolved** | WikiCrow beats Wikipedia accuracy in biology; but personal-library use case untested. Trust calibration requires Phase B stimulus. |
| Competitor gap map | **Resolved** | ZotAI/PaperQA = Q&A, not compilation. NotebookLM = silo, 50-source cap, amnesiac. Obsidian = manual maintenance collapses. Generic wiki compilers = no academic PDF/Zotero support. Gap confirmed. |
| Scale at which problem becomes acute | **Resolved** | 50–200 papers is the documented breakdown window. llm-wiki-compiler community confirms "a few dozen" to ~200 is optimal range before quality degrades. |
| Compile quality on academic PDFs | **Resolved** | Empirical fields: acceptable (MinerU 86.2 OmniDocBench; WikiCrow beats Wikipedia). Math/physics: equation parsing fails (CDM 66.9). Scale ceiling at ~200 papers without consolidation engine. |
| H17 as differentiator vs detail | **Open** | Market moving toward annotation-context (ZotAI, DeepTutor). No explicit "annotation-first compilation" request found. Phase B side-by-side required to determine whether researchers recognize the difference. |

---

## Phase A: Automated Pain Discovery

Run the pain discovery skill with the context below before any interviews or building.

---

### Context

**Pain hypothesis:**
Researchers accumulate papers in Zotero but cannot synthesize across them.
Their reading is an archive, not a knowledge base. Every new paper is written by reconstructing
knowledge from scratch rather than building on a persistent conceptual map.
Existing tools (RAG, Q&A) answer questions about individual papers but don't build cumulative structure.

**Target user:**
PhD students 2–4 years in, with 50–200 papers in Zotero, active annotations.
Researchers in a literature-heavy phase: comprehensive exam prep, dissertation writing, grant preparation.

**Competitors:**
- ZotAI — Q&A over Zotero PDFs + annotations; RAG-based; no entity pages or backlinks
- PaperQA (Future-House) — high-accuracy RAG for scientific PDFs; no persistent knowledge structure
- NotebookLM — Q&A over uploads; 50-source cap; no Zotero integration; no persistent wiki
- Atlas — auto-builds connections from uploads; silo; no Zotero integration
- Obsidian graph view — manual links; requires deliberate effort; AI plugins do similarity not compilation
- llm-wiki-compiler, llm-knowledge-bases — generic wiki compilers; no academic PDF or Zotero support

---

### Signal Targets

Classify each collected item using these gap types:

| Gap type | What it looks like for this product |
|----------|-------------------------------------|
| `retrieval_gap` | "I know I read something relevant but can't find it" |
| `synthesis_gap` | "I have 200 papers and can't see the patterns across them" |
| `abandonment` | "I stopped using Zotero / Obsidian because maintenance takes too long" |
| `workflow_gap` | "I reconstruct literature review from scratch for every new paper" |
| `trust_gap` | "AI summaries of papers are plausible but wrong — I'd have to verify everything" |

Strong signal: a researcher describes the moment of starting a new paper and having to re-excavate their own library from scratch. Or: describes maintaining a manual synthesis system that eventually collapsed.

Weak signal: general agreement that "too many papers, too little time" without a specific workflow breakdown.

**Key discriminant question:**
Is the synthesis problem primarily a *retrieval* problem (can't find what they have) or a *structure* problem (can't see relationships across what they have)?

If retrieval → better search solves it; wiki compilation may be overkill.
If structure → the compile phase is the right intervention; entity pages and backlinks are the product.

This also determines whether annotation-awareness (H17) is a differentiator or a detail.

---

### Search Tools

**Forums:**
- r/PhD — search: "literature review", "Zotero", "synthesize papers", "too many papers", "Obsidian research"
- r/GradSchool — search: "reading list", "comprehensive exam", "knowledge base", "second brain research"
- r/AskAcademia — search: "organize research", "keep track of papers", "literature review workflow"
- r/Zotero — entire subreddit; look for workflow questions and feature requests
- r/ObsidianMD — search: "research papers", "academic", "Zotero", "literature review"
- Zotero Forums — feature requests section; search "synthesis", "connections", "knowledge base"
- Obsidian Forum — search: "academic", "research", "Zotero", "literature"

**GitHub:**
- zotero/zotero issues — feature requests for synthesis, connections, AI compilation
- ussumant/llm-wiki-compiler issues — what users wish the generic compiler did
- rvk7895/llm-knowledge-bases issues — feature gaps and usage questions
- Future-House/paper-qa issues — what PaperQA doesn't do that users want

**Blogs and writeups:**
- Aaron Tay's Substack — "The Agentic Researcher"; PKM + Zotero workflow posts
- Obsidian Roundup newsletter — academic use case threads
- LibrarianShipwreck — academic tools critique
- Personal academic blogs: search "Zotero workflow" or "literature review workflow" site:substack.com
- HN threads: search "Zotero", "research wiki", "personal knowledge management academic"

---

### Expected Insights

The skill should surface:

1. **Retrieval vs structure split** — how often do researchers describe a retrieval failure vs a structural synthesis failure? (Answers the discriminant question directly)
2. **Annotation-awareness signal** — do researchers mention their own highlights/annotations as something they wish were more connected or accessible? (Validates H17 as a differentiator vs a nice-to-have)
3. **Abandonment patterns** — which tools did researchers try and abandon, and why? (PKM abandonment is well-documented; look for the specific moment the system collapsed)
4. **Trust in AI compilation** — is there resistance to AI-generated summaries replacing human reading notes? (Vault contamination concern; determines onboarding approach)
5. **Competitor gap map** — what do ZotAI and PaperQA users wish those tools did? (Direct evidence of the unmet compile-phase need)
6. **Scale at which the problem becomes acute** — do researchers mention a specific library size where synthesis broke down? (Calibrates the target user: 50 papers vs 200 papers)

---

## Phase B: Human Validation (3–4 interviews)

Phase B is stimulus-only. Phase A answers whether the synthesis gap exists. Phase B answers whether the compiled wiki output crosses the trust threshold and fits the workflow.

**Prepare before interviews:**
Run the compile pipeline on 10 papers from a willing researcher's Zotero library (or your own).
Use Zotero MCP to extract fulltext + annotations. Run compile prompt. Produce entity pages and backlinks.
Inspect the output yourself first — would you use these entity pages? Only then show a researcher.

**Who to interview:**
Researchers who have tried and abandoned a PKM system (Obsidian, Roam, Notion for research notes).
These are the people who already know they have the problem and have failed to solve it manually.

**Interview agenda (45 minutes):**

*10 min — one story:*
- "Tell me about the last time you set up a system to organize your research reading. What happened to it?"
- Let them describe the collapse. The abandonment story reveals exactly where the maintenance cost exceeded the return.

*15 min — stimulus: show compiled wiki on their papers:*
- "Here's what the compiler produced from your last 10 papers. Walk me through what you're seeing."
- "Is this entity page accurate? Does it reflect how you understand this concept?"
- "Do these backlinks connect things you think of as related — or are they noise?"
- "Is there anything here you'd forgotten you knew?"

*10 min — annotation probe (H17):*
- Show a side-by-side: entity page compiled from full text only vs entity page compiled with their annotations as primary input
- "Which of these better reflects your understanding of this paper?"
- "Is there anything in the annotation-based version that surprised you?"

*10 min — trust and workflow fit:*
- "If you ran this every time you added a new paper, what would change about how you work?"
- "Would you edit the wiki, or treat it as read-only and just query it?"
- "What would make you trust this enough to cite from it when writing?"

**Go signal:** Researcher finds the entity pages accurate, the backlinks non-obvious, and describes a specific writing task the wiki would change. Annotation-based version produces surprise.

**No-go signal:** Researcher treats the wiki as a fancy abstract ("I still need to read the paper"); or says they'd have to verify every entity page before trusting it (trust cost exceeds retrieval gain).
