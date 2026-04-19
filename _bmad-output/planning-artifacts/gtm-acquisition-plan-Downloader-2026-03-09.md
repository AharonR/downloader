---
date: 2026-03-09
author: fierce
status: draft
type: companion
parent: strategic-roadmap-Downloader-2026-03-08.md
audit: audit-10-expert-product-strategy-2026-03-09.md
findings_addressed: [20, 21]
---

# Go-To-Market Acquisition Plan: Downloader

**Date:** 2026-03-09
**Purpose:** Operational playbook for Phase 1 design partner recruitment and early user acquisition.
**Audit findings addressed:** #20 (no acquisition plan), #21 (content strategy absent)

---

## Design Partner Profile

### Ideal Design Partner

- Engineer or researcher who processes large source lists programmatically or needs pipeline-ready corpus output
- Feels pain around bulk acquisition, messy mixed-format input, or automation — not just one-at-a-time downloading
- Does not have an entrenched Zotero workflow that already covers their need
- Comfortable with CLI tools and building from source
- Willing to provide structured feedback biweekly for 8-12 weeks

### Qualification Criteria

| Criterion | Must-Have | Nice-to-Have |
|-----------|-----------|--------------|
| Processes 20+ sources per project | Yes | |
| Active project in next 3 months | Yes | |
| Needs programmatic / automated acquisition OR messy input handling | Yes | |
| Comfortable with CLI and building from source | Yes | |
| Willing to provide biweekly feedback | Yes | |
| Builds or contributes to RAG/AI pipelines | | Yes |
| Feeds corpus into downstream tools (NotebookLM, vector DB, analysis scripts) | | Yes |
| Influences tool adoption for a team or lab | | Yes |
| High-volume recurring literature pulls (not one-off projects) | | Yes |

### Target: 10-15 Design Partners

**Revised priority (updated 2026-03-29):** AI/ML engineers and computational researchers are Tier 1 because Downloader's real edge — CLI, pipeline-ready output, messy input handling, headless operation — maps directly to their workflow. General academic power users with existing Zotero workflows are deprioritized; Zotero covers most of their need.

- **Tier 1 (primary, 6-8 partners):**
  - 3-4 AI/ML engineers building RAG corpora or research pipelines
  - 3-4 computational / bioinformatics researchers with high-volume literature review needs
- **Tier 2 (secondary, 3-4 partners):**
  - 3-4 research analysts at think tanks, policy orgs, or consulting firms running recurring bulk pulls
- **Tier 3 (deprioritize for now):**
  - General PhD students / postdocs with standard Zotero workflows — insufficient differentiation
  - Research librarians — long sales cycle, institutional inertia, Zotero deeply entrenched

---

## Channel-by-Channel Strategy

> **Updated 2026-03-29:** Channel priority revised to match Tier 1/2 segment focus. Channels 1-2 are primary. Channels 3-4 deprioritized until binary releases are available (build-from-source is not a blocker for Tier 1 but is for librarians and general academics).

### Channel 1: Reddit — Tier 1 subreddits (PRIMARY)

**Target subreddits (priority order):**
- r/MachineLearning — RAG pipeline builders, corpus preparation is a known pain
- r/LocalLLaMA — engineers building local AI stacks who need document ingestion
- r/bioinformatics — high-volume literature review is standard, CLI-comfortable audience
- r/PhD, r/GradSchool — secondary; only engage threads with explicit automation or bulk-download pain
- r/AskAcademia — broad fallback

**Approach:**
- Answer existing questions about batch downloading, corpus preparation, bulk DOI processing, PDF ingestion for RAG
- Lead with the pipeline/automation angle: "here's how to go from a list of DOIs to a vector-DB-ready corpus"
- Post a dedicated thread when ready: "I built a CLI tool for bulk paper acquisition and corpus prep — RAG-ready output, provenance sidecars, handles DOIs/arXiv/PMC/URLs in one pass"

**Messaging:** Pipeline-first. Show the output structure: clean directory, JSON-LD sidecars, BibTeX export, completion summary. "Paste your source list, get a corpus."

**Volume:** 3-5 helpful comments per week in Tier 1 subreddits; 1 substantive post per month

### Channel 2: Academic Social Media (Bluesky, Twitter/X) (PRIMARY)

**Why:** AI/ML researchers and computational scientists are active on Bluesky and Twitter/X. Workflow frustration posts are common.

**Approach:**
- Search for complaints about bulk PDF acquisition, RAG corpus prep, batch DOI processing
- Engage authentically; demonstrate the tool solving a specific stated problem
- Share short demos: terminal recording of a 20-DOI batch resolving to a clean corpus

**Messaging:**
- Lead with the pipeline use case: "Turn a BibTeX file into a RAG-ready corpus in one command"
- Show provenance output — JSON-LD sidecars, completion summary, failure detail
- Never spam; contribute to conversations first

**Volume:** 3-5 genuine engagements per week; 1-2 demo posts per month

### Channel 3: Hacker News (SECONDARY — timing-dependent)

**Why:** High-quality technical audience; strong overlap with AI engineers and tool builders.

**Approach:**
- One well-timed Show HN post focused on the technical story: resolver pipeline, JSON-LD metadata, corpus-as-data, headless/scriptable design
- Be prepared for extensive technical Q&A

**Timing:** After at least 3 Tier 1 design partners have validated the workflow. Do not rush this.

**Volume:** One major post, then ongoing engagement in related threads

### Channel 4: Zotero Forums (SECONDARY — complementary framing only)

**Why:** Some Zotero power users hit acquisition limits and look for complementary tools.

**Approach:**
- Only engage threads where users have an explicit need Zotero doesn't cover (headless, automation, messy input, pipeline output)
- Position as complementary: "Downloader acquires and structures, Zotero manages"
- Do not position as a replacement or compete on Zotero's strengths

**Messaging:** Complementary, not competitive. Honest about where Zotero already covers the need.

**Volume:** 1 forum contribution per month; do not over-invest here

### Channel 5: Library Science Listservs (DEPRIORITIZED — revisit after binary release)

**Status:** Deprioritized until a binary release removes the build-from-source barrier. Research librarians are not the right early adopter for a source-build tool.

**Revisit when:** macOS binary or `cargo install` path is available.

### Channel 6: Conference and Workshop Outreach (PHASE 1-2 OVERLAP)

**Targets:**
- Research Software Engineering conferences — strongest overlap with Tier 1 segment
- Local university research computing brown bags
- Library technology conferences (Code4Lib, ACRL) — lower priority, revisit Phase 2

**Approach:**
- Lightning talks and demos showing real workflow improvement
- Workshop format: "Build a research corpus in 15 minutes"
- Target events with low barrier to submission

**Timing:** Phase 1-2 overlap, after initial design partner feedback is incorporated

---

## Outreach Messaging Templates

### Cold Outreach (AI/ML Engineer — Tier 1 PRIMARY)

> Hi [Name], I saw your post about [RAG pipeline / corpus prep pain point]. I'm building an open-source CLI tool called Downloader — you give it a list of DOIs, arXiv IDs, PMC IDs, URLs, or a BibTeX file, and it gives you back a structured directory with every file named, a JSON-LD provenance sidecar per paper, and a completion report showing what succeeded and what failed. Built for headless/automated use. Looking for engineers to test it on real corpus-building workflows. Want to take a look?

### Cold Outreach (Computational Researcher — Tier 1 PRIMARY)

> Hi [Name], I saw your thread about [bulk literature acquisition / batch DOI processing]. I'm building an open-source CLI tool for turning mixed source lists into research-ready corpora — handles DOIs, arXiv IDs, PMC IDs, URLs, and BibTeX files in one pass, with open-access resolution before hitting paywalls. Looking for researchers running high-volume literature pulls to test it and give feedback. Would you be interested?

### Cold Outreach (Research Analyst — Tier 2)

> Hi [Name], I'm building an open-source CLI tool that turns mixed source lists (URLs, DOIs, BibTeX) into structured corpora with metadata and provenance tracking — designed for recurring bulk pulls and pipeline integration. Looking for analysts who regularly process large source sets to test it on real projects. Would you have 15 minutes for a quick demo?

### Cold Outreach (Research Librarian — DEPRIORITIZED, revisit after binary release)

> Hi [Name], I'm building an open-source evidence acquisition tool designed to complement reference managers like Zotero. It handles batch downloading with robots.txt compliance, per-domain rate limiting, and structured metadata output. I'm recruiting research librarians to evaluate it for institutional use. Would you have 20 minutes for a walkthrough?

### Forum/Community Post (Tier 1 — pipeline angle)

> I built a CLI tool for bulk paper acquisition and corpus preparation. You give it a list of DOIs, arXiv IDs, PMC IDs, or URLs (or a BibTeX/RIS file), and it resolves open-access versions first, downloads everything, names files consistently, and writes a JSON-LD provenance sidecar per paper. Completion report tells you exactly what succeeded and what failed. Designed to be scriptable and pipeline-friendly — no GUI required. Looking for engineers and researchers building corpora to test it. [link]

---

## Engagement Cadence

### Design Partner Program (8-12 weeks)

| Week | Activity |
|------|----------|
| 0 | Onboarding call: install, first run on a real source list |
| 1 | Async check-in: did the first corpus work? What broke? |
| 2 | Biweekly call: workflow review, pain points, feature requests |
| 4 | Biweekly call: second project usage, integration feedback |
| 6 | Biweekly call: retention check — still using? Why/why not? |
| 8 | Biweekly call: benchmark comparison, competitive feedback |
| 10 | Final review: would you recommend? What's missing? |
| 12 | Case study interview (optional): public or internal reference |

### Ongoing (post-program)

- Monthly email update with changelog highlights
- Quarterly "what's next" preview for feedback
- Slack/Discord channel for design partner community

---

## Phase 2: Content Strategy

### Content Types (prioritized)

1. **Workflow comparison posts**: "How I replaced manual PDF downloading with a 5-minute CLI workflow"
2. **Corpus preparation tutorials**: "Building a literature review corpus from 50 mixed sources"
3. **Resolver deep-dives**: "How Downloader handles arXiv, PubMed, and paywalled sources differently"
4. **Integration guides**: "Feeding a Downloader corpus into Zotero / NotebookLM / your RAG pipeline"
5. **Benchmark results**: "Downloader vs manual intake: time, completeness, and metadata quality"

### Publishing Cadence

- 2 posts per month starting Phase 2
- Cross-post to relevant subreddits and social media
- Encourage design partners to share their own workflows

### Distribution

- Project blog / GitHub Discussions
- Academic social media (Bluesky, Twitter/X)
- Reddit (relevant subreddits)
- Hacker News (for technical deep-dives)

---

## Phase 3: Scaling Distribution

### Word-of-Mouth Mechanics

- Design partners become advocates when they share the tool with labmates
- "Share your corpus setup" feature: exportable workflow configs that others can replicate
- Public case studies with real project examples (with permission)
- Conference workshop attendees become multipliers

### Integration-Driven Distribution

- Zotero export compatibility drives adoption from Zotero power users
- BibTeX/RIS import drives adoption from users with existing bibliographies
- RAG pipeline guides drive adoption from AI engineers
- Each integration creates a new inbound channel from the partner ecosystem

---

## Metrics by Phase

### Phase 1 Metrics (Months 1-3)

| Metric | Target | Measurement |
|--------|--------|-------------|
| Design partners recruited | 10-15 | Sign-up count |
| Partners completing onboarding | ≥ 80% | First successful corpus creation |
| Partners using on 2+ projects | ≥ 60% | Self-reported or observed |
| Biweekly feedback sessions completed | ≥ 70% of scheduled | Calendar tracking |
| Net Promoter Score (informal) | ≥ 7/10 | Exit survey |

### Phase 2 Metrics (Months 4-6)

| Metric | Target | Measurement |
|--------|--------|-------------|
| Monthly active users (GitHub + direct) | 50-100 | Download/clone stats |
| Content pieces published | 8-12 | Publication count |
| Inbound interest from content | 10+ conversations | DM/email/issue tracking |
| GitHub stars | 200+ | GitHub |
| Repeat users (2+ projects) | ≥ 40% of actives | Self-reported or telemetry |

### Phase 3 Metrics (Months 7-12)

| Metric | Target | Measurement |
|--------|--------|-------------|
| Monthly active users | 200-500 | Download/usage stats |
| Team pilots initiated | 3-5 | Outreach tracking |
| Integration-driven signups | 20%+ of new users | Referral source tracking |
| Community contributors | 5-10 | PR/issue participation |
