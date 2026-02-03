---
stepsCompleted: [step-01-init, step-02-discovery, step-03-success, step-04-journeys, step-05-domain-skipped, step-06-innovation-skipped, step-07-to-12-consolidated]
status: complete
inputDocuments:
  - "_bmad-output/planning-artifacts/product-brief-Downloader-2026-01-20.md"
  - "_bmad-output/planning-artifacts/research/technical-authenticated-downloads-research-2026-01-20.md"
  - "_bmad-output/analysis/brainstorming-session-2026-01-15.md"
workflowType: 'prd'
documentCounts:
  briefs: 1
  research: 1
  brainstorming: 1
  projectDocs: 0
classification:
  projectType: desktop_app
  domain: general
  complexity: low
  projectContext: greenfield
---

# Product Requirements Document - Downloader

**Author:** fierce
**Date:** 2026-01-22

## Executive Summary

Downloader is an information ingestion engine that transforms curated lists of sources into organized, searchable, LLM-ready knowledge. It bridges the gap between "I have URLs" and "I have usable documents."

**Core Principle:** Trust that your knowledge is captured.

**Brand Promise:** Capture. Organize. Trust.

**Problem:** Knowledge workers download documents one-by-one, lose track of what's where, and never get around to organizing. The friction is high enough that organization simply doesn't happen.

**Solution:** Paste a list (URLs, DOIs, bibliographies), specify a project, and the tool handles download, organization, memory, and summarization. No decision fatigue. No manual steps.

**Key Differentiators:**
- Mental model fit — works the way you think
- Memory by default — everything remembered and searchable
- Auth site support — handles university proxies and paywalls
- LLM-ready outputs — structured formats for AI processing

**Tech Stack:** Tauri + Rust + reqwest (CLI-first, GUI in v2)

## Success Criteria

### User Success

**Core Promise:** Trust that knowledge is captured.

| Metric | Target | Measurement |
|--------|--------|-------------|
| Download success rate | ≥90% | Successful downloads / total attempted |
| Website success rate | Track per-site | Which sites work reliably |
| Manual intervention rate | <10% | Downloads requiring user action |
| Repeat usage | Consistent | User returns with new reference lists |

**The "Aha" Moment:** User pastes a bibliography, walks away, returns to find everything downloaded, organized, and indexed. "I'm never doing this manually again."

### Business Success

**Product Type:** Personal tool → potential open source

| Timeframe | Success Looks Like |
|-----------|-------------------|
| 3 months | Tool reliably handles your own workflow; used for every research project |
| 12 months | Stable enough to open source; others can adopt without hand-holding |

### Technical Success

| Metric | Target | Notes |
|--------|--------|-------|
| File naming accuracy | ≥95% | Files named correctly from metadata |
| Auth site success | ≥70% of sites | Authenticated sites work reliably |
| Concurrency | 10 parallel | Different websites simultaneously |
| List size | Up to 150 | No hard limit, 150 for MVP testing |
| Auth failure mode | Log and continue | Graceful degradation, don't block queue |
| Index generation | Correct without manual fix | Per-project index created automatically |

### Measurable Outcomes

- **MVP validation:** You use it for every new research project
- **Quality bar:** 90% downloads succeed, 95% named correctly, index works
- **Differentiator proof:** Auth sites work at 70%+ success rate

## Product Scope

### MVP - Minimum Viable Product

**Input Handling**
- URLs, DOIs, reference strings, pasted bibliographies

**Download Engine**
- Reliable fetching with retry logic
- Authenticated site support (70% success target)
- Site-specific resolvers
- Rate limiting, 10 concurrent (different sites)

**Organization**
- Project / sub-project structure
- Well-named files (95% accuracy)
- Index generation per project

**Memory & Summarization**
- Everything logged and queryable
- On-demand per-project summaries
- Auto-suggested topic teasers

### Growth Features (Post-MVP)

- Browser extension for link capture
- GUI with 90s aesthetic
- Watch folders / auto-import
- Advanced deduplication

### Vision (Future)

- MCP/AI agent integration
- Team sharing / collaboration
- LLM-powered summaries and semantic search
- Knowledge graph view

## User Journeys

### Journey 1: The Knowledge Collector (Primary User - Success Path)

**Persona:** Alex, a researcher who regularly receives bibliographies and reference lists from reports, papers, and AI-generated research summaries. Currently downloads files one-by-one, loses track of what's where, and never gets around to organizing properly.

**Opening Scene:**
Alex just finished a deep research session. They have a report with 47 references — PDFs, academic papers, web articles. The old way: open each link, download, rename, sort into folders. It takes hours and often doesn't happen. Documents pile up in Downloads, unfindable.

**Rising Action:**
Alex opens Downloader CLI. Pastes the bibliography. Types `--project "Climate Research Q1"`. Hits enter. The tool parses the messy references, resolves DOIs, handles the authenticated journal sites (Alex is logged into their university proxy in the browser), and starts downloading. Progress shows: `[32/47] Downloading... 3 queued, 2 failed (logged)`.

**Climax:**
Alex walks away. Coffee. Comes back 15 minutes later. Terminal shows:
```
✓ 44/47 downloaded successfully
✓ Organized to /Projects/Climate-Research-Q1/
✓ Index generated: index.md
⚠ 3 failed (see download.log for details)
```

**Resolution:**
Alex opens the project folder. Files are named properly (`Author_2024_Title.pdf`). The index shows everything at a glance with topics auto-detected. The 3 failures are logged with URLs and error reasons — Alex can retry manually or investigate later. No panic, no blocked queue.

**New Reality:** Alex now does this for every research session. "Paste, project name, done." The mental burden of "I should organize this" is gone. Everything is findable. The 10% that fails is logged, not lost.

### Journey 2: Error & Edge Case Handling (Logging-First Approach)

**Scenario:** Mixed-quality bibliography with broken links, paywalled content, and malformed references.

**How It's Handled:**

| Situation | Behavior | Logging |
|-----------|----------|---------|
| Download fails (404, timeout) | Log and continue | `download.log`: URL, error code, timestamp |
| Auth required but no session | Log and continue | `download.log`: "Auth required - [URL]" |
| Malformed reference | Best-effort parse, log uncertainty | `parse.log`: Original text, parsed result, confidence |
| Naming extraction fails | Use fallback name, log | `download.log`: "Naming fallback - [URL]" |
| Duplicate detected | Skip, log | `download.log`: "Duplicate skipped - [URL]" |

**Document-Level Logging:**
Each project gets a `download.log` that serves as:
- Audit trail of what was attempted
- Actionable list of failures to retry
- Debug info for improving site-specific resolvers

**Philosophy:** Never block the queue. Never lose information. Log everything, let the user decide what to investigate.

### Journey Requirements Summary

| Journey Element | Capabilities Required |
|-----------------|----------------------|
| Paste bibliography | Input parser (URL, DOI, reference string, BibTeX) |
| Specify project | Project/sub-project structure, CLI flags |
| Download with auth | HTTP client, cookie/session handling, site resolvers |
| Parallel processing | Queue manager, concurrency control (10 parallel) |
| Smart naming | Metadata extraction, naming templates |
| Index generation | Per-project index.md with topic detection |
| Error handling | Detailed logging, graceful degradation |
| Progress display | CLI progress indicator, summary on completion |

## Functional Requirements

### FR-1: Input Parsing

| ID | Requirement | Priority |
|----|-------------|----------|
| FR-1.1 | Accept direct URLs (http/https) | Must |
| FR-1.2 | Resolve DOIs to downloadable URLs | Must |
| FR-1.3 | Parse reference strings (Author, Year, Title format) | Must |
| FR-1.4 | Extract references from pasted bibliographies | Must |
| FR-1.5 | Accept BibTeX format | Should |
| FR-1.6 | Handle mixed-format input (URLs + DOIs + references) | Must |

### FR-2: Download Engine

| ID | Requirement | Priority |
|----|-------------|----------|
| FR-2.1 | Download files via HTTP/HTTPS | Must |
| FR-2.2 | Support authenticated sites via cookie/session capture | Must |
| FR-2.3 | Implement site-specific resolvers for common academic sites | Must |
| FR-2.4 | Retry failed downloads with exponential backoff | Must |
| FR-2.5 | Support concurrent downloads (configurable, default 10) | Must |
| FR-2.6 | Rate limit requests per domain | Must |
| FR-2.7 | Support resumable downloads (Range requests) | Should |

### FR-3: Organization

| ID | Requirement | Priority |
|----|-------------|----------|
| FR-3.1 | Create project folders from CLI flag | Must |
| FR-3.2 | Support sub-project organization | Must |
| FR-3.3 | Name files from metadata (Author_Year_Title.ext) | Must |
| FR-3.4 | Generate index.md per project with file listing | Must |
| FR-3.5 | Auto-detect topics via keyword extraction | Should |
| FR-3.6 | Store metadata as JSON-LD sidecar files | Should |

### FR-4: Logging & Memory

| ID | Requirement | Priority |
|----|-------------|----------|
| FR-4.1 | Log all download attempts with status | Must |
| FR-4.2 | Log failures with actionable error info | Must |
| FR-4.3 | Create per-project download.log | Must |
| FR-4.4 | Track parsing confidence for ambiguous references | Should |
| FR-4.5 | Enable querying past downloads | Should |

### FR-5: CLI Interface

| ID | Requirement | Priority |
|----|-------------|----------|
| FR-5.1 | Accept input via stdin (piped bibliography) | Must |
| FR-5.2 | Accept --project flag for organization | Must |
| FR-5.3 | Display progress during download | Must |
| FR-5.4 | Show summary on completion | Must |
| FR-5.5 | Support --dry-run for preview | Should |
| FR-5.6 | Support configuration file for defaults | Should |

## Non-Functional Requirements

### NFR-1: Performance

| ID | Requirement | Target |
|----|-------------|--------|
| NFR-1.1 | Parse 150 references | < 5 seconds |
| NFR-1.2 | Concurrent downloads | 10 parallel (different domains) |
| NFR-1.3 | Memory usage | < 200MB during operation |
| NFR-1.4 | Startup time | < 1 second |

### NFR-2: Reliability

| ID | Requirement | Target |
|----|-------------|--------|
| NFR-2.1 | Download success rate | ≥ 90% |
| NFR-2.2 | Auth site success rate | ≥ 70% |
| NFR-2.3 | Naming accuracy | ≥ 95% |
| NFR-2.4 | Graceful failure handling | Never crash, always log |

### NFR-3: Usability

| ID | Requirement | Target |
|----|-------------|--------|
| NFR-3.1 | Zero-config start | Works with defaults |
| NFR-3.2 | Clear error messages | Actionable, not cryptic |
| NFR-3.3 | Progress visibility | User knows what's happening |

### NFR-4: Maintainability

| ID | Requirement | Target |
|----|-------------|--------|
| NFR-4.1 | Site resolver modularity | Easy to add new resolvers |
| NFR-4.2 | Configuration flexibility | Overridable defaults |
| NFR-4.3 | Logging for debugging | Sufficient for resolver improvement |

## Technical Architecture Overview

### Stack

| Component | Technology | Rationale |
|-----------|------------|-----------|
| Framework | Tauri 2.0 | Small binary (~3MB), low memory, Rust backend |
| Backend | Rust | Memory safety, performance, native OS access |
| HTTP Client | reqwest | Async, cookie jar support, well-maintained |
| Frontend (v2) | Web (React/Svelte) | Simple UI, familiar tech |

### Architecture Pattern

```
┌─────────────────────────────────────────────────────────┐
│ CLI Interface                                           │
│  - Argument parsing                                     │
│  - Progress display                                     │
└─────────────────┬───────────────────────────────────────┘
                  │
┌─────────────────▼───────────────────────────────────────┐
│ Input Parser                                            │
│  - URL detection                                        │
│  - DOI resolution                                       │
│  - Reference string parsing                             │
│  - Bibliography extraction                              │
└─────────────────┬───────────────────────────────────────┘
                  │
┌─────────────────▼───────────────────────────────────────┐
│ Queue Manager                                           │
│  - Priority queue                                       │
│  - Concurrency control (semaphore)                      │
│  - Rate limiting per domain                             │
└─────────────────┬───────────────────────────────────────┘
                  │
┌─────────────────▼───────────────────────────────────────┐
│ Download Engine                                         │
│  - HTTP client (reqwest)                                │
│  - Cookie/session handling                              │
│  - Site-specific resolvers                              │
│  - Retry logic                                          │
└─────────────────┬───────────────────────────────────────┘
                  │
┌─────────────────▼───────────────────────────────────────┐
│ Organization Layer                                      │
│  - File naming from metadata                            │
│  - Project folder structure                             │
│  - Index generation                                     │
│  - Logging                                              │
└─────────────────────────────────────────────────────────┘
```

## Assumptions & Constraints

### Assumptions

1. User has browser sessions active for authenticated sites
2. User can provide cookies/session data for auth sites (browser extension in v2)
3. Most academic sites follow predictable patterns for PDF access
4. DOI resolution services (Crossref, Unpaywall) remain available

### Constraints

1. CLI-first for MVP — no GUI until v2
2. Single-user — no multi-tenant or team features in MVP
3. Local-first — all data stored locally, no cloud sync
4. No Electron — Tauri only, for small binary size

## Dependencies

### External Services

| Service | Purpose | Fallback |
|---------|---------|----------|
| Crossref API | DOI metadata resolution | Direct publisher link |
| Unpaywall API | Open access PDF locations | Skip, log as needs-auth |
| Publisher sites | Actual document downloads | Log failure, continue |

### Development Dependencies

- Rust toolchain
- Tauri CLI
- Node.js (for frontend in v2)

## Appendix: Site Resolver Strategy

Site-specific resolvers handle the differences between how various sites serve PDFs:

| Site Type | Pattern | Resolver Approach |
|-----------|---------|-------------------|
| Direct PDF links | URL ends in .pdf | Direct download |
| DOI-based | 10.xxxx/yyyy | Resolve via Crossref → publisher |
| Academic journals | ScienceDirect, JSTOR, etc. | Site-specific selectors/patterns |
| Open Access repos | arXiv, PubMed Central | Known URL patterns |
| General web | Varies | Best-effort, fallback naming |

Resolvers are modular — new sites can be added without changing core logic.

---

**Document Status:** Complete
**Ready for:** Architecture Design

