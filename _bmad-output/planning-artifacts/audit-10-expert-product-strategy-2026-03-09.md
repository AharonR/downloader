---
date: 2026-03-09
author: fierce
status: complete
type: adversarial-audit
inputs:
  - "planning-artifacts/product-brief-Downloader-2026-03-08.md"
  - "planning-artifacts/strategic-roadmap-Downloader-2026-03-08.md"
  - "planning-artifacts/research/market-downloader-future-strategy-research-2026-03-08.md"
panel_size: 10
findings_high: 16
findings_medium: 14
---

# 10-Expert Adversarial Audit: Product Strategy Documents

**Date:** 2026-03-09
**Panel:** 10 domain experts (see roster below)
**Scope:** Product brief, strategic roadmap, and market research — all dated 2026-03-08

---

## Audit Purpose

A structured adversarial review of the three March 8 planning documents to surface gaps, missing rigor, unstated assumptions, and strategic blind spots before the documents become operational strategy. Each expert reviewed independently, then findings were consolidated and de-duplicated.

## Expert Panel Roster

| # | Role | Focus Area |
|---|------|------------|
| 1 | Senior Product Manager | Feature scoping, user stories, prioritization gaps |
| 2 | Research Librarian | Academic workflow accuracy, institutional adoption |
| 3 | Competitive Intelligence Analyst | Competitor coverage, market blind spots |
| 4 | Legal/Compliance Advisor | Publisher ToS, rights, GDPR, institutional risk |
| 5 | Open-Source Sustainability Expert | Business model, funding, community health |
| 6 | Developer Experience (DX) Specialist | CLI UX, API design, onboarding friction |
| 7 | Enterprise Architect | Team workflows, governance, deployment patterns |
| 8 | Academic Power User (PhD Candidate) | Real-world workflow pain, tool-switching costs |
| 9 | AI/ML Engineer | RAG pipeline integration, corpus format expectations |
| 10 | Growth/GTM Strategist | Acquisition channels, positioning, conversion |

---

## Expert Reviews

### Expert 1: Senior Product Manager

**Finding 1 (HIGH) — Corpus definition is absent from the product brief.**
The brief talks about "corpora" and "corpus hygiene" throughout but never defines what a corpus actually is: what files, what metadata format, what directory structure. Users and implementers need a concrete spec.
- Target: product-brief (new "Corpus Definition" section)

**Finding 2 (HIGH) — Input formats are vague.**
"Bibliography-style inputs" is mentioned but never enumerated. Which formats are supported today? Which are planned? Users making adoption decisions need this.
- Target: product-brief (new "Supported Input Formats" section)

**Finding 3 (MEDIUM) — Success metrics are directional, not measurable.**
KPIs say "high and improving" or "low and decreasing" but lack concrete thresholds, measurement methods, or review cadence.
- Target: product-brief (operationalize KPI table)

### Expert 2: Research Librarian

**Finding 4 (HIGH) — Zotero batch capabilities are understated.**
The "Why Existing Solutions Fall Short" section implies reference managers cannot do batch acquisition. Zotero's "Add Items by Identifier" accepts multiple DOIs/ISBNs, its browser connector can capture multiple items, and BibTeX/RIS import is standard. The brief needs honest acknowledgment.
- Target: product-brief (revise "Why Existing Solutions Fall Short", create Zotero benchmark companion)

**Finding 5 (MEDIUM) — Research librarian persona is missing.**
Librarians are mentioned as "credibility brokers" but not elevated to a full persona. They influence departmental tool adoption and have distinct needs (institutional proxy, publisher agreements, batch workflows).
- Target: product-brief (add Lena persona to "Target Users")

**Finding 6 (MEDIUM) — Sidecar format not documented for users.**
The JSON-LD sidecar schema (ScholarlyArticle, PropertyValue DOI, Person authors) exists in code but is invisible to users evaluating the product.
- Target: product-brief (include in "Corpus Definition")

### Expert 3: Competitive Intelligence Analyst

**Finding 7 (HIGH) — Perplexity is missing from competitive landscape.**
Perplexity's Research mode with source citations, PDF upload, and Collections feature represents growing academic usage. It competes for the "find and understand sources" workflow budget.
- Target: market-research (add Perplexity entry)

**Finding 8 (HIGH) — Semantic Scholar / S2 API is missing.**
Free academic paper metadata API, full-text OA access, citation graph, TLDR summaries. Dual role: integration target AND workflow neighbor.
- Target: market-research (add Semantic Scholar entry)

**Finding 9 (HIGH) — No competitive velocity tracking.**
The landscape is a point-in-time snapshot with no refresh cadence. Competitors are shipping AI features monthly.
- Target: market-research (new "Competitive Velocity Assessment"), roadmap (new "Competitive Intelligence Cadence"), create velocity tracker companion

### Expert 4: Legal/Compliance Advisor

**Finding 10 (HIGH) — No legal risk assessment for batch downloading.**
Batch downloading academic papers at scale raises publisher ToS concerns. The product has mitigations (robots.txt, rate limiter) but no documented risk analysis.
- Target: product-brief (new "Legal and Ethical Considerations"), create legal risk assessment companion

**Finding 11 (MEDIUM) — GDPR not addressed for team/enterprise phase.**
Phase 3 team features will handle user data. No mention of privacy considerations.
- Target: legal risk assessment companion

### Expert 5: Open-Source Sustainability Expert

**Finding 12 (HIGH) — No sustainability model.**
The product is open-source with no stated path to sustainability. Academic tools without funding models often stall. Even a brief discussion of options is needed.
- Target: product-brief (new "Sustainability Model" section)

**Finding 13 (MEDIUM) — "Suite never" is too absolute.**
The strategy rule implies suite expansion should never happen. Better to defer it behind decision gates than to rule it out permanently.
- Target: roadmap (modify "Strategy Rules" last rule)

### Expert 6: Developer Experience Specialist

**Finding 14 (MEDIUM) — Success moment lacks concrete UX.**
"A clean, trustworthy corpus" is abstract. What does the user actually see? What output triggers the "aha" moment?
- Target: product-brief (enhance "Success Moment")

**Finding 15 (MEDIUM) — AI-readiness scope is ambiguous.**
Does Downloader provide text extraction? Chunking? Embeddings? Users building RAG pipelines need clarity on where Downloader stops.
- Target: product-brief (new "AI-Readiness Scope" section)

### Expert 7: Enterprise Architect

**Finding 16 (MEDIUM) — Technical moat is not documented.**
The resolver architecture (7 site-specific resolvers with domain-specific URL patterns, API quirks, rate limits) is a genuine technical moat but isn't mentioned in the roadmap.
- Target: roadmap (new "Technical Moat: Resolver Architecture" section)

### Expert 8: Academic Power User

**Finding 17 (HIGH) — No user-facing positioning statement.**
The North Star is internal. Users need a one-line value prop they can understand immediately.
- Target: roadmap (add user-facing positioning after North Star)

**Finding 18 (MEDIUM) — No integration priority matrix.**
Which integrations matter most? In what order? The market research discusses partners but doesn't rank them.
- Target: market-research (new "Integration Priority Matrix")

### Expert 9: AI/ML Engineer

**Finding 19 (HIGH) — Corpus format expectations for RAG are undefined.**
AI engineers want to know: what file formats, what metadata schema, what directory layout? "AI-ready" is meaningless without specifics.
- Target: product-brief (addressed by "Corpus Definition" + "AI-Readiness Scope")

### Expert 10: Growth/GTM Strategist

**Finding 20 (HIGH) — No acquisition plan.**
Phase 1 says "recruit 10-15 design partners" but provides no channel strategy, outreach plan, or messaging. This is the most critical gap for Phase 1 execution.
- Target: roadmap (new "Phase 1: User Acquisition Plan"), create GTM acquisition plan companion

**Finding 21 (MEDIUM) — Content strategy absent.**
No plan for content-driven growth (blog posts, workflow comparisons, tutorials) that academic tools typically need.
- Target: GTM acquisition plan companion

---

## Severity Summary

| Severity | Count | Examples |
|----------|-------|---------|
| HIGH | 16 | Corpus definition absent, input formats vague, Zotero understated, Perplexity/S2 missing, no legal risk assessment, no sustainability model, no acquisition plan, no velocity tracking, no user-facing positioning |
| MEDIUM | 14 | Metrics not measurable, librarian persona missing, sidecar format undocumented, GDPR gap, suite-never too absolute, success moment abstract, AI-readiness ambiguous, moat undocumented, integration priorities unranked, content strategy absent |

---

## Disposition Table

| # | Finding | Severity | Target Document | Action |
|---|---------|----------|----------------|--------|
| 1 | Corpus definition absent | HIGH | product-brief | New section: "Corpus Definition" |
| 2 | Input formats vague | HIGH | product-brief | New section: "Supported Input Formats" |
| 3 | Success metrics directional | MEDIUM | product-brief | Operationalize KPI table |
| 4 | Zotero batch understated | HIGH | product-brief + companion | Revise comparison, create zotero-batch-benchmark |
| 5 | Librarian persona missing | MEDIUM | product-brief | Add Lena persona |
| 6 | Sidecar format undocumented | MEDIUM | product-brief | Include in Corpus Definition |
| 7 | Perplexity missing | HIGH | market-research | Add entry in Key Market Players |
| 8 | Semantic Scholar missing | HIGH | market-research | Add entry in Key Market Players |
| 9 | No competitive velocity tracking | HIGH | market-research + roadmap + companion | New sections + velocity tracker |
| 10 | No legal risk assessment | HIGH | product-brief + companion | New section + legal-risk-assessment |
| 11 | GDPR not addressed | MEDIUM | companion | Include in legal-risk-assessment |
| 12 | No sustainability model | HIGH | product-brief | New section: "Sustainability Model" |
| 13 | Suite-never too absolute | MEDIUM | roadmap | Modify Strategy Rules |
| 14 | Success moment abstract | MEDIUM | product-brief | Enhance Success Moment |
| 15 | AI-readiness ambiguous | MEDIUM | product-brief | New section: "AI-Readiness Scope" |
| 16 | Technical moat undocumented | MEDIUM | roadmap | New section: "Technical Moat" |
| 17 | No user-facing positioning | HIGH | roadmap | Add after North Star |
| 18 | Integration priorities unranked | MEDIUM | market-research | New section: "Integration Priority Matrix" |
| 19 | RAG corpus format undefined | HIGH | product-brief | Addressed by findings 1 + 15 |
| 20 | No acquisition plan | HIGH | roadmap + companion | New section + GTM companion |
| 21 | Content strategy absent | MEDIUM | companion | Include in GTM companion |

---

## Top 5 Actionable Follow-Ups

1. **Define the corpus concretely** (Findings 1, 6, 19) — Add "Corpus Definition" and "AI-Readiness Scope" to the product brief with the actual JSON-LD schema, file naming convention, and directory layout from the codebase.

2. **Create the legal risk assessment** (Findings 10, 11) — Document publisher ToS risks, current mitigations (robots.txt compliance, rate limiting, no paywall circumvention), and recommended additions before Phase 1 user outreach begins.

3. **Build the acquisition playbook** (Findings 20, 21) — The GTM companion doc should be the first operational artifact created from this audit, since Phase 1 depends on recruiting design partners.

4. **Honest Zotero differentiation** (Finding 4) — Run the benchmark test plan from the Zotero companion doc to produce evidence-based rather than assertion-based competitive claims.

5. **Establish competitive monitoring** (Findings 7, 8, 9) — The velocity tracker with quarterly refresh is essential given the pace of AI feature releases across the competitive landscape.

---

## Audit Methodology

- Each expert reviewed all three documents independently
- Findings were submitted with severity ratings and target document recommendations
- Duplicates were merged (e.g., corpus format concerns from experts 1, 6, 8, and 9)
- Disposition was assigned based on most natural document home
- Follow-up priority was ranked by Phase 1 execution dependency
