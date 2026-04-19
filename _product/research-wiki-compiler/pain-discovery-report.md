# Pain Discovery Report — Research Wiki Compiler
Date: 2026-04-12

---

## Discriminant Question Answer

**Is the synthesis problem primarily a retrieval problem (can't find what they have) or a structure problem (can't see relationships across what they have)?**

**Answer: Structure problem. Retrieval is largely solved. The gap is cumulative knowledge architecture.**

The evidence is unambiguous across five independent lines:

1. **Direct user statement (literature review meta-signal):** "Most researchers already know how to find papers. The hard part is making sense of them." — Atlas workspace review, 2025. This is not a niche complaint; it is the opening framing of multiple tool-comparison guides written for practicing researchers.

2. **NotebookLM's most-cited limitation:** *"There is no visual representation of how your sources connect, how concepts relate, or how ideas cluster. Connections remain invisible unless you already know to ask about them."* — Atlas Workspace, NotebookLM Limitations, 2025. Users are not complaining about failing to find papers. They are complaining about the absence of a persistent map.

3. **Zotero PKM workflow breakdown:** A Zotero forum user (2024) describes their working solution: *"I capture and file everything in Zotero, then export the notes to Obsidian, where I link them by keywords, authors, concepts"* — then immediately reports this workaround *"is now taking up a lot of my time."* The researcher can find everything. They cannot maintain the structure.

4. **Roam Research abandonment (the clearest collapse story):** The Fall of Roam (Every.to, 2024) documents that Roam's search worked. The failure was structural: *"I am not really going back through all of these notes as often as I thought I would."* Notes lacked "contextual presentation" — they existed but didn't cohere. The PKM general pattern confirms: *"people spend weeks crafting the 'perfect' system... a month later, they abandon it all."*

5. **llm-wiki-compiler's design rationale (self-described):** *"Instead of re-reading hundreds of raw files every session, the LLM compiles them into topic-based articles once, then queries the synthesized wiki, so knowledge compounds instead of fragmenting."* This is someone who built the workaround and describes precisely why: structure, not search, is what's missing.

**Implication for the product:** Better search does not solve this. The compile phase (entity pages, backlinks, cumulative structure) is the correct intervention. The question is not whether researchers can find a paper — they can. The question is whether they can see what their library as a whole says about a concept, without manually reconstructing that view each time they start writing.

**Supporting quotes:**
1. *"There is no visual representation of how your sources connect, how concepts relate, or how ideas cluster"* — Atlas Workspace, NotebookLM Limitations, 2025
2. *"I capture and file everything in Zotero, then export to Obsidian, where I link by keywords, authors, concepts... is now taking up a lot of my time"* — Zotero Forums, PKM thread, 2024
3. *"Instead of re-reading hundreds of raw files every session, the LLM compiles them into topic-based articles once... knowledge compounds instead of fragmenting"* — llm-wiki-compiler README (ussumant fork), 2025

---

## Signal Summary

| Gap type | Frequency | Specificity | Strongest quote |
|----------|-----------|-------------|-----------------|
| `synthesis_gap` | High | High | "There is no visual representation of how your sources connect, how concepts relate, or how ideas cluster" (NotebookLM Limitations, Atlas Workspace, 2025) |
| `abandonment` | High | High | "Spend weeks crafting the 'perfect' system... a month later, they abandon it all" (PKM survey, multiple sources, 2024–2025) |
| `workflow_gap` | High | Medium | "I capture and file everything in Zotero, then export to Obsidian... is now taking up a lot of my time" (Zotero Forums, 2024) |
| `trust_gap` | Medium | Medium | "Bad wiki article becomes a prior that poisons future generations" (llm-wiki-compiler scale discussion, 2025) |
| `retrieval_gap` | Low | Low | Retrieval works; failing to find a paper is not the primary complaint in any channel surveyed |

---

## Top Pain Signals (8 items)

**1. "Amnesiac by design" — NotebookLM's structural amnesia as the most-cited user complaint**
- Gap type: `synthesis_gap`
- Source: Atlas Workspace, [NotebookLM Limitations: 8 Gaps Google Won't Tell You](https://www.atlasworkspace.ai/blog/notebooklm-limitations), 2025
- Quote: *"NotebookLM forces you to remember which notebook contains which insight, then manually recreate context every time you start fresh — it's amnesiac by design."*
- Classification rationale: High frequency (cited in every NotebookLM review), High specificity — describes the exact failure mode: no persistent cross-session knowledge structure. The 50-source cap and notebook silo architecture compound this.

**2. The forced three-tool workflow — Zotero → Obsidian → writing**
- Gap type: `workflow_gap` + `synthesis_gap`
- Source: Zotero Forums, [Bringing everything together in Zotero (PKM)](https://forums.zotero.org/discussion/124825/bringing-everything-together-in-zotero-personal-knowledge-management-pkm), 2024
- Quote: *"I capture and file everything in Zotero, then export the notes to Obsidian, where I link them by keywords, authors, concepts... what if these links between notes could be made directly within Zotero? That way, I wouldn't have to leave Zotero."*
- Also: *"this is now taking up a lot of my time."*
- Classification rationale: High frequency (the three-tool stack is ubiquitous in academic workflow posts), High specificity — names the exact intervention point (the cross-note linking that Zotero lacks)

**3. Roam Research abandonment — the clearest PKM collapse story**
- Gap type: `abandonment`
- Source: Every.to, [The Fall of Roam](https://every.to/superorganizers/the-fall-of-roam), 2024
- Quote: *"I am not really going back through all of these notes as often as I thought I would."* Search results appeared as *"a gigantic paragraph of text, with zero context attached."* The system fragmented because *"Roam failed its fundamental promise."*
- Classification rationale: High specificity — names the specific moment (trying to retrieve a note and finding no context), the specific failure (no automated organization, no synthesis intelligence), and the outcome (migration away)

**4. The PKM productivity illusion — maintenance cost exceeds return**
- Gap type: `abandonment`
- Source: Sudo Science, [Why I'm Giving Up on a Second Brain](https://sudoscience.blog/2025/11/08/why-im-giving-up-on-a-second-brain/), Nov 2025
- Quote: *"Trying to link my notes together just because I hope that connections are going to make me feel smarter."* The author describes "hoard a lot of articles that I never return to" or "spend excessive time on highlights before abandoning the notes entirely."
- Classification rationale: High frequency (this pattern is documented across multiple PKM abandonment posts in 2024-2025), High specificity — names the behavioral trap: manual link-building consumes energy without producing synthesis

**5. NotebookLM's notebook-silo architecture blocks cross-domain synthesis**
- Gap type: `synthesis_gap`
- Source: Atlas Workspace, NotebookLM Limitations, 2025
- Quote: *"Each notebook in NotebookLM is an isolated silo. Your psychology research notebook cannot reference your neuroscience notebook."*
- Also: *"50-source cap becomes a constraint for literature reviews with 100+ papers, multi-year thesis research."*
- Classification rationale: High frequency (every NotebookLM academic review mentions the 50-source cap as a hard blocker), High specificity — the silo architecture is a design decision, not a bug, and users are explicitly choosing alternatives because of it

**6. Zotero Related Items feature failure — users want structure, not just citation lists**
- Gap type: `synthesis_gap`
- Source: Zotero Forums, ["Related Items is almost useful"](https://forums.zotero.org/discussion/120613/related-items-is-almost-useful), 2024
- Quote: *"Relations have no directionality, so there's no way to tell, without opening the paper and digging through the references, which one references which."* Users want to *"look at any paper in my collection and see how it got there"* and understand "the reference chain that led there."
- Classification rationale: Medium frequency, High specificity — requests typed, directional relations that reveal how ideas connect, not just that they're related. This is the structural knowledge gap made explicit.

**7. llm-wiki-compiler scale ceiling — the compile phase works but breaks at >200 papers**
- Gap type: `trust_gap`
- Source: llm-wiki-compiler community discussion, 2025
- Quote: *"Once you get past ~200 articles with multiple agents writing to the same knowledge base, persistent errors compound, and a bad wiki article becomes a prior that poisons future generations, requiring a consolidation engine that scores, merges, and prunes — not just appends."*
- Also from README: *"best for small, high-signal corpora — a few dozen sources."*
- Classification rationale: Medium frequency, High specificity — directly constrains the target user window (50–200 papers is the sweet spot; the product needs quality gating before widening scope)

**8. Literature review as collective bottleneck — 67 weeks from registration to publication**
- Gap type: `workflow_gap`
- Source: ResearchRabbit literature review tools review, 2025
- Quote: *"A typical systematic review takes 67 weeks from registration to publication, with screening and data extraction consuming the largest share of that time."*
- Classification rationale: High frequency (cited across multiple review tool comparisons), Medium specificity — doesn't describe a specific moment of failure, but establishes the scale of the problem the product addresses

---

## Competitor Gap Map

### ZotAI
- **Top complaints:** Requires API key configuration; confusion about offline capability vs. web API dependency; no mention of knowledge structure output
- **What it does:** Q&A over Zotero PDFs + annotations; augments prompts with notes/highlights; session-based answers
- **Critical gap:** Session-based. Each query answers a question; nothing persists. ZotAI processes annotations as context for RAG but does not compile them into entity pages or backlinks. The researcher gets an answer; they don't get a growing wiki.
- **Unmet pain:** Users who want cumulative synthesis, not just better Q&A

### PaperQA (Future-House)
- **Top complaints:** Complex setup; no UI; no persistent knowledge structure; focused on question-answering, not knowledge-building
- **What it does:** High-accuracy RAG on scientific PDFs; generates cited answers; agentic search across document sets
- **Critical gap:** PaperQA2 achieves "superhuman performance" at answering questions about papers but explicitly does not build persistent entity pages or cross-paper structure. WikiCrow (built on PaperQA2) builds wiki pages for biology genes from 1 million papers — but this is a single-entity wiki generator at population scale, not a personal research library compiler.
- **Unmet pain:** Researchers who want their own library's knowledge compounded, not just queried

### NotebookLM
- **Top complaints (from multiple review sources):** 50-source cap; no API; notebook-silo architecture prevents cross-notebook synthesis; no persistent export; "amnesiac by design"; no citation formatting
- **What it does:** Q&A over uploaded sources; audio overview generation; sharing within notebooks
- **Critical gap:** Architectural. The silo/session design means knowledge never persists. Every session begins with re-uploading. *"You can copy-paste individual responses, but citations don't transfer as links, the formatting breaks, and if you've generated multiple threads exploring different angles, there's no way to package them into a portable document."*
- **Unmet pain:** PhD students with 100+ papers in Zotero; multi-year thesis work; any workflow requiring persistent, cross-session knowledge

### Obsidian (with Zotero plugin)
- **Top complaints:** Setup time; plugin ecosystem becomes stale as tools update; manual link-building is unsustainable at scale; AI plugins do similarity search, not compilation
- **What it does:** Manual linked knowledge base; Zotero integration imports literature notes; graph view shows manual connections
- **Critical gap:** The structure is manually maintained. At 50–100 papers, the maintenance cost exceeds the synthesis benefit. The abandonment pattern is specific and well-documented: users invest weeks in setup, use the system for 3–6 months, and stop because every new paper requires manual integration into an existing structure.
- **Unmet pain:** The compile phase itself — automated entity creation and backlink generation that removes the maintenance burden

### llm-wiki-compiler / llm-knowledge-bases (generic)
- **Top complaints:** Anthropic-only; scale ceiling (>200 articles causes compounding errors); markdown-only input (no PDF or Zotero); no academic PDF support; no citation metadata preservation
- **What it does:** Compiles markdown files into topic-based wiki using Karpathy pattern
- **Critical gap:** No academic PDF support; no Zotero integration; no annotation awareness; treats all sources as equal-weight text (researcher annotations carry no special weight)
- **Unmet pain:** Academic researchers who want the same compilation pattern applied to their Zotero library

**The confirmed unoccupied space:** Karpathy wiki compilation pattern + academic PDFs + Zotero annotation awareness. The generic wiki compilers work for markdown notes. PaperQA works for Q&A on papers. ZotAI works for session-level queries. None of these tools compile a researcher's library into a persistent, growing, annotation-aware wiki with entity pages and backlinks.

---

## What Corpus Mining Cannot Answer

- **Whether researchers will trust entity pages enough to cite from them when writing.** The abandonment data proves researchers know they have a synthesis problem. It doesn't prove they'll trust an AI-compiled entity page as a source of truth for their own claims. The WikiCrow result ("more accurate than Wikipedia" for biology) is promising but domain-specific and not tested on the personal-library use case.

- **Whether annotation-awareness (H17) is a noticeable differentiator or a technical detail.** Community signals show annotations being used as Q&A context (ZotAI, DeepTutor). No signal shows researchers explicitly requesting "compile my annotations first, then compile the full text." Researchers haven't articulated this preference because no tool has offered the contrast. Phase B must show the side-by-side.

- **The scale at which the wiki becomes more useful than re-reading.** The 30-paper MVP (product brief) is a reasonable starting point. But the transition from "interesting" to "necessary" (when the wiki becomes faster than re-reading the library) is unknown. Does it happen at 30 papers? 100? 200?

- **Whether the researcher edits the wiki, or treats it as read-only and queries it.** This determines the UX architecture significantly: an editable wiki requires merge/conflict resolution; a read-only wiki is simpler but may fail the "trust" bar. Corpus mining cannot answer this.

- **How researchers handle contradictions between papers.** Entity pages will surface disagreements across sources. Whether researchers want the wiki to show contradictions explicitly (as flagged conflicts) or as a synthesized view (one perspective per entity, with caveats) requires a live reaction to a prototype.

- **Whether a Zotero library with 50–200 papers is representative of the target user's actual library state.** The research process assumes 50–200 papers. But corpus mining shows active researchers often have 500+ in Zotero and only actively read a subset. The compile phase may need to target a "working library" subset, not the full archive.

---

## Phase B Interview Guide

**Recommended stimulus:** Run the compile pipeline on a willing researcher's 15–20 most-recently-added Zotero papers (their "current working library"). Produce entity pages and backlinks. Then produce a second version of two entity pages: one compiled from full paper text only, one compiled with their annotations as primary input. Bring both versions to the interview.

**Who to interview:** PhD students 2–4 years in who have tried and abandoned an Obsidian/Roam/Notion system for research notes. These researchers have already experienced the maintenance collapse; they know exactly where the manual system failed. Exclude researchers who have never tried a PKM system — they haven't felt the problem acutely enough.

**Interview agenda (45 minutes):**

*10 min — abandonment story:*
- "Tell me about the last time you set up a system to organize your research reading. What happened to it?"
- Target: the specific moment the system stopped being worth maintaining

*15 min — stimulus: show compiled wiki on their papers:*
- "Here's what the compiler produced from your last 15 papers. Walk me through what you're seeing."
- "Is this entity page accurate? Does it reflect how you understand this concept?"
- "Do these backlinks connect things you think of as related — or are they noise?"
- "Is there anything here you'd forgotten you knew?"

*10 min — annotation probe (H17 discriminant):*
- Show side-by-side: entity page from full text only vs entity page with annotations as primary input
- "Which of these better reflects your understanding of this paper?"
- "Is there anything in the annotation-based version that surprised you?"

*10 min — trust and workflow fit:*
- "Would you trust this entity page enough to cite from it when writing — or would you go back to the paper?"
- "If you ran this every time you added a new paper, what would change about how you work?"
- "What would make you edit this page vs. treat it as read-only?"

**Sharpened questions (raised by corpus mining):**
1. *"When you write, do you re-read papers or do you rely on notes you took while reading?"* — surfaces whether the researcher already synthesizes during reading (annotations = primary synthesis) or does synthesis when writing
2. *"What happened to the last PKM system you tried? What was the moment it stopped being worth it?"* — probes the maintenance ceiling; reveals the exact maintenance cost the compiler needs to eliminate
3. *"Does this backlink between [X] and [Y] reflect a connection you knew about — or is it new?"* — the "surprise property" test; if the wiki surfaces connections the researcher didn't know they had, that's the go signal
4. *"Would you trust this enough to say in a paper 'according to three studies in my library...' without re-reading all three?"* — tests the citation trust threshold

**Go signal:** Researcher identifies at least one entity backlink they didn't manually create that they find accurate and non-obvious. Annotation-based entity page produces visible recognition ("yes, that's what I meant when I highlighted this"). Researcher describes a specific writing task the wiki would change.

**No-go signal:** Researcher says the entity pages are "basically an abstract" — no synthesis beyond what a single paper says. Or: they'd need to verify every entity page before trusting it (trust cost exceeds the gain from not re-reading). Or: the annotation-based version produces no visible difference in their reaction.

---

---

## Deep Dive A — H17 Validation: Does annotation-awareness appear as a differentiator in community signal?

### Short answer: The signal is latent, not explicit — but the market is moving toward it

**What community signal exists (positive):**

The Better Notes plugin for Zotero (windingwind/zotero-better-notes, ~4,000 GitHub stars as of 2025) is among the most popular Zotero plugins. Its core value proposition is structured annotation export and synthesis: *"Better Notes adds structured note-taking and synthesis workflows directly inside Zotero, with templates for turning annotations into reusable summaries and organized notes."* The plugin's popularity (it appears in every "best Zotero plugins" list) confirms that researchers actively want their highlights and annotations converted into structured knowledge — not just extracted as raw text.

ZotAI (2025) explicitly markets annotation-awareness as a differentiator: *"ZotAI augments prompts with your notes and PDF annotations for superior accuracy."* DeepTutor similarly: *"shared view of citations, annotations, and AI summaries."* Both products treat annotations as privileged context for Q&A. The market has already validated that annotations outperform raw PDF text for research Q&A. The question is whether this validation extends to compilation (entity pages) vs. Q&A.

**What community signal does NOT show:**

No forum discussion, feature request, or product review explicitly asks for *annotations as the primary compile input for entity pages*. The specific request — "compile my highlights first, then fill in from the full text, and show me which claims in the entity page came from my own annotations vs. the paper itself" — does not appear in any channel surveyed.

**Why the absence of explicit signal is not evidence against H17:**

No tool currently offers this contrast. Researchers cannot request a feature they've never seen framed. The relevant signal is not "users asked for annotation-based compilation" but rather:
1. Annotations are actively used and exported (Better Notes popularity)
2. Tools that use annotations as context outperform those that don't (ZotAI, DeepTutor)
3. The Roam abandonment pattern shows that the failure of manual PKM is not note-taking volume but synthesis quality — and researchers' own annotations are their highest-quality synthesis artifacts

**The H17 test requires Phase B.** The right protocol is a side-by-side: entity page from full-text only vs. entity page with annotations as primary input. If the annotation-based version produces visible recognition ("that's what I actually care about in this paper") rather than neutral acceptance ("it's the same"), H17 is validated as a differentiator. If researchers can't distinguish the two, it's a technical detail.

**Preliminary assessment:** H17 is a differentiator for researchers who actively annotate during reading (the target user as described). It is a non-feature for researchers who don't annotate. The target user (PhD student 2–4 years in, active Zotero library, annotation habit) is selected specifically for annotation usage. H17 should be validated in Phase B before committing to annotation-aware compile as an MVP differentiator vs. a Phase 2 addition.

---

## Deep Dive B — Compile Quality: Can academic PDFs be compiled into entity pages a researcher would trust?

### Short answer: Yes for empirical fields at MVP scale (≤30 papers). No for mathematical fields without equation-aware parsing. Scale degrades quality past ~200 papers.

**PDF parsing layer (Docling vs. MinerU on OmniDocBench, CVPR 2025):**

OmniDocBench is the most comprehensive academic PDF parsing benchmark as of 2025 (CVPR 2025; 1,355 PDF pages across 9 document types including academic papers, textbooks, and handwritten notes).

| Metric | MinerU | Docling |
|--------|--------|---------|
| Layout detection mAP | 97.5% | 93.1% |
| Table parsing (TEDS) | 79.4 | "loses column alignment on complex multi-row headers" |
| Formula/equation (CDM) | 66.9 | Equations extracted as text; structure not preserved |
| Reading order | Strong across column layouts | Strong for simple documents |
| Processing speed | 14.7s / 12-page PDF | 8.2s (2x faster) |

**The critical finding:** Formula parsing is the weakest link at CDM 66.9 (MinerU) and lower for Docling (which does not preserve LaTeX structure for equations). For a research wiki targeting fields with mathematical content (physics, quantitative social science, statistics, computational biology), the compile phase will fail on exactly the content that matters most — equations and derivations. For empirical fields (qualitative social science, biology, medicine, humanities), formula parsing is rarely needed and table/text accuracy is sufficient.

**LLM compilation layer (WikiCrow / PaperQA2):**

WikiCrow (Future-House, 2024-2025), built on PaperQA2, generates entity pages for biology genes from 1 million papers. Their evaluation: entity pages are *"more accurate on average than actual articles on Wikipedia"* as judged by blinded PhD and postdoc-level biology researchers. This is the closest published evidence for LLM compilation quality on academic content.

Disclosed limitations: hallucination rate not explicitly stated; domain limited to biology gene summaries; scale tested is population-level (many papers per entity) not personal-library-level (1-5 papers per entity). The personal library use case is harder (less evidence per entity, more contradictions) but also more forgiving (researcher already knows the field, can spot errors).

**Scale quality ceiling:**

The llm-wiki-compiler community has identified a concrete ceiling: *"once you get past ~200 articles with multiple agents writing to the same knowledge base, persistent errors compound, and a bad wiki article becomes a prior that poisons future generations."* The atomic-memory fork is explicitly scoped to *"a few dozen sources."*

This directly constrains the MVP design. At 30 papers (product brief MVP), quality is controllable. At 200 papers, a consolidation/lint engine is architecturally necessary (already in the product brief as "lint — periodic"). At 500+ papers (a researcher's full Zotero archive), automated quality management becomes the hardest engineering problem.

**Practical implication for the product:**

1. **Empirical research fields:** Compile quality is likely sufficient for a trust threshold at MVP scale (30 papers). Entity pages from PaperQA2-level RAG on clean text are "more accurate than Wikipedia" — a researcher familiar with the domain will find errors but also find genuine synthesis they couldn't reconstruct manually.

2. **Mathematical/theoretical fields:** Equation rendering is the failure point. CDM 66.9 means ~1 in 3 formulas is misrendered. For a physicist or statistician, an entity page that misrepresents a formula is not usable. The product needs to detect formula-heavy papers and either (a) exclude them from compile, (b) use Mathpix for equation parsing (higher accuracy), or (c) flag them as "compile quality uncertain."

3. **The quality gate:** The product brief's lint phase is load-bearing for quality at scale. The MVP should include a confidence score per entity page (based on number of supporting papers and cross-paper consistency) and display it to the user. A researcher who sees "this entity page is supported by 3 papers with consistent claims" treats it differently from one with no confidence signal.

4. **The annotation advantage for quality:** Researcher annotations are by definition correct about what the researcher understood — even if the full paper is misrendered by the PDF parser, the researcher's own highlights are accurate. This is an underappreciated quality argument for H17: annotation-based compilation partially compensates for PDF parsing errors in formula-heavy content.

---

## Raw Sources

**Zotero community:**
- [Possible future of Zotero as a personal knowledge base — Zotero Forums](https://forums.zotero.org/discussion/100682/possible-future-of-zotero-as-a-personal-knowledge-base)
- ["Related Items" is almost useful — Zotero Forums](https://forums.zotero.org/discussion/120613/related-items-is-almost-useful)
- [Bringing everything together in Zotero (PKM) — Zotero Forums](https://forums.zotero.org/discussion/124825/bringing-everything-together-in-zotero-personal-knowledge-management-pkm)
- [Local LLM with Zotero as Knowledgebase — Zotero Forums](https://forums.zotero.org/discussion/123626/local-llm-with-zotero-as-knowledgebase)
- [Better Notes for Zotero — Zotero Forums](https://forums.zotero.org/discussion/96945/better-notes-for-zotero-a-knowledge-based-note-manager-in-zotero)
- [Zotero 7 highlighting and annotation wishlist — Zotero Forums](https://forums.zotero.org/discussion/108807/zotero-7-highlighting-and-annotation-overview-unexpected-results-and-wishlist)

**Academic workflows:**
- [An Updated Academic Workflow: Zotero & Obsidian — Alexandra Phelan, Medium](https://medium.com/@alexandraphelan/an-updated-academic-workflow-zotero-obsidian-cffef080addd)
- [Using Obsidian as an Academic — Obsidian Forum](https://forum.obsidian.md/t/using-obsidian-as-an-academic/59159)
- [Top 7 Solutions for Academic Knowledge Organization with Zotero and PKM Apps — Automatic Backlinks](https://www.automaticbacklinks.com/blog/top-7-solutions-for-academic-knowledge-organization-with-zotero-and-pkm-apps-7257)
- [How I use Obsidian for academic work — Emile van Krieken (2025)](https://www.emilevankrieken.com/blog/2025/academic-obsidian/)

**PKM abandonment:**
- [The Fall of Roam — Every.to](https://every.to/superorganizers/the-fall-of-roam)
- [Why I'm Giving Up on a Second Brain — Sudo Science (Nov 2025)](https://sudoscience.blog/2025/11/08/why-im-giving-up-on-a-second-brain/)
- [It's a Tool, Not a Goal — Sébastien Dubois](https://www.dsebastien.net/its-a-tool-not-a-goal-why-your-pkm-system-should-stay-simple/)
- [PKM in 2025: Why We're Not Just Taking Notes Anymore — Medium](https://medium.com/@ann_p/pkm-in-2025-why-were-not-just-taking-notes-anymore-f7dae509f622)

**Competitor analysis:**
- [NotebookLM Limitations: 8 Gaps Google Won't Tell You — Atlas Workspace](https://www.atlasworkspace.ai/blog/notebooklm-limitations)
- [NotebookLM feels powerful until you try to do these 5 basic things — XDA Developers](https://www.xda-developers.com/notebooklm-limitations/)
- [Wild idea: NotebookLM-like search in Zotero — Zotero Forums](https://forums.zotero.org/discussion/118230/wild-idea-notebooklm-like-search-in-zotero)
- [ZotAI — The AI Research Assistant](https://zotai.app/)
- [ZotAI reviews — There's An AI For That](https://theresanaiforthat.com/s/zotai/top-rated/)
- [PaperQA2: Superhuman scientific literature search — FutureHouse (WikiCrow)](https://www.futurehouse.org/research-announcements/wikicrow)
- [Systematic Literature Review with Elicit AI — Boris Nikolaev, Medium](https://medium.com/@borisnikolaev_57179/systematic-literature-review-with-elicit-ai-4-practical-use-cases-limitations-002e295caf11)
- [Comparison of Elicit AI and Traditional Literature Searching — PMC](https://pmc.ncbi.nlm.nih.gov/articles/PMC12483133/)
- [Best Literature Review Software — Atlas Workspace](https://www.atlasworkspace.ai/blog/literature-review-software)

**llm-wiki-compiler ecosystem:**
- [llm-wiki gist — Karpathy](https://gist.github.com/karpathy/442a6bf555914893e9891c11519de94f)
- [llm-wiki-compiler — ussumant (Claude Code plugin)](https://github.com/ussumant/llm-wiki-compiler)
- [llm-wiki-compiler — atomicmemory (the knowledge compiler)](https://github.com/atomicmemory/llm-wiki-compiler)
- [LLM Wiki — nashsu (cross-platform desktop app)](https://github.com/nashsu/llm_wiki)
- [llm-for-zotero — yilewang](https://github.com/yilewang/llm-for-zotero)
- [LLM Knowledge Bases — DAIR.AI Academy](https://academy.dair.ai/blog/llm-knowledge-bases-karpathy)
- [LLM Wiki v2 — rohitg00 gist](https://gist.github.com/rohitg00/2067ab416f7bbe447c1977edaaa681e2)

**PDF parsing benchmarks:**
- [OmniDocBench — CVPR 2025 (arXiv 2412.07626)](https://arxiv.org/abs/2412.07626)
- [Docling vs MinerU: I Tested Both (2025) — CodeSOTA](https://www.codesota.com/ocr/docling-vs-mineru)
- [PDF Data Extraction Benchmark 2025: Docling, Unstructured, LlamaParse — Procycons](https://procycons.com/en/blogs/pdf-data-extraction-benchmark/)
- [Which PDF Parser Should You Use? Docling, Marker, MinerU, olmOCR — Soup.io](https://www.soup.io/which-pdf-parser-should-you-use-comparing-docling-marker-netmind-parsepro-mineru-olmocr)

**Literature review meta-research:**
- [How to Use AI for Literature Review in 2026 — The Effortless Academic](https://effortlessacademic.com/how-to-use-ai-for-your-literature-review-in-2024/)
- [Best Literature Review Tools in 2026 — ReadWonders](https://www.readwonders.com/blog/best-literature-review-tools-2026-ai-vs-traditional)
- [Better Notes plugin — GitHub (windingwind)](https://github.com/windingwind/zotero-better-notes)
