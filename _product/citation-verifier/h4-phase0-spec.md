# H4 Phase 0 — Citation Verifier

> **Status:** Ready for development  
> **Date:** 2026-04-13  
> **Goal:** Ship a Claude Code skill that verifies draft citations against full-text papers, returning a per-citation verdict with evidence quote and page number.

---

## 1. What It Is

A Claude Code skill that checks whether the papers a researcher has cited actually support the claims being made. Given a draft paragraph and a list of Zotero item IDs, it retrieves full text for each cited paper, extracts the specific claim being attributed to each citation, and classifies the relationship: `supported`, `contradicted`, or `not_found`.

**The gap it fills:** Existing tools (Scite, Elicit, Citely) verify citation format, corpus context, or reference existence. None check whether a specific draft sentence is supported by the full text of the paper cited. SemanticCite (arXiv:2511.16198) has the right architecture but is a research paper, not a tool.

**The moat:** 64% of scholarly literature is paywalled. Without full-text access, classification degrades from 91-95% F1 to 63% (SciFact benchmark, open deployment). Downloader's resolver pipeline (Wiley, ACM, Springer, Elsevier, MDPI) is architecturally necessary — not a fallback. Competitors cannot replicate this without building equivalent infrastructure.

**Distribution:** Claude Code skill (markdown). No server. No UI. Researcher runs it locally.

---

## 2. Scope (Phase 0)

### In scope
- Draft paragraph (pasted text) + Zotero item IDs as input
- Full-text retrieval via Zotero MCP; Downloader CLI as fallback for paywalled papers
- Claim extraction from draft paragraph (which assertion is attributed to each citation)
- Verdict classification: `supported` / `contradicted` / `not_found` / `unverifiable`
- Per-citation report: verdict, confidence, evidence quote, page hint, reasoning
- `unverifiable` gate: if full text is unavailable, return `unverifiable` with reason — never classify from abstract
- Single paragraph per run (MVP scope; batching is Phase 2)

### Out of scope (Phase 0)
- `partially_supported` verdict (requires rewrite prompt — Phase 2)
- Batch processing of entire manuscripts
- Suggested replacement citations or rewrite suggestions
- Integration with word processors (Overleaf, Word)
- GUI / web interface
- Fetching papers by URL alone (input requires Zotero item ID; DOI extracted from metadata)

---

## 3. Dependencies

| Dependency | Role | Risk |
|---|---|---|
| **Zotero MCP** (`54yyyu/zotero-mcp`) | Primary full-text retrieval + metadata + annotations | Single maintainer; must be running locally |
| **Downloader CLI** | Paywalled PDF retrieval fallback | Must be installed and on PATH |
| **Claude Read tool** | PDF text extraction after Downloader saves to disk | Built-in; reliable |
| **Claude API** (Sonnet) | Claim extraction + verdict classification | Standard |
| **Zotero desktop app** | Must be running for MCP server to function | Standard dependency |

**No other dependencies.** Do not add pdftotext, MinerU, or other PDF parsers — the Claude Read tool handles PDF files directly and avoids an extra installation surface.

---

## 4. Architecture

```
User Input (verify.md or inline)
  │  draft paragraph + Zotero item IDs + citation markers
  │
  ▼  STAGE 1: Claim Extraction (one call per citation marker)
  Extract: what specific assertion is attributed to this citation?
  → claim: string (single declarative sentence)
  │
  ▼  STAGE 2: Full-Text Retrieval (per citation)
  Try 1: zotero_get_item_fulltext(item_id)         → full extracted text
  Try 2: zotero_get_annotations(item_id)            → highlights (partial fallback)
  Try 3: Downloader CLI with DOI from item metadata → PDF saved to /tmp/cv_<slug>.pdf
         Claude Read tool reads the saved PDF
  If all fail: verdict = unverifiable, reason = retrieval_failed
  │
  ▼  STAGE 3: Passage Location + Verdict Classification (per citation)
  Given: claim + full text
  Find: most relevant passage; classify relationship
  → verdict: supported | contradicted | not_found
  → confidence: high | medium | low
  → evidence_quote: exact text from paper
  → page_hint: page number or section if determinable
  → reasoning: one sentence
  │
  ▼  STAGE 4: Report Generation
  Structured markdown report, one block per citation
  Printed to stdout or written to verify-report.md
```

---

## 5. Input Format

The user creates a `verify.md` file (or pastes inline). Format:

```markdown
# Citation Verification Request

## Draft paragraph

The intervention reduced anxiety symptoms by 47% compared to placebo [Smith2022],
with effects persisting at 6-month follow-up [Jones2021]. The mechanism is thought
to involve prefrontal cortex regulation of the amygdala [Chen2020].

## Citations

| Marker | Zotero item ID | Label (optional) |
|---|---|---|
| Smith2022 | ZOTERO_ID_1 | Smith et al. 2022 |
| Jones2021 | ZOTERO_ID_2 | Jones & Lee 2021 |
| Chen2020  | ZOTERO_ID_3 | Chen 2020 |
```

**Rules:**
- Markers in the draft paragraph must appear in the table
- Zotero item ID is the 8-character alphanumeric key (e.g., `AB12CD34`) visible in Zotero's "Item Info" panel or URL
- Label is optional but improves report readability
- Multiple citations in one bracket (e.g., `[Smith2022, Jones2021]`) are treated as separate citations each attributed to the same claim span — specify which claim belongs to which in the table's `note` column if they differ

---

## 6. Output Format

Written to `verify-report.md` in the working directory. Also printed to stdout.

```markdown
# Citation Verification Report
Generated: 2026-04-13
Paragraph: "The intervention reduced anxiety symptoms by 47% compared to placebo..."

---

## Smith et al. 2022 [Smith2022]

**Claim in draft:** "The intervention reduced anxiety symptoms by 47% compared to placebo"  
**Verdict:** SUPPORTED ✓  
**Confidence:** High  
**Evidence:** "Participants in the treatment arm showed a 47.3% reduction in GAD-7 scores
relative to placebo (p < 0.001, 95% CI [41.2, 53.4])"  
**Location:** p. 6, Results — Primary Outcome  
**Reasoning:** Paper directly reports the claimed percentage reduction against placebo.

---

## Jones & Lee 2021 [Jones2021]

**Claim in draft:** "Effects persisting at 6-month follow-up"  
**Verdict:** NOT FOUND ✗  
**Confidence:** High  
**Evidence:** "We assessed outcomes at 4-week and 12-week intervals. Long-term follow-up
was beyond the scope of this study."  
**Location:** p. 3, Methods  
**Reasoning:** Paper explicitly states no 6-month follow-up was conducted. The claimed
persistence at 6 months is not in this paper — check if a different citation was intended.

---

## Chen 2020 [Chen2020]

**Verdict:** UNVERIFIABLE  
**Reason:** Full text not accessible. Downloader attempted DOI 10.xxxx/xxxxx — returned
403 (Springer, not in resolver coverage). Cannot classify from abstract.  
**Action:** Retrieve PDF manually and re-run, or check if your institution has access.

---

## Summary

| Citation | Verdict | Confidence |
|---|---|---|
| Smith2022 | SUPPORTED | High |
| Jones2021 | NOT FOUND | High |
| Chen2020 | UNVERIFIABLE | — |

**Coverage:** 2 of 3 citations verified (1 unverifiable — no full text).  
**Action required:** Review Jones2021 (wrong citation or overclaimed finding); retrieve Chen2020 full text.
```

---

## 7. Prompts

### 7.1 Claim Extraction Prompt

Run once per citation marker before retrieval begins.

```
You are analyzing an academic draft paragraph to identify what specific claim
a particular citation is meant to support.

DRAFT PARAGRAPH:
{paragraph}

CITATION MARKER: {marker}
Cited paper: {title} ({authors}, {year})

Task: Identify the single most specific factual or empirical claim in this paragraph
that is directly attributed to the citation marker {marker}. The claim may be a
number, a finding, a mechanism, or a conclusion.

Express it as one declarative sentence. Do not include hedging language from the
paragraph. Extract the core empirical claim as stated.

If multiple sentences are attributed to {marker}, extract the most specific one
(prefer quantitative claims over qualitative).

Output exactly:
CLAIM: [single declarative sentence]
```

### 7.2 Verdict Classification Prompt

Run once per citation after full text is retrieved.

```
You are a fact-checker verifying whether an academic paper supports a specific claim
from a draft manuscript.

CLAIM TO VERIFY:
{claim}

PAPER FULL TEXT:
{full_text}

---

Task: Find the passage in the paper most relevant to this claim and classify the
relationship between the paper's content and the claim.

Verdict definitions:
- `supported`: The paper directly states this claim or presents data that clearly
  supports it AS STATED — including the specific magnitude, direction, and scope
  claimed in the draft. Do not classify as supported if the paper's finding is
  narrower, more hedged, or applies to a different population.
- `contradicted`: The paper presents a finding or argument that directly opposes
  this claim (e.g., different direction of effect, explicit refutation).
- `not_found`: No passage in the paper addresses this claim. Also use `not_found`
  if the paper has a related finding but it is narrower, conditional, or applies
  to a different population than the draft implies — in this case, include the
  paper's actual finding in `evidence_quote` so the researcher can judge.

Rules:
- Base your verdict only on the paper text provided. Do not use prior knowledge.
- If you cannot find a relevant passage after searching the full text, verdict is
  `not_found`.
- Confidence levels:
  - `high`: the evidence is unambiguous (direct quote, exact numbers, explicit
    refutation)
  - `medium`: judgment is required (implied, indirect, or requires inference)
  - `low`: uncertain — only one weak signal found

Output as JSON:
{
  "verdict": "supported" | "contradicted" | "not_found",
  "confidence": "high" | "medium" | "low",
  "evidence_quote": "exact verbatim quote from the paper (1-3 sentences)",
  "page_hint": "page number, section, or figure label — or null if not determinable",
  "reasoning": "one sentence explaining the verdict"
}
```

**Important classifier notes:**
- If the paper says "47%" and the draft says "47%" → `supported`
- If the paper says "47% in healthy volunteers" and the draft says "47% in clinical populations" → `not_found` (scope mismatch; quote the paper's actual population)
- If the paper says "no significant effect" and the draft says "significant effect" → `contradicted`
- If the paper reports a finding at 12 weeks and the draft claims 6-month persistence → `not_found` (include paper's actual timeframe in quote)

---

## 8. Retrieval Logic

Execute per citation in order. Stop at first success.

```
Step 1 — Zotero full text:
  Call zotero_get_item_fulltext(item_id)
  If returns non-empty text of >= 500 words → proceed to Stage 3
  If returns empty or < 500 words → Step 2

Step 2 — Zotero annotations:
  Call zotero_get_annotations(item_id)
  If returns >= 3 annotations → use as partial context; proceed to Stage 3
    (note in report: "classified from annotations only — full text unavailable")
  If returns < 3 annotations → Step 3

Step 3 — Downloader fallback:
  Call zotero_get_item(item_id) to get metadata
  Extract DOI from metadata
  If DOI present:
    Run: downloader "{doi}" --output /tmp/cv_{slug}.pdf
    If PDF saved successfully:
      Read /tmp/cv_{slug}.pdf using Read tool
      Proceed to Stage 3
    If Downloader returns error:
      verdict = unverifiable
      reason = "Downloader: {error message}"
  If no DOI in metadata:
    Try URL field from metadata
    If URL present: downloader "{url}" --output /tmp/cv_{slug}.pdf
    If no URL: verdict = unverifiable, reason = "no DOI or URL in Zotero metadata"
```

**Never classify from abstract.** If the only text available is an abstract (< 500 words with no Methods/Results sections), set `verdict = unverifiable` with `reason = "abstract only — full-text classification would risk wrong verdict"`.

---

## 9. Skill File Structure

**The Citation Verifier ships as its own GitHub repository** (`citation-verifier`), separate from the Downloader repo. Downloader is a prerequisite — a Rust CLI the user installs — not a codebase that gets merged together.

The planning spec (this file) stays in `Downloader/_product/citation-verifier/`. The built skill ships to the new repo.

### Repo: `citation-verifier`

```
citation-verifier/              ← standalone repo (MIT licensed)
  README.md                     ← setup + usage (see below)
  CLAUDE.md                     ← registers the /cite-verify skill
  .claude/
    skills/
      citation-verifier.md      ← skill definition (invoked via /cite-verify)
  verify-template.md            ← input template the user copies and fills in
```

### Prerequisites (documented in README)

1. **Zotero desktop app** — must be running
2. **Zotero MCP server** (`54yyyu/zotero-mcp`) — configured and connected to Claude Code
3. **Downloader CLI** — installed and on PATH (`cargo install --path .` from the Downloader repo, or via a release binary when available)
4. **Claude Code** — with the `citation-verifier` repo open (or `.claude/skills/citation-verifier.md` copied to the user's global Claude Code config)

### `CLAUDE.md` (in citation-verifier repo)

Registers the skill so Claude Code picks it up when the repo is open:

```markdown
# Citation Verifier

## Skills
- /cite-verify: Verify draft citations against full-text papers. Run from a directory
  containing a filled-in `verify.md` file. See verify-template.md for the input format.
```

### `.claude/skills/citation-verifier.md` — Skill Definition

The skill file is the Claude Code skill invoked by the user. It contains:
1. The pipeline instructions (stages 1-4 from Section 4)
2. The two prompt templates (Sections 7.1, 7.2)
3. The retrieval logic (Section 8)
4. The output format template (Section 6)

The skill runs entirely as an agent: it reads `verify.md` from the current directory, calls Zotero MCP tools, optionally calls Downloader via Bash, reads PDFs via the Read tool, runs the LLM prompts, and writes `verify-report.md`.

### Relationship to Downloader repo

```
github.com/you/downloader          ← Rust CLI (PDF resolver infrastructure)
github.com/you/citation-verifier   ← H4 Claude Code skill (depends on Downloader CLI)
github.com/you/research-wiki       ← H13 Claude Code skill (depends on Downloader CLI)
```

The Downloader README should gain a "Built with Downloader" section listing these tools once they ship.

---

## 10. Build Sequence

### Step 1 — Skeleton (Day 1)
- Create new repo `citation-verifier` (MIT license)
- Create `CLAUDE.md` registering the `/cite-verify` skill
- Create `.claude/skills/citation-verifier.md` with pipeline stub
- Create `verify-template.md`
- Verify skill loads in Claude Code without errors

### Step 2 — Zotero retrieval (Day 1-2)
- Test `zotero_get_item_fulltext` on 5 known Zotero items
- Test `zotero_get_annotations` on same items
- Test `zotero_get_item` for metadata/DOI extraction
- Document: which Zotero item types return full text vs not

### Step 3 — Claim extraction (Day 2)
- Run claim extraction prompt on 10 draft sentences (manually constructed)
- Check: does it extract the right claim? Is it appropriately specific?
- Tune prompt if needed; fix edge cases (multi-claim sentences, implicit attribution)

### Step 4 — Verdict classification (Day 2-3)
- Assemble 20 (claim, paper excerpt) pairs: 8 supported, 6 not_found, 4 contradicted, 2 scope_mismatch (expect not_found)
- Run verdict prompt on all 20
- Target: 0 false `supported` on the `contradicted` cases; < 2 wrong on scope_mismatch cases
- Adjust prompt if false positives on scope_mismatch → supported

### Step 5 — Downloader integration (Day 3)
- Test Downloader CLI call from skill via Bash tool
- Test Read tool on saved PDF: verify text extraction quality
- Test on 3 paywalled papers (Wiley, Springer, Elsevier — known resolver coverage)
- Document: what Downloader errors look like; map to `unverifiable` reason strings

### Step 6 — End-to-end test (Day 4)
- Run full pipeline on 3 real papers from a Zotero library
- 15-20 citations total; record: retrieval path taken, verdict, time per citation
- Check: does `unverifiable` gate hold? (No abstract-only classifications)
- Fix any prompt or retrieval issues

### Step 7 — README + release (Day 5)
- Write `skills/citation-verifier/README.md`: prerequisites, setup (Zotero MCP, Downloader), usage
- Write a brief worked example (real paragraph + real output)
- Push to GitHub

---

## 11. Success Criteria (Phase 0)

Evaluated on a 20-citation hand-labelled test set (8 supported, 6 not_found, 4 contradicted, 2 scope-mismatch-as-not_found) across at least 3 different papers.

| Metric | Target | Notes |
|---|---|---|
| False `supported` on `contradicted` cases | 0% | This is the highest-risk error |
| False `supported` on scope-mismatch cases | < 25% (1 of 4) | Scope detection is hardest |
| Overall verdict accuracy | > 80% | Against human labeller |
| `unverifiable` gate holds | 100% | No abstract-only classifications in output |
| `unverifiable` rate (real library) | < 30% | Depends on Downloader coverage; log retrieval path |
| End-to-end latency per citation | < 45s | Retrieval dominates; log per-step timing |
| Skill loads and runs without manual setup beyond prerequisites | Yes | — |

**Phase 0 is complete when:** The skill correctly classifies at least 16 of 20 test citations, produces zero false `supported` verdicts on the 4 `contradicted` cases, and the `unverifiable` gate has no violations.

---

## 12. Phase B Integration

Phase B (stimulus interviews) is pending for H4. The following questions remain open and should be tested with a live verifier output before committing to the distribution plan:

1. **Do researchers act on a `contradicted` verdict without re-reading?** The design response is evidence quote + page number (reduces re-read from "full paper" to "one passage check") — Phase B validates whether this is enough.
2. **Is the supervisor / PI a stronger buyer than the individual researcher?** Changes pricing and distribution if yes.
3. **"As I write" or "one pass before submit"?** Determines whether the MVP is a paragraph-level tool or manuscript-level tool — MVP is paragraph; if "as I write" dominates, invest in Zotero connector or editor plugin instead.
4. **Does evidence quote + page number produce trust in the verdict?** HKUST Library study showed baseline AI trust is low; Phase B tests whether the output design clears the bar.

**Phase B stimulus:** Run the skill on 15-20 citations from a published paper in the interviewee's field. Select 3 results to show: one `supported`, one `contradicted`, one `not_found` with a revealing evidence quote. This is the artifact for the stimulus portion of the interview (see `pain-discovery-report.md` → Phase B Interview Guide).

---

## 13. Open Questions (Technical)

1. **Zotero full-text quality:** Does `zotero_get_item_fulltext` return the actual PDF text or just indexed text? Quality matters for passage location. Test on 10 items before assuming it's reliable.
2. **Long paper handling:** Full text of a 40-page paper may exceed context. May need to chunk by section and run verdict prompt per section, then aggregate. Measure median paper length in the test corpus before deciding whether chunking is needed.
3. **Multi-column PDFs from Downloader:** Academic PDFs often have two-column layouts. The Claude Read tool handles these reasonably but column ordering is sometimes wrong. Flag if evidence quotes contain artifacts.
4. **Annotation-only fallback quality:** If only annotations are available, the verdict is based on what the researcher highlighted — which may miss the relevant passage. Note this in the report; don't suppress it silently.
5. **Zotero item ID format:** Confirm whether item IDs are 8-char keys or full URIs (`zotero://select/library/items/XXXXXXXX`). Accept both; parse accordingly in the skill.
