# Research Wiki Compiler — Product Brief

**Hypotheses:** H13 (Personal Research Wiki) + H17 (Annotation-Centric Compile)  
**Layer:** Foundation — everything downstream depends on this  
**Build estimate:** 4 weeks to MVP (30 papers)  
**Pain:** 5/5 | **Effort:** 2/5 (revised from 3 after Karpathy validation) | **Competition:** 2/5

**Phase A status (2026-04-12):** Discriminant question resolved — structure problem confirmed, not retrieval. Gap confirmed. Target user confirmed. H17 status: latent signal, requires Phase B side-by-side to validate as differentiator. Field scope decision: MVP should target empirical fields (biology, social science, medicine); flag formula-heavy content (CDM ~67% on math equations). See `pain-discovery-report.md` for full findings.

---

## Problem

Researchers accumulate papers but cannot synthesize across them.
A typical Zotero library of 200+ papers contains years of reading that is practically inaccessible:
- Papers are findable by title/author but not by concept or claim
- Annotations live inside PDFs, disconnected from each other
- No persistent conceptual map exists across the library
- Every time a researcher starts a new paper, they reconstruct knowledge from scratch

RAG tools answer questions about individual papers. They do not build a cumulative, inspectable knowledge structure.

---

## Solution

A compile pipeline that transforms a Zotero library into a personal research wiki:
- **Entity pages** for concepts, methods, findings, and claims
- **Paper pages** with structured summaries and source provenance
- **Automatic backlinks** between related entities
- **Annotation integration**: researcher highlights and notes as first-class compile inputs (not decoration)
- Output: flat markdown vault, queryable by any LLM agent without additional infrastructure

The wiki compounds over time. Each new paper added enriches existing entity pages and creates new backlinks.

---

## Target User

PhD students and academic researchers with an active Zotero library (50–300 papers) who:
- Are in a literature-heavy phase (comprehensive exam, dissertation writing, grant preparation)
- Already use annotations and highlights when reading
- Are comfortable with markdown and command-line tools

Secondary target: research labs that want a shared conceptual map of their reading.

---

## MVP Scope (30 papers, 4 weeks)

**What it builds:**
1. Zotero → ingest layer: metadata + fulltext + annotations extracted per paper via Zotero MCP
2. Hash-tracked staging: only new or changed papers re-compiled on each run
3. LLM compile phase (Sonnet subagents, one per paper): key claims, methods, results, concepts
4. Annotation-aware compile: researcher highlights integrated with source page numbers preserved
5. Orchestrator pass (Opus): create/update entity pages, add backlinks, resolve conflicts
6. Output vault: `papers/`, `concepts/`, `_index.md`, `_sources.md`

**Success criteria for MVP:**
- Researcher can ask "what does my library say about X?" and get a useful, cited answer
- Entity pages have backlinks that the researcher did not manually create
- Annotations from their own reading appear as evidence in entity pages

**What is explicitly out of scope for MVP:**
- Web UI or app — output is markdown files
- Multi-user or shared wikis
- Automatic re-compilation on new Zotero imports (manual trigger only)
- Integration with Obsidian vault (keep AI wiki separate from human notes)
- Mathematical/physics fields with heavy equation content (formula CDM ~67%; defer until Mathpix/equation-aware parsing is integrated)

**Distribution:** Claude Code skill pack. No server. No new infrastructure.

---

## Architecture (4-Phase Cycle)

```
Zotero library (PDFs + metadata + annotations via Zotero MCP)
        ↓  [ingest]
raw/ staging: metadata.md + fulltext.txt + annotations.md per paper
hash-tracked against _index.md
        ↓  [compile — Sonnet subagents]
extract: key claims, methods, results, concepts, cited work relationships
annotation integration: highlight text → claim evidence with page provenance
        ↓  [orchestrate — Opus]
create/update entity pages, add backlinks, resolve conflicts, update index
        ↓  [lint — periodic]
stale articles, broken links, contradictions between papers
```

---

## Competitive Landscape

- **ZotAI** — Q&A over Zotero PDFs + annotations; RAG-based, not wiki compilation; no entity pages or backlinks
- **PaperQA** (Future-House) — high-accuracy RAG for scientific PDFs; agentic query; no persistent knowledge structure
- **NotebookLM** — Q&A over uploaded sources; capped at 50 sources; no Zotero integration; no persistent wiki
- **Atlas** — auto-builds connections from uploads; silo; no Zotero/Obsidian integration
- **llm-wiki-compiler, llm-knowledge-bases, wiki-skills** — Claude Code skill packs for generic wikis; no academic PDF or Zotero support

**The confirmed gap:** No tool combines wiki compilation (Karpathy pattern) + academic papers + Zotero annotation awareness.

---

## Where Downloader Fits

Zotero MCP retrieves full text for open-access papers.
For paywalled papers (majority of scholarly literature), Downloader fills the retrieval gap.
Downloader is the PDF acquisition layer that feeds the ingest stage.

---

## Hypotheses Unlocked After This Ships

Once the wiki MVP is running with 30 papers:
- **H2 Research Memory** — falls out of the query+enhance phase automatically
- **H6 Backlog Triage** — score new Zotero imports against wiki concept proximity (single prompt)
- **H11 Research Context Agent** — any agent reads wiki markdown directly; zero added infrastructure
- **H18 Epistemic Versioning** — git history accumulates automatically; 2-day add-on
- **H19 Argument Graph** — claim nodes extracted during compile become the substrate
