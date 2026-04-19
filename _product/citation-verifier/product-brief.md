# Citation Verifier — Product Brief

**Hypothesis:** H4  
**Layer:** Independent (no prerequisites)  
**Build estimate:** 2 weeks  
**Pain:** 5/5 | **Effort:** 2/5 | **Competition:** 3/5

---

## Problem

Citation errors — papers cited for claims they don't actually support — are endemic in academic writing.
Manual fact-checking is slow and rarely done. The error is invisible until peer review or post-publication.

Specific failure modes:
- Author cites a paper for a claim that is only implied, not stated
- Author misremembers a finding and cites the paper for the wrong result
- A cited paper's claim is hedged or conditional; the draft treats it as absolute
- The cited paper contradicts the draft claim

---

## Solution

Given a draft paragraph and a Zotero library, verify each citation against the full text of the cited paper.
Return a per-citation verdict with evidence quote and page number.

**Verdict types (MVP — 3-class):**
- `supported` — full text directly supports the draft claim
- `not_found` — no matching claim found in the paper
- `contradicted` — the paper's actual finding opposes the draft claim

**Phase 2 verdict (not in MVP):**
- `partially_supported` — finding exists but hedged, conditional, or narrower than cited

`Partially supported` catches the most common error type (unjustified extrapolation) but a bare label isn't actionable without a suggested rewrite of the citing sentence. That rewrite requires additional generation work and user testing. Ship the 3-class system first; add `partially_supported` once the rewrite prompt is designed and validated.

---

## Target User

PhD students and researchers actively writing a paper or dissertation.
Secondary: supervisors and peer reviewers who want to audit a draft.

---

## MVP Scope

1. Input: draft paragraph (pasted text) + Zotero item IDs for cited papers
2. Full-text retrieval via Zotero MCP (`zotero_get_item_fulltext`); Downloader resolver for paywalled PDFs
3. **If full text unavailable: return `unverifiable` — do not classify from abstract** (abstract-only classification degrades accuracy by ~16pp; a confident wrong verdict is worse than no verdict)
4. Claim extraction from draft: identify the specific assertion attributed to each citation
5. Full-text search + LLM judgment: locate the most relevant passage; classify verdict
6. Output: structured report — verdict, evidence quote, page number, confidence score

**What is explicitly out of scope for MVP:**
- `partially_supported` verdict (Phase 2 — requires rewrite prompt)
- Batch processing of entire manuscripts
- Suggested replacements or alternative citations
- Integration with word processors

**Distribution:** Claude Code skill (markdown + CLAUDE.md). No server. No UI.

---

## Competitive Landscape

- **SemanticCite** (arXiv:2511.16198) — the closest prior art; 4-class verdict system; 66% standard accuracy / 84% weighted accuracy on a 112-example test set; not a user-facing tool; primary failure mode is paywalled papers (falls back to abstract, degrades sharply)
- **Elicit** — structured extraction per paper; not claim-level verification against a draft; references "should be treated as index cards rather than proof"
- **Scite.ai** — classifies how a paper *has been cited by others*; not whether *your draft claim* is supported by *your cited paper*; classification reliability issues acknowledged
- **iThenticate / Turnitin** — plagiarism/similarity only; no claim accuracy
- **Citely / AI Citation Auditor** — verify references *exist* in databases; do not verify claim-to-source support

The specific combination (personal draft + full-text access including paywalled papers + per-citation verdict with evidence quote) is unoccupied.

---

## Compliance Framing

**ICMJE 2025** (applies to thousands of journals): authors must ensure "all references are real, relevant, and accurately cited" and that "summaries correctly represent the original research findings." This is the first major style body to put claim-to-source accuracy in writing as an author obligation.

No automated tool is yet mandated. But this language gives libraries and research integrity offices a compliance handle: the tool provides what ICMJE now requires. Use this framing in institutional pitches; do not build distribution around a mandate that doesn't exist yet.

**Clarivate/JCR 2025**: citations to retracted articles excluded from Impact Factor — signals the broader field moving toward citation quality, though limited to retraction status.

---

## Where Downloader Fits — The Moat

Downloader is architecturally necessary, not a nice-to-have fallback.

64% of scholarly literature is paywalled. When a citation verifier cannot access the full text, it falls back to classifying from the abstract. Benchmark data (CliVER, PMC 2024) shows this degrades accuracy by ~16.5 percentage points in open deployment — and variance across domains is severe (44% on some fields vs 80% on others).

A confident wrong verdict — `contradicted` when the paper actually supports the claim — is worse than no verdict. Without full-text access, the tool produces exactly these errors on the papers that are hardest to check manually.

**The competitive consequence:** Any citation verifier built without a paywalled PDF retrieval layer will degrade on exactly the papers that matter most. Competitors cannot replicate Downloader's resolver pipeline (Wiley, ACM, Springer, Elsevier coverage) without building equivalent infrastructure. This is the moat.

**The `unverifiable` gate:** If Downloader cannot retrieve the full text, the verifier returns `unverifiable` with the reason. No classification. This protects the tool's reliability signal — users learn to trust verdicts when they appear, because false verdicts don't appear.
