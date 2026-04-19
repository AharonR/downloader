# Why We're Building This
### Background & Motivation for H13 Phase 0 — Research Wiki Compiler

> This document doubles as the GitHub README introduction for the open source release.

---

## The Problem

Researchers accumulate papers. They don't accumulate understanding.

A typical researcher has 200-500 papers in Zotero. They've read most of them. They've highlighted passages, left margin notes, tagged things as important. But ask them what those 200 papers collectively say about a contested claim in their field — and they'll tell you they'd have to go back through their notes manually. The papers are there. The synthesis isn't.

This isn't a retrieval problem. Researchers already know how to find papers. Search works. The hard part is making sense of them — seeing how Smith 2019 conflicts with Jones 2021, noticing that three papers you read six months apart were all pointing at the same methodological assumption you're now making, understanding how your own thinking about a concept changed as you read more.

Current tools don't solve this. They make it worse in a predictable way.

---

## Why Existing Tools Fail

**NotebookLM** — Upload your sources, ask questions. Works for a session. Amnesiac by design: come back tomorrow and the conversation is gone. 50-source cap. No way to see how your sources connect. The #1 community complaint: "no visual representation of how sources relate or cluster."

**ZotAI** — Reads your Zotero PDFs and annotations, answers questions. The closest thing to what we're building, but fundamentally different: it's a Q&A interface, not compilation. It answers "what does paper X say about Y given my highlights" — it doesn't build a persistent, cross-paper index of your understanding. Every session starts from zero.

**PaperQA2** — Superhuman accuracy on scientific Q&A, genuinely impressive. But no entity pages, no backlinks, no persistent wiki. Great at answering individual questions; doesn't accumulate across sessions.

**Obsidian + manual linking** — Works until it doesn't. At 50-200 papers, manual link-building collapses. The documented failure pattern across PKM communities: capture is easy, synthesis is deferred, the vault becomes a mausoleum. "I now have a mass of stuff... several generations of systems cohabit and they don't talk to each other."

**GraphRAG** — The technical approach many assumed was the answer. Independent evaluation shows 39% win rate vs. the reported 66%. Hallucinates on QA tasks. Overkill for personal libraries. Requires a vector database and graph database you have to maintain.

The pattern across all of these: they answer questions in a session, or they require manual maintenance that collapses at scale. None of them compile your library into something that compounds.

---

## The Insight: LLM Wiki as Compile Phase

In April 2026, Andrej Karpathy published a gist describing how he manages his personal knowledge: a markdown wiki maintained by an LLM. Not a RAG system, not a vector database — a compiled, human-readable markdown wiki where an LLM reads raw sources and writes structured entity pages with backlinks.

The key observation: *"A large fraction of my recent token throughput is going less into manipulating code, and more into manipulating knowledge."*

The pattern is simple:
1. Drop raw sources into a `raw/` directory
2. LLM reads them and writes entity pages (one per concept, method, finding)
3. Entity pages cross-link to each other and back to source papers
4. A lint pass periodically audits for contradictions, gaps, stale entries

Within two weeks, the community had built five independent implementations. All of them were Claude Code skill packs. All of them worked on general markdown content.

None of them worked on academic papers. None of them knew what Zotero was.

---

## The Gap

The confirmed gap: **no project combines wiki-compilation (Karpathy pattern) + academic PDFs + Zotero annotation awareness.**

Every Zotero+LLM tool uses RAG. The LLM compile phase — where raw sources become a structured, interlinked, persistent wiki — has never been built for academic libraries.

This is the gap H13 Phase 0 fills.

---

## Why Zotero

Zotero is where academic researchers actually live. It's free, open source, and has ~8M active users. More importantly, it stores not just the papers but the researchers' relationship with those papers: highlights, sticky notes, colors, tags, page-specific annotations. These annotations are the highest-signal artifact in any research library — the researcher has already done the triage work, marking what was significant enough to note.

The Zotero MCP server (`54yyyu/zotero-mcp`) makes this data programmatically accessible: full text extraction, annotation retrieval with page numbers, recent items. It's the minimal integration point — one dependency instead of the multi-plugin chain (Better BibTeX + Obsidian Integration) that historically broke and drove researchers away from automation.

---

## Why Annotations Are the Primary Compile Unit

Most tools treat annotations as a secondary input — context for Q&A, or a search filter. We treat them as the primary compile unit.

Here's why: when you highlight a passage, you're making a salience judgment. You're saying "this matters." That judgment is more information-dense than anything an LLM can infer from reading the same paper cold. A 40-page paper might have 8 annotations. Those 8 annotations represent your selective attention across the whole paper — pre-filtered by relevance to your research.

Compiling from annotations first means the wiki reflects *your understanding* of the literature, not the model's summary of it. The output carries your intellectual signature.

This has a practical consequence: it makes the output worth sharing. The sharpest criticism of LLM wikis is "there's nothing personal about a knowledge base you filled by asking AI questions." Annotation-centric compilation is the direct answer to that criticism.

---

## The Parallel Paths Design

We run two compile passes in parallel on every paper:

- **Annotation path**: compile from your highlights and notes, using full text only for gap-filling context
- **Model path**: compile from full text only, as if you'd never annotated the paper

Then we diff them.

The diff is the product. Where the paths agree, the claim is high-confidence. Where only the annotation path surfaces something, it's researcher-salient — you highlighted it for a reason. Where only the model path surfaces something, we show it to you as "you may have missed this." Where they contradict each other, we surface a compile conflict rather than silently resolving it.

This turns a passive compilation step into an active reading partner.

---

## Why the PKM History Matters

Tools like Roam Research, Notion, and every "second brain" system before them tried to solve this problem and failed in a predictable pattern. Understanding why matters for the design of this tool.

The failure modes are well-documented:

**Stratification.** Systems don't collapse because content decays — they collapse because accumulated content from multiple eras becomes unnavigable. Notes from two years ago, written in a different conceptual vocabulary, coexist with recent notes and can't be connected.

**The deferral trap.** Capture is easy. Synthesis is deferred. The backlog of "I'll process this later" grows until the tool becomes a mausoleum. The LLM compile phase directly attacks this: the processing happens automatically, not when you remember to do it.

**Episodic trust breaks.** PKM abandonment is rarely gradual. It's triggered by a single episode: a plugin breaks, a sync fails, exported notes are garbled. Academic users don't migrate to the next tool — they exit the category. The design implication: minimize the integration surface. One MCP server, not a plugin chain.

**Compounding is aspirational; retrieval is actual.** Tools pitched on "your knowledge will compound over time" churn because the promised emergence doesn't arrive in the first session. The hook must be a retrieval win: "ask a question about your Zotero library and get an answer with annotation provenance, right now."

Every design constraint in this tool is derived from one of these failure modes. They're not preferences — they're load-bearing.

---

## The Competitive Window

llmwiki.app (launched April 2026) is the first standalone product built on the Karpathy pattern. It supports multi-format upload and Claude MCP. It is one Zotero MCP integration away from replicating this tool's core value proposition.

We estimate a 4-6 week window before that integration closes.

This is why Phase 0 is an open source preview, not a finished product. The goal is to plant a flag — working code on GitHub, researchers forking it on their own Zotero libraries — before the window closes. Prior art and community adoption matter more at this stage than polish.

---

## What This Unlocks

Phase 0 is not the destination. It's the substrate.

A working wiki — entity pages, backlinks, claim provenance — is the foundation for everything that follows:

- **H18 (Epistemic Versioning)**: the git history of your wiki is already a record of how your understanding changed over time. No additional build required.
- **H19 (Argument Graph)**: once entity pages exist with structured claims, typed logical edges (supports / contradicts / qualifies) can be added. This is the primary research contribution — the step that turns retrieval into reasoning.
- **N11 (Anti-Library)**: a read-only query on the argument graph topology that maps what you *haven't* read against what you're claiming. Falls out of H19 in a day.
- **H4 (Citation Verifier)**: uses the same Downloader pipeline to verify that the papers you cite actually say what you claim they say. Independent of the wiki; can run in parallel.

The wiki is infrastructure. Once it exists, the downstream tools are mostly queries on top of it.

---

## What We're Building

A Claude Code skill pack. Open source, MIT licensed. Researchers run it on their own Zotero library, on their own machine, against their own API key.

No SaaS. No vendor lock-in. No sending your research notes to a server. The output is plain markdown — readable in any editor, version-controlled with git, compatible with Obsidian.

The goal for Phase 0: a working pipeline on 20-50 papers, a README that gets a researcher to their first compiled wiki page in under 10 minutes, and a GitHub release that the academic tooling community can fork, extend, and improve.

---

*Full development spec: [`h13-phase0-spec.md`](./h13-phase0-spec.md)*
