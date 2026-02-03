---
stepsCompleted: [1, 2, 3, 4]
inputDocuments: []
session_topic: 'Downloader overall concept, features, and technical approaches for site handling'
session_goals: 'Comprehensive feature list (MVP to future), novel differentiation approaches'
selected_approach: 'ai-recommended'
techniques_used: ['first-principles-thinking', 'cross-pollination', 'scamper', 'reverse-brainstorming', 'ai-era-fit']
ideas_generated: 62
context_file: ''
session_complete: true
---

# Brainstorming Session Results

**Facilitator:** fierce
**Date:** 2026-01-15

## Session Overview

**Topic:** Downloader's overall concept, feature set, and technical approaches for handling different sites

**Goals:**
- Generate comprehensive feature list spanning MVP through future versions
- Discover novel approaches that differentiate Downloader from existing tools (wget, curl, JDownloader, etc.)

### Project Context

Building a 90s-style reference list downloader with:
- Simple, intuitive UI with retro aesthetic
- Initial focus on PDFs from authenticated sites
- Growth path toward more formats, site-specific rules, complex link handling
- Gradual UI evolution matching capability growth

### Session Setup

**Scope:** Broad exploration covering concept, features, and technical differentiation
**Depth:** Both immediate (MVP) and long-term (evolution roadmap)
**Angle:** Innovation-focused - discovering what's uniquely compelling

## Technique Selection

**Approach:** AI-Recommended Techniques
**Analysis Context:** Product ideation with technical depth, practical and outcome-focused

**Recommended Technique Sequence:**

1. **First Principles Thinking** (Creative) - Strip away assumptions about what a "downloader" should be
2. **Cross-Pollination** (Creative) - Transfer solutions from other industries and domains
3. **SCAMPER Method** (Structured) - Systematic feature coverage
4. **Reverse Brainstorming** (Creative) - Find differentiation through inversion

**AI Rationale:** This sequence builds from fundamental truths → external inspiration → systematic coverage → unique positioning. Each phase builds on the previous while bringing fresh perspective.

---

## Technique Execution

### Phase 1: First Principles Thinking (21 ideas)

**Key Breakthroughs:**
- Core need isn't "downloading files" - it's "seamless information flow"
- Tool vs Product duality: both can coexist in layered architecture
- Context signals (CLI/GUI, Project) determine processing depth - no explicit modes
- Post-processing (indexing, memory) is core product; pre-processing (reference extraction) is plugin

| # | Idea | Essence |
|---|------|---------|
| 1 | Seamless Information Flow | Core need is information flow, not file download |
| 2 | Dual-Nature Architecture | Info-centric inside, file-based outside for compatibility |
| 3 | Progressive Capability Revelation | Ship simple, build smart |
| 4 | Two-Layer Architecture | Tool + Product coexist |
| 5 | Action-Oriented List Processing | Core primitive: list + actions |
| 6 | Native + Add-on Action System | Plugins from day one |
| 7 | Persistent Memory Layer | Queryable forever |
| 8 | Context-Aware Defaults | CLI vs GUI determines baseline |
| 9 | Universal History | Everything logged, even light mode |
| 10 | Signal-Based Processing Depth | No explicit modes |
| 11 | Retroactive Promotion | Past downloads gain intelligence later |
| 12 | UI + Project as Intent Signals | Two-axis system |
| 13 | Clear Core, Extensible Edges | Core is focused, plugins extend |
| 14 | Pre-processor Pipeline | Fuzzy → clean happens before core |
| 15 | The Irreducible Core | Robustness + normalization + actions + unified interface |
| 16 | Source → Information Transformation | Abstract over source chaos |
| 17 | Layered Output Representation | Tool: file+envelope. Product: extracted+structured |
| 18 | Envelope Architecture | Metadata is first-class |
| 19 | Hub Rules + Method Patterns | Two-tier: curated + generic |
| 20 | Rule Infrastructure | Future-proofed for community/AI |
| 21 | Product Layer as Module (TBD) | Separate but integrated |

---

### Phase 2: Cross-Pollination (16 ideas)

**Domains Raided:** Package managers, torrent clients, music libraries, read-it-later apps, web scrapers, PKM tools, Git

| # | Idea | Source Domain |
|---|------|---------------|
| 22 | Multi-Source Fallback | Package managers (registries) |
| 23 | Intelligent Local Cache | Package managers |
| 24 | Smart Queue with Priority | Torrent clients |
| 25 | Per-Site Rate Limiting | Torrent clients |
| 26 | Auto-Labeling Rules | Torrent clients |
| 27 | Watch Folders | Torrent clients |
| 28 | Order-Preserving Parallel Download | Torrent clients |
| 29 | Automatic Metadata Enrichment | Music libraries (MusicBrainz) |
| 30 | Content-Aware Duplicate Detection | Music libraries |
| 31 | Configurable Naming Templates | Music libraries |
| 32 | Built-in Viewer / Markdown Conversion | Read-it-later apps |
| 33 | Retry with Exponential Backoff | Web scrapers |
| 34 | Per-Project Settings Profiles | PKM tools |
| 35 | Knowledge Graph View (Future) | PKM tools |
| 36 | Independent Projects + Merge | Git (branches) |
| 37 | Multiple Custom Knowledge Bases | Git (repositories) |

---

### Phase 3: SCAMPER Method (15 ideas)

| # | Idea | SCAMPER Letter |
|---|------|----------------|
| 38 | Native DOI/BibTeX/ISBN Input | Substitute |
| 39 | Pluggable Output Destinations | Substitute |
| 40 | Storage-Agnostic Design | Substitute |
| 41 | Scheduled/Automated Fetching (Future) | Substitute |
| 42 | Team-Ready Architecture | Substitute |
| 43 | Preview Before Download | Adapt |
| 44 | Computed Metadata Fields (Future) | Adapt |
| 45 | Project Templates | Adapt |
| 46 | Rich Provenance Capture | Modify |
| 47 | Zero-Config Start | Modify |
| 48 | Semantic Progress Display | Modify |
| 49 | Actionable Error Handling | Modify |
| 50 | True "Just Download" Mode | Eliminate |
| 51 | No Account Required | Eliminate |
| 52 | Minimal First-Run | Eliminate |

---

### Phase 4: Reverse Brainstorming (3 ideas)

**Differentiators discovered by inverting "how to make it worse":**

| # | Idea | Differentiation |
|---|------|-----------------|
| 53 | "Remembers Everything" | Memory as brand promise (vs wget amnesia) |
| 54 | "Set It and Trust It" | Reliability as brand promise (vs fragile tools) |
| 55 | "One Input, Everything Handled" | End-to-end pipeline (vs manual steps) |

---

### Phase 5: AI-Era Fit (7 ideas)

| # | Idea | AI Integration |
|---|------|----------------|
| 56 | MCP Server / Tool API | Agents can call Downloader |
| 57 | LLM-Ready Output Formats | Markdown, chunks, JSON for RAG |
| 58 | Source-Attributed Chunks | Provenance in AI context |
| 59 | AI-Assisted Rule Generation | Self-improving rules |
| 60 | Natural Language Query (Future) | Conversational access |
| 61 | Webhook/Callback on Completion | Event-driven pipelines |
| 62 | Structured Result Manifest (Future) | JSON job report |

---

## Idea Clustering

### Cluster 1: Core Philosophy & Identity (8 ideas)
Ideas: 1, 4, 5, 15, 16, 53, 54, 55

### Cluster 2: Architecture & Data Model (8 ideas)
Ideas: 2, 3, 13, 17, 18, 21, 39, 40

### Cluster 3: Input Handling (3 ideas)
Ideas: 14, 38, 43

### Cluster 4: Download Engine & Reliability (9 ideas)
Ideas: 19, 20, 22, 23, 24, 25, 28, 33, 49

### Cluster 5: Processing & Actions (6 ideas)
Ideas: 6, 26, 29, 30, 31, 46

### Cluster 6: Knowledge Management (9 ideas)
Ideas: 7, 9, 11, 34, 35, 36, 37, 44, 45

### Cluster 7: User Experience & Interface (10 ideas)
Ideas: 8, 10, 12, 27, 32, 47, 48, 50, 51, 52

### Cluster 8: Automation & Scheduling (2 ideas)
Ideas: 27, 41

### Cluster 9: Collaboration (1 idea)
Ideas: 42

### Cluster 10: AI/Agent Integration (7 ideas)
Ideas: 56, 57, 58, 59, 60, 61, 62

---

## Prioritization Roadmap

### MVP (20 ideas) - CLI tool that works reliably

**Core Engine:**
- 5: Action-Oriented List Processing
- 19: Hub Rules + Method Patterns
- 23: Intelligent Local Cache
- 25: Per-Site Rate Limiting
- 24: Smart Queue with Priority
- 33: Retry with Exponential Backoff
- 49: Actionable Error Handling

**Input/Output:**
- 38: Native DOI/URL Input
- 18: Envelope Architecture
- 40: Storage-Agnostic Design

**User Experience:**
- 8: Context-Aware Defaults
- 47: Zero-Config Start
- 52: Minimal First-Run
- 48: Semantic Progress Display
- 50: True "Just Download" Mode
- 51: No Account Required

**History:**
- 9: Universal History

**Architecture Foundations:**
- 2: Dual-Nature Architecture
- 13: Clear Core, Extensible Edges
- 20: Rule Infrastructure

---

### v1 (21 ideas) - GUI, projects, AI integration, polish

Ideas: 1, 3, 4, 6, 12, 15, 16, 17, 22, 28, 29, 31, 34, 42, 46, 53, 54, 55, 56, 57, 61

**Key additions:**
- GUI interface
- Project system with settings
- Multi-source fallback
- Metadata enrichment from external DBs
- Configurable naming templates
- MCP/API for agent integration
- LLM-ready output formats
- Webhooks for pipelines

---

### v2 (16 ideas) - Product layer, advanced features

Ideas: 7, 10, 11, 14, 21, 26, 27, 30, 32, 36, 37, 39, 43, 45, 58, 59

**Key additions:**
- Full product layer (memory, indexing)
- Pre-processor plugins
- Watch folders
- Duplicate detection
- Built-in viewer
- Multiple knowledge bases
- AI-assisted rule generation

---

### Future (5 ideas) - Vision features

Ideas: 35, 41, 44, 60, 62

- Knowledge Graph View
- Scheduled/Automated Fetching
- Computed Metadata Fields
- Natural Language Query
- Structured Result Manifest

---

## Session Summary

**Total Ideas Generated:** 62
**Techniques Used:** First Principles, Cross-Pollination, SCAMPER, Reverse Brainstorming, AI-Era Fit

**Core Concept Crystallized:**
> Downloader is a **list-action engine** that transforms diverse sources into **unified actionable information**, with **context-aware depth** from quick CLI tool to persistent knowledge base. It sits in the gap between "pure download tools" (wget, curl) and "full knowledge management" (Notion, Obsidian) - automated intake that feeds intelligent memory.

**Key Differentiators:**
1. Remembers everything (vs wget amnesia)
2. Set it and trust it (reliability without babysitting)
3. One input, everything handled (end-to-end pipeline)
4. AI-era ready (MCP, LLM outputs, webhooks)

