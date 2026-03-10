---
stepsCompleted: [1, 2, 3, 4, 5]
status: complete
inputDocuments:
  - "_bmad-output/planning-artifacts/research/market-downloader-future-strategy-research-2026-03-08.md"
  - "_bmad-output/planning-artifacts/product-brief-Downloader-2026-01-20.md"
  - "_bmad-output/project-context.md"
date: 2026-03-08
author: fierce
v2_date: 2026-03-09
v2_audit: audit-10-expert-product-strategy-2026-03-09.md
---

# Product Brief: Downloader

## Executive Summary

Downloader should evolve into the trusted evidence-acquisition and corpus-preparation layer for research workflows. The market signal is clear: researchers and research-adjacent teams are adopting AI quickly, but their workflows remain fragmented and trust in outputs remains low. Existing tools either manage citation libraries, summarize papers, or provide general-purpose document ingestion. Very few own the step between "I found relevant sources" and "I have a clean, reusable corpus I can trust."

Downloader's strongest near-term opportunity is to serve academic power users and other high-volume evidence handlers who need to turn mixed URLs, DOIs, and bibliographies into normalized, traceable local corpora. The longer-term expansion path is to become the intake layer for governed research and AI knowledge workflows, but only after the initial wedge proves durable.

This brief therefore recommends a disciplined strategy: wedge first, infrastructure second, suite never. Downloader should compete on determinism, provenance, corpus hygiene, and workflow composability rather than on generic AI assistance or full reference-manager UX.

---

## Core Vision

### Problem Statement

The core problem is not source discovery. The harder problem is turning discovered sources into trustworthy, usable evidence. Researchers, librarians, analysts, and AI knowledge builders regularly collect URLs, DOIs, and bibliographies from heterogeneous places, but the handoff from "candidate sources" to "working corpus" is still fragmented, manual, and error-prone.

### Problem Impact

When this handoff breaks down, evidence workflows degrade quickly. Users lose time downloading one item at a time, cleaning metadata, resolving duplicates, and reorganizing files before they can cite, share, or analyze anything. For AI-assisted workflows, the impact is larger: weak acquisition and messy corpora produce lower-trust downstream outputs, higher review overhead, and reduced willingness to operationalize the workflow.

### Why Existing Solutions Fall Short

Classical reference managers such as Zotero, Mendeley, EndNote, Paperpile, and ReadCube are strong at organizing libraries, citing sources, and supporting reading workflows. Zotero in particular has meaningful batch capabilities: "Add Items by Identifier" accepts multiple DOIs/ISBNs at once, the browser connector can capture multiple items from a page, and BibTeX/RIS import is standard. These tools are not, however, optimized around acquisition-first, mixed-input corpus preparation.

Where Downloader differs from Zotero and other reference managers:
- **Mixed-input tolerance**: Downloader processes URLs, DOIs, and direct links in a single pass. Zotero requires separate workflows for identifiers vs web pages vs bibliography files.
- **Site-specific resolvers**: 7 resolvers (arXiv, Crossref, IEEE, PubMed, ScienceDirect, Springer, YouTube) encode domain-specific URL patterns, API quirks, and access strategies. Zotero's translators focus on metadata extraction for library management.
- **Explicit per-item failure reporting**: Completion summary with per-item status, reason for failure, and provenance. Reference managers may silently skip items or show generic errors.
- **CLI/automation-first**: Scriptable, repeatable, pipeline-friendly. Reference managers are GUI-first.
- **Portable corpus output**: Self-contained directory with JSON-LD sidecar metadata. No database dependency.

For a detailed capability comparison, see the companion document: `research/zotero-batch-benchmark-2026-03-09.md`.

AI-native tools such as Elicit, Consensus, Scite, and NotebookLM accelerate discovery and synthesis, but they compete primarily on answer quality and source-grounded analysis, not on deterministic batch retrieval and corpus hygiene. Enterprise ingestion vendors address a different layer entirely: they are broader and heavier than most research users need at the start.

The result is a workflow gap. Users can find things, cite things, or summarize things, but they still lack a trusted, purpose-built layer for collecting, normalizing, and packaging evidence.

### Proposed Solution

Downloader should become the product that reliably transforms mixed source inputs into reusable evidence corpora. The product accepts URLs, DOIs, and bibliography-style inputs, resolves and downloads what it can, preserves provenance, normalizes naming and structure, and prepares outputs for downstream use in citation managers, shared repositories, and LLM pipelines.

The initial product expression remains focused and pragmatic: a powerful, deterministic workflow tool for people who handle evidence at volume. Over time, the same core can expand into APIs, automation hooks, and governed deployment patterns for team and enterprise research workflows.

### Key Differentiators

1. **Deterministic acquisition**: Downloader makes retrieval explicit, inspectable, and reliable rather than implicit inside broader tools.
2. **Provenance by default**: Source URLs, DOIs, and retrieval paths remain visible and traceable.
3. **Corpus hygiene**: Normalization, deduplication, sidecar metadata, and predictable local structure are part of the value proposition.
4. **Workflow composability**: Downloader fits into existing research stacks instead of demanding that users abandon Zotero, Mendeley, Papers, NotebookLM, or AI workflows they already use.
5. **Rights-aware realism**: It treats auth, redirects, blocked sources, and real-world web friction as first-class workflow problems.

## Corpus Definition

A **corpus** is the primary output of a Downloader run: a local directory containing downloaded files, structured metadata, and a manifest. The corpus is self-contained and portable — no database required.

### Directory Structure

```
my-corpus/
├── attention_is_all_you_need_a1b2c3d4.pdf
├── attention_is_all_you_need_a1b2c3d4.json    ← sidecar metadata
├── deep_residual_learning_e5f6g7h8.pdf
├── deep_residual_learning_e5f6g7h8.json
├── ...
└── manifest.json                                ← corpus manifest
```

### File Naming

Downloaded files use the pattern `{sanitized_title}_{hash8}.{ext}`, where `hash8` is an 8-character hash suffix for uniqueness.

### Sidecar Metadata Format

Each downloaded file has a companion `.json` sidecar (replacing the file's original extension) containing Schema.org/ScholarlyArticle JSON-LD:

```json
{
  "@context": "https://schema.org",
  "@type": "ScholarlyArticle",
  "name": "Attention Is All You Need",
  "author": [
    { "@type": "Person", "name": "Ashish Vaswani" },
    { "@type": "Person", "name": "Noam Shazeer" }
  ],
  "datePublished": "2017",
  "identifier": {
    "@type": "PropertyValue",
    "propertyID": "DOI",
    "value": "10.48550/arXiv.1706.03762"
  },
  "url": "https://arxiv.org/pdf/1706.03762"
}
```

Standard metadata keys across all resolvers: `title`, `authors`, `doi`, `year`, `source_url`.

All fields except `url` are optional and populated when available from the source. The sidecar is generated by `downloader-core/src/sidecar/mod.rs`.

### Manifest

A JSON file listing all items in the corpus with their status, provenance, and file paths. Enables programmatic inspection of what was fetched, what failed, and why.

## AI-Readiness Scope

Downloader produces clean files and structured metadata. It does **not** provide downstream AI processing.

**What Downloader provides:**
- Downloaded files in their original format (PDF, HTML, etc.)
- Structured JSON-LD metadata per file (Schema.org vocabulary)
- Predictable directory layout suitable for batch processing
- Explicit provenance linking each file to its source URL/DOI

**What Downloader does NOT provide:**
- Text extraction from PDFs or HTML
- Chunking, splitting, or segmentation
- Embeddings or vector representations
- RAG-ready formats (no pre-built indexes or retrieval structures)

Users feed Downloader corpora into their own downstream pipelines: text extractors, embedding models, RAG frameworks, or analysis tools. Downloader's job ends at producing a clean, well-structured corpus.

**Future consideration:** Optional text extraction add-on is a potential Phase 2+ feature, deferred until MVP validation confirms demand.

## Target Users

### Primary Users

**Maya, the academic power user**

Maya is a PhD candidate or research staff member working across dozens of papers, preprints, references, and citation trails at a time. She already uses tools like Zotero, browser tabs, spreadsheets, shared drives, and ad hoc scripts, but her intake workflow is still fragmented. She often reaches the same point in every project: she has a list of promising sources, but not a clean corpus she can trust and reuse.

Her motivation is not novelty. She wants to move faster without losing rigor. She feels the pain in very practical ways: downloading one item at a time, checking whether a PDF is the right one, cleaning file names, fixing metadata, and losing track of what came from where. Emotionally, the problem shows up as overload and low-grade anxiety about whether something important is missing or mislabeled.

Success for Maya looks like this: she drops in a mixed list of URLs, DOIs, or bibliography entries and gets back a well-structured, traceable corpus that is ready for citation management, reading, and AI-assisted analysis. Her “this is exactly what I needed” moment is when she realizes she no longer has to babysit corpus setup.

**Evan, the research-heavy analyst**

Evan works in a think tank, policy group, or research-heavy business role. He is not a formal academic, but he behaves like one in practice: he gathers lots of source material, compares evidence across documents, and needs a workflow that is fast, defensible, and reusable. He cares less about citation perfection than Maya does, but he cares just as much about traceability and repeatability.

For Evan, the problem is that evidence handling sits in the middle of a larger workflow with deadlines. If corpus preparation is slow or messy, the rest of the analysis slips. Downloader fits when it becomes the reliable intake utility that prepares evidence without demanding a heavyweight new system.

### Secondary Users

**Priya, the knowledge operations lead**

Priya works on a team building an internal knowledge base, research repository, or RAG-ready corpus. She is less concerned with an individual paper and more concerned with the repeatability of the intake process. Her priorities are governance, provenance, predictable outputs, and smooth handoff into downstream systems.

Priya is not the most natural first user for Downloader, but she is an important second-phase user because she influences whether a wedge product graduates into a team workflow component. She benefits when Downloader can prove that corpus ingestion is reliable, inspectable, and compatible with governed workflows.

**Lena, the research librarian**

Lena evaluates research tools for her department and advises faculty on workflow best practices. She manages institutional proxy access, negotiates with publishers, and needs batch acquisition tools that respect publisher agreements. She influences purchasing and adoption decisions for entire research groups.

Her priorities are different from Maya's: she cares less about personal speed and more about institutional compatibility, compliance, and reproducibility. Downloader earns her trust by being transparent about what it does (robots.txt compliance, rate limiting, no paywall circumvention) and producing auditable output.

Success for Lena looks like this: she can recommend Downloader to faculty with confidence that it will not violate institutional access agreements, and she can verify its behavior through clear documentation and inspectable output.

**Other librarians, research support staff, and technical champions**

These users may not be daily operators, but they are credibility brokers. They help other users choose tools, validate workflows, and set expectations for trustworthy handling of sources. Their approval can strongly influence adoption in academic and institutional settings.

### User Journey

**Discovery**

Users discover Downloader when the volume or messiness of source intake becomes painful enough to justify a dedicated workflow. They may hear about it through peers, academic communities, librarians, documentation, or adjacent workflows involving Zotero, bibliography processing, or AI-ready corpora.

**Onboarding**

The first experience should be simple and concrete: provide a list of sources, run the workflow, and inspect what came back. The product must make success legible immediately through clear file output, metadata, and provenance rather than through abstract promises.

**Core Usage**

The product becomes useful when it is part of a repeated intake rhythm: collecting sources for a new project, adding a bibliography, pulling materials for a literature review, or preparing documents for downstream analysis.

**Success Moment**

The “aha” moment is not merely that files downloaded. It is that a messy source list has become a clean, trustworthy corpus that can actually be cited, searched, shared, or fed into another system.

What the user sees: a completion summary with green/yellow/red status per item, a provenance card per file showing source URL and metadata, and final counts: *”41 of 47 sources acquired, 4 behind paywall, 2 DOIs unresolvable.”* The corpus directory is ready to open, browse, or feed into the next tool.

What triggers the feeling: speed (minutes, not hours), transparency (nothing is hidden or silently dropped), and clean output (every file named, every source traced).

**Long-term**

Downloader earns a durable place in the workflow when users stop thinking of it as a downloader and start relying on it as the intake layer for their evidence stack. For individual users, that means repeat project use. For teams, that means standardizing parts of the workflow around it.

## Success Metrics

Downloader succeeds when users trust it with real evidence intake and keep coming back because it reduces corpus-preparation work in a measurable way.

### User Success

The primary user outcome is simple: a user turns a messy source list into a trustworthy, reusable corpus with minimal manual cleanup.

Users will know the product is working when:

- they can process mixed source inputs without babysitting the workflow
- outputs are clean enough to move directly into citation, reading, or AI analysis workflows
- they can understand what was fetched, what failed, and why
- they return to Downloader for the next project instead of reverting to manual collection

The core “aha” moment is: “I gave it a chaotic list of sources and got back a corpus I can actually use.”

### Business Objectives

**At 3 months**

- validate that academic power users repeatedly use Downloader for real research projects
- prove that the retrieval-and-preparation wedge is strong enough to stand on its own
- identify the most important gaps in metadata quality, provenance visibility, and downstream handoff

**At 12 months**

- establish Downloader as a trusted intake utility for serious evidence-heavy workflows
- grow from a solo power-user tool into a workflow component with strong export and automation surfaces
- generate clear evidence about whether team and enterprise-adjacent packaging is justified

### Key Performance Indicators

| KPI | Target | Measurement Method | Cadence |
|-----|--------|-------------------|---------|
| Corpus completion rate | ≥ 80% on benchmark sets of 25+ mixed items | Automated benchmark runs against reference test sets | Monthly |
| Manual cleanup rate | < 15% of items require manual intervention | Design partner self-report + post-run surveys | Biweekly during Phase 1 |
| Provenance completeness | ≥ 95% of successfully fetched items have full sidecar metadata | Automated check: sidecar exists + has title + source_url | Monthly |
| Repeat project usage | ≥ 60% of design partners use on 2+ real projects | Design partner tracking | End of Phase 1 |
| Downstream handoff success | ≥ 70% of active users export or use corpus in another tool | Self-reported in feedback sessions | Quarterly |
| Hard-case resolver success | Improving quarter-over-quarter on auth-walled and redirect-heavy sources | Benchmark test set with known hard cases | Quarterly |
| Retention among heavy users | ≥ 50% monthly active retention among users with 3+ prior runs | Usage tracking (opt-in telemetry or self-report) | Monthly |

### Measurement Principles

- prefer workflow outcome metrics over vanity adoption metrics
- measure success at the corpus level, not just the file-download level
- treat repeat use and low cleanup burden as stronger signals than one-time activation
- use enterprise-adjacent metrics only after the power-user wedge is validated

## Sustainability Model

**Current posture:** Open-source (MIT/Apache 2.0 dual license).

**Sustainability options under consideration:**

| Model | Description | Timing |
|-------|-------------|--------|
| Open-core | Free CLI + core library; paid team/enterprise features (shared workspaces, managed hosting, priority support) | Phase 3+ |
| Academic grants | NSF, Mellon, Sloan, or similar funding for research infrastructure tools | Phase 2+ |
| Hosted API | Pay-per-use cloud service for teams that don't want to run their own instance | Phase 3+ |
| Sponsorship | GitHub Sponsors, Open Collective, or institutional sponsorship | Phase 1+ |

**Firm commitment:** The core library (`downloader-core`) and CLI (`downloader-cli`) will always remain open-source and free.

**Decision trigger:** Revisit sustainability model after Phase 1 validation confirms the wedge is durable and user demand patterns are clear.

## MVP Scope

### Core Features

The MVP should do one job exceptionally well: transform mixed research source inputs into a trustworthy, reusable corpus.

### Supported Input Formats

| Format | Status | Notes |
|--------|--------|-------|
| Plain-text URL lists (one per line) | **Implemented** | Direct URLs to PDFs, HTML pages, or landing pages |
| DOI strings (e.g., `10.1234/example`) | **Implemented** | Resolved via Crossref and site-specific resolvers |
| Direct download URLs | **Implemented** | Passed through Direct resolver |
| arXiv identifiers and URLs | **Implemented** | Dedicated arXiv resolver |
| PubMed IDs and URLs | **Implemented** | Dedicated PubMed resolver |
| YouTube watch URLs | **Implemented** | oEmbed metadata + transcript extraction |
| BibTeX files (.bib) | **Planned** | Parser for incoming bibliography references |
| RIS files (.ris) | **Planned** | Standard reference interchange format |
| CSV with DOI/URL columns | **Planned** | Spreadsheet-friendly input |

Core features:

- mixed input handling for URLs, DOIs, and source lists across the formats above
- deterministic acquisition with explicit success and failure reporting
- source-aware resolution for heterogeneous research destinations
- provenance-preserving metadata capture tied to each retrieved item
- corpus normalization including predictable structure, naming, and basic deduplication
- outputs that fit downstream research workflows, especially local archives, citation tools, and AI-ready corpus preparation

### Out of Scope for MVP

To preserve focus, the MVP should explicitly defer:

- full reference-manager functionality such as writing plugins, citation formatting workflows, and library-of-record UX
- broad AI assistant functionality such as in-product synthesis, question answering, or generic research chat
- team collaboration features such as shared workspaces, permissions, and approval flows
- enterprise control-plane features such as extensive connectors, orchestration, policy administration, and deployment management
- browser-extension or broad GUI-first experiences unless they directly unblock the wedge

### Legal and Ethical Considerations

Batch downloading academic content at scale raises publisher Terms of Service concerns. Downloader addresses this through responsible design:

**Current mitigations (implemented):**
- **robots.txt compliance**: Fetches and respects robots.txt per origin with 24-hour cache TTL (`download/robots.rs`)
- **Per-domain rate limiting**: Configurable minimum delay between requests to the same domain, with optional jitter to avoid regular spacing (`download/rate_limiter.rs`)
- **No paywall circumvention**: Resolvers surface auth requirements to the user rather than working around them
- **No DRM bypass**: Files are downloaded as-is from the source

**Planned additions:**
- User ToS acknowledgment on first run
- Per-publisher rate limit profiles with conservative defaults
- Clear "Responsible Use" documentation
- OA vs licensed content flagging in completion summary

For a detailed risk analysis, see the companion document: `legal-risk-assessment-Downloader-2026-03-09.md`.

### MVP Success Criteria

The MVP is successful if it proves three things:

1. users trust Downloader with real evidence intake work
2. the product materially reduces corpus-preparation effort
3. the outputs are good enough to become part of repeated downstream workflows

Signals that justify moving beyond MVP:

- strong repeat usage among academic power users
- clear evidence that users prefer Downloader over manual intake for new projects
- reliable output quality across mixed source sets
- demand for exports, automation, or team-compatible packaging rather than demand for generic AI features

### Future Vision

If the wedge proves durable, Downloader should evolve by extending the same core rather than abandoning it.

Future vision:

- richer metadata enrichment and stronger duplicate handling
- exports and integrations into mainstream research and AI tools
- APIs and automation hooks for workflow embedding
- team-friendly packaging for governed internal research and RAG corpus building
- selective intelligence features only where they amplify trust and workflow usefulness rather than dilute focus
