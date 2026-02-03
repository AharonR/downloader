---
stepsCompleted: [1, 2, 3, 4, 5, 6]
status: complete
inputDocuments:
  - "_bmad-output/analysis/brainstorming-session-2026-01-15.md"
  - "_bmad-output/planning-artifacts/research/technical-authenticated-downloads-research-2026-01-20.md"
date: 2026-01-20
author: fierce
partyModeInsights:
  - "Core principle: Trust that knowledge is captured"
  - "Brand promise: Capture. Organize. Trust."
  - "Summaries: On-demand with auto-suggested topic teasers"
  - "Architecture: Simple keyword extraction first, LLM later"
---

# Product Brief: Downloader

## Executive Summary

Downloader is an information ingestion engine that transforms curated lists of sources into organized, searchable, LLM-ready knowledge. It bridges the gap between "I have URLs" and "I have usable documents" — a gap that existing tools leave unfilled.

The core insight: the bottleneck has shifted. Finding sources isn't the problem anymore. The friction is in downloading, organizing, and making content accessible for processing. Most tools solve one piece (downloaders fetch, reference managers catalog, note apps organize) but none provide the end-to-end flow from list to knowledge.

Downloader solves this with ruthless simplicity: paste a list, specify a project, and the tool handles download, organization, memory, and summarization. No decision fatigue. No manual steps that "don't get done." Just flow.

**Core Principle:** Trust that your knowledge is captured.

**Brand Promise:** Capture. Organize. Trust.

## Core Vision

### Problem Statement

Knowledge workers, researchers, and power users regularly need to acquire documents from curated lists — bibliographies, reference lists, research collections. The current workflow is fragmented: download manually one-by-one, drop files in folders (when motivated), use separate tools for cataloging (Zotero), and cobble together scripts for automation. The result is lost track of downloads, high manual friction, and organizational work that simply doesn't happen.

### Problem Impact

When organization doesn't happen, knowledge becomes inaccessible. Documents sit in Downloads folders, unsearchable. Projects lack the context of what was collected and why. The promise of LLM-powered processing goes unrealized because the inputs aren't ready. Time is wasted re-finding, re-downloading, and re-organizing.

### Why Existing Solutions Fall Short

| Tool Category | What It Does Well | Where It Fails |
|---------------|-------------------|----------------|
| Download Managers (wget, IDM, JDownloader) | Fast, reliable fetching | No memory, no organization, no project context |
| Reference Managers (Zotero, Mendeley) | Cataloging, citations | Poor download reliability, separate from workflow |
| Note/PKM Tools (Obsidian, Notion) | Organization, search | Don't handle acquisition — assume you already have files |
| CLI Utilities (curl, yt-dlp) | Powerful, scriptable | No state, no memory, requires glue code |

No tool provides the end-to-end flow: list → download → organize → remember → summarize.

### Proposed Solution

Downloader is a list-action engine with a simple core flow:

1. **Input:** Paste a list (URLs, DOIs, references)
2. **Context:** Specify the project
3. **Process:** Download + organize based on user-defined criteria
4. **Remember:** Everything is logged, searchable, queryable
5. **Summarize:** Per-project summaries generated on-demand

The tool fits the user's mental model — not a new system to learn, but an automation of what they already do manually. It outputs formats that integrate with existing tools and LLM workflows.

### Key Differentiators

1. **Mental Model Fit** — Works the way you think. No new paradigms, just automated flow.
2. **One Clear Way** — Zero decision fatigue. Paste, specify project, done.
3. **Memory by Default** — Everything remembered. "What did I download about X?" is always answerable.
4. **LLM-Ready Outputs** — Structured formats (JSON-LD, markdown, chunks) ready for AI processing.
5. **Organization That Happens** — Low enough friction that the work actually gets done.

### Progressive Value Layers

| Layer | Value | User Perception |
|-------|-------|-----------------|
| Base | Downloads reliably | "It works" |
| Core | Organizes automatically | "It saves me time" |
| Trust | Remembers everything | "I can find anything" |
| Intelligence | Suggests & summarizes | "It understands my work" |

MVP ships Layers 1-3. Layer 4 (intelligence teaser with auto-suggested topics) is visible but optional — progressive value revelation.

## Target Users

### Primary Users

**Power Users & Data Hoarders**
- **Profile:** Knowledge workers who collect information at scale — curated lists, bibliographies, reference collections
- **Current Pain:** Download manually one-by-one, drop files in folders "when important enough," lose track of what was downloaded and why, organizational work often doesn't get done
- **Tools Used:** Zotero for PDFs, basic scripts, manual folder management
- **Success Looks Like:** "I paste a list, specify a project, and everything is organized and searchable without me thinking about it"

**Researchers**
- **Profile:** Academics, analysts, anyone building knowledge bases for projects
- **Current Pain:** Fragmented workflow between finding sources and having them ready to use; LLM processing blocked because inputs aren't organized
- **Success Looks Like:** Per-project summaries, searchable archives, documents ready for AI processing

### Secondary Users

- **Future consideration:** Teams sharing organized collections, developers integrating via CLI/API
- **Not MVP focus**

### User Journey

1. **Discovery:** "I have 47 URLs and no patience to download them one by one"
2. **Onboarding:** Paste list → specify project → done
3. **Core Usage:** Repeat for each research session/project
4. **Success Moment:** "I can actually find that paper I downloaded last month"
5. **Long-term:** Trusted system for all knowledge intake — "everything I collect is here"

## Success Metrics

### User Success

**Core Success Indicator:** Users trust the system enough to keep using it.

**Measurable Outcomes:**
- **Download Success Rate:** 90%+ of documents download accurately without manual intervention
- **Seamless Experience:** Users don't need to troubleshoot or retry — it just works
- **Repeat Usage:** Users return with new reference lists and reports to process (the real signal of trust)

**The "Aha" Moment:** User pastes a list of 30+ references, walks away, comes back to find everything organized and searchable. They think: "I'm never doing this manually again."

### Business Objectives

**Product Type:** Personal tool → potential open source

**Success at 3 months:**
- Tool reliably handles your own workflow
- Core loop (list → download → organize → remember) works smoothly
- You use it for every research project

**Success at 12 months:**
- Stable enough to share / open source
- Handles edge cases gracefully (auth sites, rate limits, various formats)
- Others could adopt it without hand-holding

### Key Performance Indicators

| KPI | Target | Why It Matters |
|-----|--------|----------------|
| Download success rate | ≥90% | Core promise — it works |
| Organization accuracy | Files end up where expected | Trust in the system |
| Personal usage frequency | Used for every new project | You've adopted your own tool |
| Manual intervention rate | <10% of downloads need help | Seamless experience |

## MVP Scope

### Core Features

**Input Handling**
- URLs (direct links)
- DOIs (resolve to source)
- Reference strings (parse and resolve)
- Pasted bibliographies (extract and process multiple references)

**Download Engine**
- Reliable fetching with retry logic
- Authenticated site support (university proxies, paywalls)
- Site-specific resolvers (handle different site rules/patterns)
- Rate limiting and polite crawling

**Organization**
- Project / sub-project structure
- Well-named files (derived from metadata)
- Index generation per project
- Predictable, searchable file structure

**Memory & Summarization**
- Everything logged and queryable
- On-demand per-project summaries
- Auto-suggested topic teasers (keyword extraction)

### Out of Scope for MVP

- Browser extension
- MCP / AI agent integration
- Watch folders / auto-import
- Team sharing / collaboration features
- Advanced deduplication (beyond basic URL matching)
- GUI (CLI-first, GUI is v2)

### MVP Success Criteria

- 90%+ download success rate across supported sites
- User completes full workflow (paste → download → organized) without manual intervention
- Tool used for every new research project (personal adoption)
- Auth sites work reliably (the hard differentiator)

### Future Vision (v2+)

- **Browser extension** — Capture references while browsing
- **MCP/AI integration** — Expose as tool for AI agents
- **Watch folders** — Auto-process dropped files/lists
- **Team features** — Shared collections, collaboration
- **GUI** — Desktop app with 90s aesthetic
- **Advanced intelligence** — LLM-powered summaries, semantic search

