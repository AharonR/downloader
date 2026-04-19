# Citation Verifier — Research Process

Skill: `_product/skills/pain-discovery.md`

---

## Phase A: Automated Pain Discovery

Run the pain discovery skill with the context below before any interviews or building.

---

### Context

**Pain hypothesis:**
Researchers regularly cite papers for claims the papers do not actually support.
This error is invisible during writing and only surfaces at peer review or post-publication.
Researchers have no tool to check citation accuracy against full text during drafting.

**Target user:**
PhD students and researchers actively writing a paper or dissertation chapter.
Secondary: supervisors and peer reviewers auditing a draft.

**Competitors:**
- Scite.ai — citation context classification at corpus scale; not personal draft audit
- iThenticate / Turnitin — plagiarism detection; not claim accuracy
- Elicit — structured extraction per paper; not claim-level verification against a draft
- SemanticCite — published NLP research; no user-facing tool

---

### Signal Targets

What to look for — classify each collected item using these gap types:

| Gap type | What it looks like for this product |
|----------|-------------------------------------|
| `retrieval_gap` | "I can't remember what the paper actually said" |
| `trust_gap` | "I don't know if my citation actually supports my claim" |
| `abandonment` | "I stopped double-checking citations because it takes too long" |
| `workflow_gap` | "I'd have to re-read every cited paper before submission" |

Strong signal: a researcher describes a specific moment where a citation was wrong and they found out *after* submission or at peer review.

Weak signal: general agreement that "citation errors exist" without personal story.

**Key discriminant question:**
Do researchers feel *responsible* for citation accuracy and experience it as a personal failure when citations are wrong — or do they treat it as a systemic problem they can't solve individually?

If researchers feel responsible → tool targets writing workflow (drafting phase).
If researchers feel it's systemic → tool targets gatekeeping workflow (pre-submission QA or peer review).
This determines the entire positioning and distribution path.

---

### Search Tools

**Forums:**
- r/AskAcademia — search: "citation wrong", "cited incorrectly", "misquoted", "peer review caught"
- r/GradSchool — search: "citation error", "citation checking", "re-read papers"
- r/PhD — search: "citation accuracy", "wrong reference", "cited paper didn't say"
- PubPeer — browse recent comments; these are post-publication citation disputes in the wild
- Zotero Forums — search: "verify citation", "check claim", "citation accuracy"

**GitHub:**
- scite-ai issues — what users wish Scite did that it doesn't
- elicit-ai issues or community threads — claim verification feature requests
- zotero/zotero issues — annotation and claim-checking requests

**Blogs and writeups:**
- Retraction Watch — post-publication citation error case studies (gives concrete error taxonomy)
- Aaron Tay's Substack — librarian perspective on citation accuracy tools
- The 100% CI blog — methodological critique culture; surfaces reasoning gaps
- Search: "citation accuracy study" site:scholar.google.com — empirical error rate literature

---

### Expected Insights

The skill should surface:

1. **Error rate baseline** — do researchers cite empirical evidence of citation error frequency, or is the problem anecdotal? (Simkin & Roychowdhury 2003 is the canonical reference; look for community awareness of it)
2. **Discovery moment** — at what stage do researchers find out a citation is wrong? (During writing, peer review, post-publication)
3. **Workarounds** — what do careful researchers already do? (re-reading, annotation review, asking co-authors)
4. **Trust in automation** — is there any signal that researchers would trust an automated verdict, or is there skepticism about AI claim classification?
5. **Competitor gap** — what does Scite fail to do that users want? (This is the direct evidence of the unmet need)

---

## Phase A Results (completed 2026-04-12)

Report: `pain-discovery-report.md`

**Resolved — do not re-investigate in Phase B:**

| Question | Answer |
|----------|--------|
| Does the pain exist and is it measurable? | Yes — 20–25% error rate across multiple empirical studies; 80% of authors don't read full text they cite |
| When is the error discovered? | Pre-submission panic is the primary moment; peer review and post-publication are the costly moments |
| Positioning: writing tool or QA tool? | Pre-submission QA — the "night before" pattern is the use case |
| Does the NLP pipeline work? | Yes at 91–95% F1 with full text; degrades to 63% without it — Downloader is architecturally necessary |
| Is there a journal mandate? | Not yet — ICMJE 2025 creates compliance framing but no enforced requirement |
| Who is the institutional buyer? | Library, championed by a librarian after a trial; 6–18 month cycle |
| Who is the individual buyer? | PhD student / postdoc pre-submission; $10–15/month ceiling |
| Is `partially_supported` in MVP? | No — requires rewrite prompt to be actionable; Phase 2 |

**Open — Phase B must answer these:**

1. Will researchers act on a `contradicted` verdict without re-reading the full paper? (Automation bias vs algorithm aversion split by expertise level — unknown which dominates for citation use case)
2. Does the evidence quote + page number format reduce the re-read friction enough to produce action?
3. Is the supervisor / PI a stronger buyer than the individual researcher? (Past-embarrassment trigger vs pre-submission panic trigger)
4. When exactly in the workflow would they run it — "as I write each section" or "one pass before I submit"? (Changes UX architecture)

---

## Phase B: Human Validation (3–4 interviews)

Phase B is stimulus-only. Do not ask "do you have this problem?" — Phase A answered that.
Use the time entirely for the three things corpus mining cannot answer.

**Prepare before interviews:**
Run the verifier pipeline manually on 15–20 citations from a real paper (ask a researcher to share a submitted paper, or use a published paper in a known field). Record: how many citations are `partially_supported`, `not_found`, or `contradicted`? Identify the 2–3 most surprising cases. These become the stimulus.

**Who to interview:**
Researchers who have had a citation disputed by a reviewer or have caught a citation error in someone else's work. These are the people who feel the pain acutely.

**Interview agenda (30 minutes):**

*10 min — one story:*
- "Tell me about a time a citation was wrong — yours or someone else's. How did you find out?"
- Let them talk. Do not interrupt. The story reveals the discovery moment and the emotional weight.

*10 min — stimulus:*
- Show the verifier output on 5 citations from their field (or their paper if they shared it)
- "What's your first reaction to this verdict?"
- "If this were available before submission, what would you have done with it?"
- "Would you trust a `contradicted` verdict enough to remove the citation?"

*10 min — workflow fit:*
- "When would you want to run this — during drafting or before submission?"
- "Would you run it on every citation or only the ones you're uncertain about?"

**Go signal:** Researcher has a specific story + trusts the `contradicted` verdict + names a specific moment in their workflow where they'd use it.

**No-go signal:** Researcher agrees citations can be wrong but doesn't feel it's their responsibility to check, or dismisses the verdict ("I'd still need to verify manually").
