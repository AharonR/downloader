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

- Active researcher or research-adjacent professional handling 20+ sources per project
- Currently uses a fragmented workflow: browser tabs + Zotero/Mendeley + manual downloading + file renaming
- Feels pain around corpus preparation, not just source discovery
- Willing to provide structured feedback biweekly for 8-12 weeks
- Comfortable with CLI tools or has a technical collaborator who is

### Qualification Criteria

| Criterion | Must-Have | Nice-to-Have |
|-----------|-----------|--------------|
| Processes 20+ sources per project | Yes | |
| Active research project in next 3 months | Yes | |
| Currently uses manual or semi-manual intake | Yes | |
| Comfortable with CLI or has proxy | Yes | |
| Willing to provide biweekly feedback | Yes | |
| Uses Zotero, Mendeley, or similar | | Yes |
| Builds or contributes to RAG/AI pipelines | | Yes |
| Influences tool adoption for a group | | Yes |
| Based in an institution with proxy access | | Yes |

### Target: 10-15 Design Partners

- 6-8 academic power users (PhD candidates, postdocs, research staff)
- 2-3 research-heavy analysts (think tanks, policy groups, consulting)
- 2-3 research librarians or research support staff
- 1-2 AI/ML engineers building research corpora

---

## Channel-by-Channel Strategy

### Channel 1: Academic Social Media (Bluesky, Twitter/X)

**Why:** Researchers publicly share workflow frustrations. Academic Twitter/Bluesky has high density of potential design partners.

**Approach:**
- Search for complaints about corpus preparation, batch downloading, PDF management, citation import friction
- Engage authentically in threads about research workflow pain
- Share concrete examples of Downloader solving specific problems (short demo clips, before/after screenshots)
- Follow and engage with research methodology accounts, open science advocates, digital humanities communities

**Messaging:**
- Lead with the problem, not the product: "Spent 2 hours downloading 40 papers one by one? Here's how I automated it."
- Show output quality: clean directory, sidecar metadata, completion summary
- Never spam; contribute to conversations first

**Volume:** 3-5 genuine engagements per week, 1-2 demo posts per month

### Channel 2: Reddit

**Target subreddits:**
- r/GradSchool — workflow pain is a constant topic
- r/bioinformatics — high-volume literature review is standard
- r/PhD — tool recommendations are actively sought
- r/AcademicPhilosophy, r/AskAcademia — broader academic audience
- r/MachineLearning, r/LocalLLaMA — RAG pipeline builders

**Approach:**
- Answer existing questions about batch downloading, corpus preparation, PDF management
- Post workflow comparisons only when they add genuine value
- Create a dedicated post when ready: "I built a tool for turning messy source lists into research-ready corpora — looking for feedback"

**Messaging:** Problem-first. Show the before (chaotic file list, manual renaming) and after (structured corpus with metadata).

**Volume:** 2-3 helpful comments per week, 1 substantive post per month

### Channel 3: Zotero Forums and Community

**Why:** Zotero users are the closest adjacent audience. They already care about source management and hit Zotero's batch acquisition limits.

**Approach:**
- Monitor Zotero forums for threads about batch import limitations, bulk DOI processing, corpus export
- Position Downloader as complementary: "Use Downloader to acquire, Zotero to manage"
- Contribute to discussions about BibTeX/RIS workflows, bulk identifier processing
- Offer to help solve specific user problems using Downloader

**Messaging:** Complementary, not competitive. "Downloader + Zotero" is the story.

**Volume:** 1-2 forum contributions per week

### Channel 4: Library Science Listservs and Professional Networks

**Why:** Research librarians evaluate tools for departments. Their recommendation carries institutional weight.

**Targets:**
- Code4Lib (code4lib.org) — technically sophisticated library community
- ACRL discussion lists — academic research library professionals
- Library Carpentry / Software Carpentry communities — research computing training
- Digital Humanities Slack/Discord communities

**Approach:**
- Present Downloader as a tool that respects publisher agreements (robots.txt, rate limiting, no paywall circumvention)
- Emphasize provenance, auditability, and institutional compatibility
- Offer structured pilot programs for library staff evaluating research tools

**Messaging:** Trust and compliance first. "Every file traced, every source cited, every limit respected."

**Volume:** 1 listserv post per month, ongoing engagement in threads

### Channel 5: Hacker News

**Why:** High-quality technical audience. Show HN posts drive sustained traffic for developer tools.

**Approach:**
- One well-timed Show HN post when the tool has a polished CLI experience and clear demo
- Focus on the technical architecture story: resolver pipeline, JSON-LD metadata, corpus-as-data
- Be prepared for extensive technical Q&A in comments

**Timing:** After Phase 1 hardening, when the tool can handle the "HN hug of death" use case gracefully

**Volume:** One major post, then ongoing engagement in related threads

### Channel 6: Conference and Workshop Outreach

**Targets:**
- Library technology conferences (Code4Lib, ACRL)
- Research computing workshops (Research Software Engineering conferences)
- Digital humanities events
- Local university research computing brown bags

**Approach:**
- Lightning talks and demos showing real workflow improvement
- Workshop format: "Build a research corpus in 15 minutes"
- Target events with low barrier to submission

**Timing:** Phase 1-2 overlap, after initial design partner feedback is incorporated

---

## Outreach Messaging Templates

### Cold Outreach (Academic Researcher)

> Hi [Name], I saw your thread about [specific pain point]. I'm building an open-source tool called Downloader that turns mixed source lists (URLs, DOIs, BibTeX) into structured research corpora with metadata and provenance tracking. I'm looking for researchers willing to test it on real projects and give feedback. Would you be interested in a 15-minute demo?

### Cold Outreach (Research Librarian)

> Hi [Name], I'm building an open-source evidence acquisition tool designed to complement reference managers like Zotero. It handles batch downloading with robots.txt compliance, per-domain rate limiting, and structured metadata output. I'm recruiting research librarians to evaluate it for institutional use. Would you have 20 minutes for a walkthrough?

### Forum/Community Post

> I built Downloader because I was tired of downloading papers one at a time and losing track of what came from where. It takes a list of URLs, DOIs, or BibTeX entries and gives you back a clean directory with every file named, traced, and ready to cite. Looking for researchers to test it on real projects. [link]

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
