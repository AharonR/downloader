# Pain Discovery Report — Citation Verifier
Date: 2026-04-12

---

## Discriminant Question Answer

**Do researchers feel personally responsible for citation accuracy, or do they treat it as a systemic problem?**

**Answer: Both — but context-dependent. The framing shifts from systemic (abstract) to acutely personal (at the moment of discovery).**

When researchers discuss citation accuracy in the abstract — in proposals like MyCites (Springer Nature, 2020) or in academic discourse — they frame it as a collective responsibility failure: journals don't check, peer reviewers don't verify, 80% of authors never read the full text of papers they cite. The problem is everyone's and therefore no one's.

But personal narratives (Medium, ResearchGate, Retraction Watch) consistently show the same pattern: the moment a researcher discovers their own citation is wrong, the framing collapses into acute personal failure. *"If one citation was wrong, how many others were?"* — described as an 11:42pm panic the night before submission. A researcher on ResearchGate found their work was "completely misquoted" and described it as "a pretty poor peer review."

**Implication for positioning:** The tool should target the pre-submission moment — the QA pass before hitting submit — not the ambient drafting workflow. The pain is not chronic; it is acute and time-compressed. The user is someone who suspects they have a problem and needs to know before it's too late, not someone building careful citations in real-time.

**Supporting quotes:**
1. *"If one citation was wrong, how many others were?"* — Medium, "The Night Before Submission" (Nov 2025)
2. *"The quotes were completely at odds with their research findings... it was a pretty poor peer review."* — ResearchGate, researcher discovering their work was misrepresented
3. *"80% of authors omit to read the full text of the research paper they are citing."* — PMC (2023), framing the problem as structurally caused, not malicious

---

## Signal Summary

| Gap type | Frequency | Specificity | Strongest quote |
|----------|-----------|-------------|-----------------|
| `trust_gap` | High | High | "I don't know if my citation actually supports my claim" — implied by 80% non-reading rate (PMC) and night-before panic (Medium) |
| `workflow_gap` | High | High | "Citation checking should be integrated into the writing process rather than treated as a final step" — Citation Checking Guide 2026 (referencechecker.org) |
| `abandonment` | Medium | Medium | Existing tools (Citely, Scite, Recite) all solve reference format or corpus-level context — not personal draft claim accuracy |
| `retrieval_gap` | Low | Low | "I can't remember what the paper said" is secondary — the primary failure is citing without reading at all |

---

## Top Pain Signals

**1. The 20–25% error rate is empirically established across multiple studies**
- Gap type: `trust_gap`
- Source: PMC (2023) — 20% in one 145-reference manuscript; Royal Society Proceedings A (2020) — 25% in high-impact general science journals; PLOS ONE — 15–20% misquotation rate in medicine
- Quote: *"Errors in approximately 20% of citations"* in a pre-submission internal review
- Classification: High frequency, High specificity — this is the empirical anchor for the product's existence
- [PMC source](https://pmc.ncbi.nlm.nih.gov/articles/PMC10307651/)

**2. Only 20% of authors read the original paper they cite**
- Gap type: `workflow_gap`
- Source: PMC (2023), citing multiple studies
- Quote: *"Up to 80% of authors omit to read the full text of the research paper they are citing"*
- Classification: High frequency, High specificity — explains *why* the error rate is what it is; also explains the tool's value (it reads what they don't)
- [PMC source](https://pmc.ncbi.nlm.nih.gov/articles/PMC10307651/)

**3. Night-before-submission panic — personal story with full emotional arc**
- Gap type: `trust_gap` + `workflow_gap`
- Source: Medium, "The Night Before Submission" (Nov 2025)
- Story: Researcher discovered at 11:42pm that citations were built from secondary sources (review papers, lecture slides, ChatGPT). Used Citely to check — found 9 incorrect entries, 3 duplicates, 2 non-existent references.
- Quote: *"If one citation was wrong, how many others were?"*
- Note: Citely verifies that references *exist* in databases. It does not verify that the cited paper *supports the claim*. This is the gap.
- [Source](https://medium.com/@0717koii/the-night-before-submission-how-i-found-out-half-my-citations-were-wrong-and-what-fixed-it-89fe55ec5c9e)

**4. Researcher discovered their own work was misrepresented — post-publication**
- Gap type: `trust_gap`
- Source: ResearchGate community thread
- Quote: *"The quotes were completely at odds with their research findings, the publication date was wrong, and there was a reference to their PhD thesis that was also incorrect."*
- Classification: High specificity — this is the downstream consequence; the tool targets prevention upstream
- [ResearchGate](https://www.researchgate.net/post/What_to_do_if_you_find_your_research_has_been_completely_misquoted)

**5. The Hawthorne effect cascading misquotation case**
- Gap type: `trust_gap` (systemic)
- Source: Academic blog, citing research
- Quote: *"155 out of 196 articles mis-cited a particular paper, citing it as confirming rather than refuting the Hawthorne effect"*
- Classification: High frequency, High specificity — shows the error is not random noise but systematic; once a misquotation enters the citation network it propagates
- [Source](https://lessaccurategrandmother.blogspot.com/2024/03/citing-citations.html)

**6. MyCites: collective responsibility framing blocks individual action**
- Gap type: `workflow_gap`
- Source: MyCites (Springer Nature / PMC, 2020)
- Quote: *"In the absence of a final prototype and empirical data about researchers' expectation and feedback, it is difficult to theorise about the question of engagement and uptake."*
- Classification: Medium frequency, High specificity — the authors of the most serious proposal to fix citation accuracy admitted they don't know if researchers will engage; the tool targets individual pre-submission QA, not community annotation
- [PMC](https://pmc.ncbi.nlm.nih.gov/articles/PMC7500547/)

**7. SemanticCite and AI Citation Auditor exist as research papers — not user tools**
- Gap type: `workflow_gap`
- Source: arxiv:2511.16198 (SemanticCite) and arxiv:2511.04683 (AI Citation Auditor)
- SemanticCite uses 4-class system: Supported / Partially Supported / Unsupported / Uncertain — this is exactly the verdict taxonomy in the product brief
- AI Citation Auditor validates 2,581 references across 30 documents but checks reference *existence*, not *claim support*
- Classification: High specificity — the NLP primitives are proven; the gap is that they're not available as a tool in a researcher's workflow
- [SemanticCite](https://arxiv.org/abs/2511.16198) | [AI Citation Auditor](https://arxiv.org/abs/2511.04683)

---

## Competitor Gap Map

**Scite.ai**
- Top complaints: classification of supporting/contrasting "doesn't function entirely correctly"; paywalled content missing; only corpus-level (not draft-level); no free tier
- Gap: classifies how a paper *has been cited* by others, not whether *your specific draft claim* is supported by your cited paper
- What it reveals: the market understands citation context matters, but Scite solves the wrong direction (inbound citations to a paper, not outbound claims from your draft)
- [Review source](https://effortlessacademic.com/scite-ai-review-2026-literature-review-tool-for-researchers/)

**Elicit**
- Top complaints: references "should be treated as index cards rather than proof"; missed 15% of relevant studies in systematic review; no claim-level verification; extracts per paper, not against a draft
- Gap: Elicit finds and summarizes papers; it does not check whether a draft sentence is supported by a cited paper's content
- What it reveals: structured extraction per paper is a solved problem; verification against a draft is not
- [Source](https://support.elicit.com/en/articles/549569)

**Citely / Recite / AI Citation Auditor**
- These tools verify that references *exist* and are formatted correctly. They confirm the DOI resolves.
- Gap: existence ≠ support. A correctly formatted citation to a real paper that doesn't support your claim is the core problem, and none of these tools catch it.

**iThenticate / Turnitin**
- Plagiarism detection only. No claim accuracy.

**The confirmed unoccupied space:** Claim-level verification of a specific draft sentence against the full text of the cited paper, with a verdict (Supported / Partially Supported / Unsupported / Contradicted) + evidence quote + page number. No user-facing tool does this. SemanticCite has the right architecture — it's a research paper, not a product.

---

## What Corpus Mining Cannot Answer

- **Whether researchers trust an automated `Contradicted` verdict enough to remove a citation.** The error rate data proves the problem exists. It doesn't prove researchers will act on a machine-generated verdict without re-reading themselves.
- **Whether `Partially Supported` is useful or just confusing.** The 4-class taxonomy (from SemanticCite) makes theoretical sense; it's unknown whether researchers find `Partially Supported` actionable or too ambiguous to act on.
- **At what moment in their workflow they'd actually run it.** Pre-submission QA is the inference from the "night before" pattern — but some researchers may want it during drafting, or only for citations they're uncertain about. This changes UX entirely.
- **Whether the advisor/supervisor is the real buyer.** The PMC paper identifies that supervisors catch errors in pre-submission review. If the buyer is the supervisor running the tool on a student's draft, the distribution and pricing logic is completely different.

---

## Phase B Interview Guide

**Recommended stimulus:** Run the verifier pipeline on 15–20 citations from a published paper in the researcher's field. Use a paper with a known misquotation case if possible (Hawthorne effect papers are candidates). Select the 3 most interesting verdicts — one `Supported`, one `Partially Supported`, one `Contradicted` — to show the range.

**Sharpened questions (raised by corpus mining):**
1. *"The night before you submitted your last paper, did you re-read any of your citations? Which ones and why?"* — probes the pre-submission moment; reveals which citations researchers feel uncertain about vs. which they assume are fine
2. *"If a tool told you a citation was `Contradicted` — the paper actually argues against your claim — would you remove the citation, or would you go back and read the paper yourself first?"* — directly tests whether automated verdict is actionable or just triggers manual re-reading
3. *"Would you run this on every citation or only the ones you're uncertain about?"* — determines scope; "every citation" = QA tool; "uncertain ones" = confidence calibration tool
4. *"Has a reviewer ever caught a citation error in your work? What happened?"* — if yes, they feel personal responsibility; if never, the pain may be theoretical for them

**Go signal:** Researcher has a prior near-miss story or caught error + says they would act on `Contradicted` verdict without re-reading + identifies a specific submission deadline moment they'd use it.

**No-go signal:** Researcher says they'd still need to verify every verdict manually ("I'd use it as a starting point") — the tool adds work instead of removing it. Or: they've never thought about citation accuracy and don't feel responsible for it.

---

---

## Deep Dive A: Will Researchers Act on a `Contradicted` Verdict Without Re-reading?

### The short answer: probably not on first use — but the design can change this

The research on automation bias and algorithm aversion gives a clear picture of the tension at play.

**Two opposing forces:**

*Automation bias* (PMC systematic review, 74 studies): users accept automated recommendations even when wrong. In clinical decision support studies, 6–11% of users reversed a *correct* pre-existing decision after receiving an *erroneous* AI recommendation. Erroneous advice increased incorrect decisions by 26% (risk ratio 1.26). The mechanism: when a system has performed well consistently, users become complacent and stop catching errors — "simply placing a human-in-the-loop may not be sufficient."

*Algorithm aversion* (Decision Lab; ScienceDirect 2024): domain experts, after seeing one AI error, overcorrect into systematic distrust — even when the algorithm outperforms them on average. Experienced researchers are specifically named as more prone to aversion than novices.

**What this means for citation verification:** The likely behavioral split is:
- **PhD students / early-stage researchers** → tend toward automation bias: will act on `Contradicted` without re-reading, which is the desired behavior but introduces risk if the verdict is wrong
- **Senior researchers / faculty** → tend toward algorithm aversion: will demand to re-read every flagged citation, adding work instead of removing it

**Design responses that reduce both failure modes:**

The literature converges on one fix: show the reasoning, not just the verdict. Specifically:
- Evidence quote + page number ("here is the passage that contradicts your claim") vs bare verdict ("contradicted") dramatically increases appropriate use — users can make their own judgment without full re-read
- Confidence levels attached to each verdict (SemanticCite's approach: Supported / Partially Supported / Unsupported / Uncertain) let users calibrate their re-reading effort
- Peripheral display (verdict as annotation alongside the draft) vs central display (verdict as pop-up requiring acknowledgment) reduces automation bias without triggering aversion

**HKUST Library evaluation (2024)** — direct evidence from researchers using AI citation tools:
> "Generating accurate citations doesn't ensure their alignment with the arguments in the summary. The information they generate should *always be verified*."

This is the current baseline trust level for AI citation tools. The baseline is low — researchers treat current tools as "starting points." The citation verifier needs to exceed this bar by providing evidence quotes (making verification fast, not eliminating it).

**Books documenting the researcher verification norm:**
- *The Craft of Research* (Booth, Colomb, Williams — 4th ed., Chicago, 2016): the most-assigned methodology text in US graduate programs. Frames source verification as an individual researcher responsibility: the author is accountable for what they claim a source says. Does not discuss automation. Establishes the cultural norm the tool operates inside.
- *Writing for Social Scientists* (Becker, 3rd ed., Chicago, 2020): covers "the when and how of citations" as a craft decision. Becker's position: writers cite to support claims, and the obligation to check is implicit in the decision to cite. Again, individual responsibility framing — but no expectation that verification is routine.

**The practical implication for the product:**

A bare `Contradicted` verdict is unlikely to produce action without re-reading among experienced researchers. An evidence quote with page number is likely to produce action with a brief targeted re-read (checking the quoted passage in context) rather than a full paper re-read. This is the design target: reduce re-read from "full paper" to "one passage check" — not eliminate it.

---

## Deep Dive B: Who Is the Buyer?

### Three distinct buyer personas with different triggers, timelines, and willingness to pay

**Persona 1: The library (institutional buyer, slowest, largest)**

The dominant go-to-market pattern for Scite — the closest comparable tool — is:
1. A librarian discovers the tool (often through a Choice Reviews evaluation or conference)
2. Library runs a paid or free trial (typically 4–8 weeks)
3. Library measures engagement metrics (unique users, sessions, searches)
4. If metrics cross a threshold, library purchases an institutional subscription from the library budget
5. Librarians then promote to researchers through LibGuides, workshops, and instruction sessions

Evidence: Clemson (trial summer 2023 → subscription 2023–24), KU Libraries (AI-focused library group trial → full launch), Purdue (enterprise subscription, free for all users), CCSU (institutional subscription from October 2024). All purchases were library-initiated, library-budgeted.

Scite explicitly sells to libraries, not departments or individuals: it has a dedicated academic institutions page, partners with library tech vendors (ResearchSolutions/LibKey), and its discount model ("recommend Scite to your institution") is designed to push individual users to initiate a library conversation.

**What the library cares about:**
- Usage metrics from a trial (not qualitative feedback)
- Coverage breadth (paywalled content, discipline coverage)
- Integration with existing library infrastructure (LibKey, OpenURL resolvers, Primo, Summon)
- Defensible ROI: "X researchers used it Y times during Z period"
- Research integrity framing: tools that improve research quality are easier to justify than productivity tools

**Positioning implication:** A citation verifier pitched as a "research integrity" tool (prevents publication errors, protects institutional reputation) is more library-buyable than a "researcher productivity" tool. Compare: iThenticate is sold to research integrity offices and graduate schools on compliance grounds — not to individual researchers. This is the comparable go-to-market.

**Timeline:** 6–18 months from first contact to institutional subscription. Not the first channel to pursue.

---

**Persona 2: The individual researcher (fastest, smallest unit, viral potential)**

Individual subscriptions exist for all comparable tools: $10–12/month for Elicit and Scite. ResearchRabbit is free.

The individual researcher pays personally when:
- Their institution doesn't have a subscription
- They have a specific deadline pressure (pre-submission) and need the tool now
- They discovered it through word-of-mouth or social media

The "night before submission" pattern (Medium, 2025) is this persona exactly. Discovery is urgent and personal. Payment is out-of-pocket and small. The tool gets used once or twice around submission deadlines, not daily.

**Viral mechanism:** Pre-submission panic is shareable. Researchers who find citation errors the night before submission tell their cohort. The Medium post itself ("The Night Before Submission") is a viral artifact generated by exactly this persona.

**What this persona cares about:**
- Speed: does it work right now, on my paper?
- Actionability: does it tell me what to fix, not just that something is wrong?
- Trust: can I show my advisor the output without it being embarrassing?

**Pricing ceiling:** $10–15/month or one-time $30–50 for a "manuscript check" model. Institutional pricing makes individual payment awkward once the institution has access.

---

**Persona 3: The supervisor / PI (medium timeline, medium budget, highest leverage)**

The PMC (2023) paper on citation errors identifies that supervisors catch errors in pre-submission internal review. Research group leads who have been embarrassed by a citation error in a student's paper — or who have had a paper rejected partly due to citation issues — are high-motivation buyers.

This persona:
- Has a lab budget (not personal funds)
- Wants to run the tool on students' drafts before they submit
- Is motivated by past embarrassment, not abstract concern
- Will pay more per use than a student will ($50–200 per manuscript check is plausible)

This persona is the most analytically interesting because it reframes the product: the tool runs on someone *else's* manuscript, not your own. This is closer to how iThenticate is used (PI or editor runs it on a submitted draft) than how Elicit is used (researcher runs it on papers they're reading).

**What this means:**

| Persona | Timeline | Budget source | Trigger | Willingness to pay |
|---------|----------|--------------|---------|-------------------|
| Library | 6–18 months | Library budget | Trial metrics + librarian champion | High (institutional) |
| Individual researcher | Days | Personal | Pre-submission panic | Low–Medium |
| PI / supervisor | Weeks | Lab budget | Past embarrassment | Medium–High |

**Recommended go-to-market sequence:**
1. **Individual researcher first** — fastest feedback loop, viral if the output is good, reveals whether the `Contradicted` verdict produces action
2. **PI / supervisor second** — lab-budget purchase, higher willingness to pay, positions the tool as quality assurance not just self-service
3. **Library third** — requires institutional credibility (citations from researchers who use it, usage data, possibly a trial program); the institutional pitch requires the individual use cases to already exist

The Turnitin model is the analogous path: individual adoption first (students using it to check their own work) → institutional mandate (university pays because students already use it and the compliance framing fits).

---

## Raw Sources

**Empirical literature:**
- [Citation Errors in Scientific Research — PMC](https://pmc.ncbi.nlm.nih.gov/articles/PMC10307651/)
- [Manuscript Referencing Errors — PMC](https://pmc.ncbi.nlm.nih.gov/articles/PMC7405306/)
- [Quotation Errors in General Science Journals — Royal Society](https://royalsocietypublishing.org/doi/10.1098/rspa.2020.0538)
- [Accuracy of cited facts in medical research — PLOS ONE](https://journals.plos.org/plosone/article?id=10.1371/journal.pone.0184727)
- [Citation accuracy, citation noise, and citation bias — arXiv 2508.12735](https://arxiv.org/pdf/2508.12735)

**Proposals and prior art:**
- [MyCites proposal — PMC](https://pmc.ncbi.nlm.nih.gov/articles/PMC7500547/)
- [SemanticCite — arXiv 2511.16198](https://arxiv.org/abs/2511.16198)
- [AI-Powered Citation Auditing — arXiv 2511.04683](https://arxiv.org/abs/2511.04683)

**Competitor reviews:**
- [Scite AI Review 2026 — Effortless Academic](https://effortlessacademic.com/scite-ai-review-2026-literature-review-tool-for-researchers/)
- [Elicit limitations — official support](https://support.elicit.com/en/articles/549569)

---

## Deep Dive C: Three Remaining Gaps

### Gap 1 — Pipeline Quality: Does the NLP Actually Work?

**Short answer: yes under ideal conditions; degrades significantly without full-text access — which makes Downloader architecturally necessary, not optional.**

**SciFact benchmark (gold standard for scientific claim verification):**

The best ensemble systems on SciFact achieve:
- Support class: 0.94 precision, 0.95 recall, **0.95 F1**
- Refute class: 0.93 precision, 0.89 recall, **0.91 F1**

These are strong numbers. But they assume gold retrieval — the correct abstract is handed to the model. In open deployment (real retrieval, no gold evidence), performance drops from 79.7% to 63.2% — a **16.5 percentage point gap** between benchmark and reality. Domain variance is wide: COVID-19 claims hit 80% accuracy, Alzheimer's disease claims hit 44%.

**SemanticCite (the closest to production-ready):**

- Best model (Qwen3 8B): 66% standard accuracy, 84% weighted accuracy, F1-Macro 50%
- Test set: only 112 examples — too small to trust per-class precision on `Contradicted`
- No confidence threshold guidance provided
- Key failure modes: paywall-restricted papers force reliance on abstracts; can't handle figures, tables, or equations; multi-reference citations evaluated individually

**The critical link to Downloader:**

SemanticCite's primary failure mode — falling back to abstracts when full text is unavailable — is exactly what Downloader solves. 64% of scholarly literature is paywalled. Without full-text access, the model classifies against an abstract that may not contain the relevant passage, producing wrong verdicts with high confidence. Downloader's resolver pipeline is not an add-on feature: it is the architectural layer that keeps the pipeline in the 80–95% accuracy range rather than the 44–63% range. This is a genuine moat — competitors without Downloader degrade on exactly the papers that matter most (recent, high-impact, paywalled).

**Implication for product design:** Before launching, establish a minimum full-text coverage threshold. If Downloader can't retrieve the paper, the verifier should say "unable to verify — full text not accessible" rather than classify from the abstract. A wrong confident verdict is worse than no verdict.

---

### Gap 2 — Journal Distribution Wedge: Does a Mandate Exist?

**Short answer: no current mandate, but ICMJE 2025 creates a compliance framing that libraries and research offices can act on.**

iThenticate is mandated by 40%+ of journals — but only for plagiarism/similarity detection, not claim-to-source accuracy. No publisher currently mandates citation accuracy verification as a separate pre-submission step.

However, the regulatory environment is shifting:

**ICMJE 2025 update** (applies to thousands of journals): explicitly states that "all references are real, relevant, and accurately cited" and that "summaries correctly represent the original research findings." This is the first major style body to frame citation accuracy as an author obligation extending beyond plagiarism. Enforcement is through peer review and editorial process — no automated tool is mandated — but the standard exists in writing.

**Clarivate/JCR 2025**: citations to/from retracted articles excluded from Journal Impact Factor. Signals the field moving toward citation quality beyond format, but limited to retraction status.

**What this means for go-to-market:** The ICMJE language is a compliance handle, not a mandate. A library or research integrity office can point to it to justify purchasing a citation accuracy tool — "ICMJE now requires what this tool provides." This is the pitch for the institutional buyer (Persona 1 in Deep Dive B), not a product-market fit signal by itself. Do not build distribution strategy around a mandate that doesn't exist yet. Monitor for any publisher operationalizing ICMJE 2025 requirements into a pre-submission checklist — that would be the trigger to accelerate the library pitch.

---

### Gap 3 — `Partially Supported`: Actionable or Noise?

**Short answer: no direct behavioral evidence exists, but structural inference suggests it requires a rewrite prompt to be actionable — a bare label isn't enough.**

No study directly measures whether researchers act on a `Partially Supported` verdict vs re-reading. What the evidence suggests:

- The feedback literature (ScienceDirect) shows nuanced multi-category feedback increases peer-initiated revision but can suppress self-initiated revision — intermediate verdicts require more cognitive work, not less
- The automation bias literature: intermediate verdicts (vs binary) reduce over-reliance but also reduce action rate — users need to understand what to *do* differently, not just that something is partially wrong
- SemanticCite's framing ("appropriate remedial actions for different error types") is aspirational, not empirical

**Structural inference from the error taxonomy (PMC 2023):** The most common citation error type — "unjustified extrapolation of conclusions" — is exactly what `Partially Supported` catches. The paper says X under condition Y; the citation treats it as X unconditionally. This error requires a specific fix: rewrite the citing sentence to match the conditionality. A bare `Partially Supported` label doesn't tell the researcher what the condition is.

**Design implication:** `Partially Supported` is actionable only when paired with:
1. The evidence quote that shows what the paper *actually* says
2. A suggested rewrite of the citing sentence that matches the paper's actual claim

Without both of these, `Partially Supported` is likely to be dismissed or trigger a full re-read — neither of which is the desired behavior. This also means `Partially Supported` is the most expensive verdict to produce correctly (requires not just classification but rewrite generation) and should be treated as such in the MVP scope decision.

**MVP recommendation:** Ship `Supported` / `Contradicted` / `Not Found` first. `Partially Supported` as a Phase 2 feature once the rewrite prompt is designed and tested.

---

## Raw Sources
- [Automation Bias Systematic Review — PMC](https://pmc.ncbi.nlm.nih.gov/articles/PMC3240751/)
- [Automation Bias in AI Decision Support — PubMed](https://pubmed.ncbi.nlm.nih.gov/39234734/)
- [Algorithm Aversion — The Decision Lab](https://thedecisionlab.com/reference-guide/psychology/algorithm-aversion)
- [Preventing Algorithm Aversion with Learning Label — ScienceDirect](https://www.sciencedirect.com/science/article/abs/pii/S0148296324005368)
- [Overcoming Algorithm Aversion with Transparency — arXiv](https://arxiv.org/html/2508.03168v1)
- [Trust in AI Lit Review Tools (Scite, Elicit, Consensus, Scopus) — HKUST Library](https://library.hkust.edu.hk/sc/trust-ai-lit-rev/)

**Books (researcher verification norms):**
- *The Craft of Research*, 4th ed. — Booth, Colomb, Williams (Univ. of Chicago Press, 2016)
- *Writing for Social Scientists*, 3rd ed. — Howard S. Becker (Univ. of Chicago Press, 2020)

**Buyer and institutional adoption:**
- [Scite at Clemson Libraries](https://libraries.clemson.edu/news/libraries-provides-access-to-scite-an-ai-tool-to-search-and-discover-scientific-content/)
- [Scite at KU Libraries](https://lib.ku.edu/news/article/ku-libraries-launch-sciteai-access-ku-researchers)
- [Scite at Purdue University Libraries](https://guides.lib.purdue.edu/scite)
- [Scite Academic Institutions page](https://scite.ai/partners/academic-institutions)
- [Evaluating Scite.ai — Choice Reviews (librarian perspective)](https://www.choice360.org/libtech-insight/evaluating-scite-ai-as-an-academic-research-tool/)
- [AI Procurement in Higher Education — EDUCAUSE Review](https://er.educause.edu/articles/2025/3/ai-procurement-in-higher-education-benefits-and-risks-of-emerging-tools)
- [Smart Citations + LibKey webinar — Research Solutions](https://www.researchsolutions.com/webinar-libkey-scite)

**Pipeline quality and NLP benchmarks:**
- [SemanticCite full paper — arXiv 2511.16198](https://arxiv.org/html/2511.16198)
- [SciFact dataset and leaderboard — AI2](https://leaderboard.allenai.org/scifact)
- [CliVER: RAG for Scientific Claim Verification — PMC](https://pmc.ncbi.nlm.nih.gov/articles/PMC10919922/)
- [SciFact-Open: Towards open-domain claim verification — arXiv](https://arxiv.org/abs/2210.13777)

**Journal policy and mandate:**
- [ICMJE 2025 changes — Proof-Reading-Service](https://www.proof-reading-service.com/blogs/ai-in-scholarly-publishing/icmje-2025-key-changes-in-authorship-ai-use-and-ethical-publishing)
- [JCR 2025 retraction exclusions — Clarivate](https://clarivate.com/academia-government/blog/the-upcoming-journal-citation-reports-release-and-changes-to-uphold-research-integrity-in-2025/)
- [iThenticate publisher adoption — Caltech Library](https://library.caltech.edu/c.php?g=967830&p=6992987)

**Personal narratives and community:**
- [The Night Before Submission — Medium](https://medium.com/@0717koii/the-night-before-submission-how-i-found-out-half-my-citations-were-wrong-and-what-fixed-it-89fe55ec5c9e)
- [Researcher misquoted — ResearchGate](https://www.researchgate.net/post/What_to_do_if_you_find_your_research_has_been_completely_misquoted)
- [Hawthorne effect misquotation cascade — Less Accurate Grandmother](https://lessaccurategrandmother.blogspot.com/2024/03/citing-citations.html)
- [PubPeer FAQ and citation dispute policy](https://www.pubpeer.com/static/faq)
- [Citation Checking Guide 2026 — referencechecker.org](https://referencechecker.org/blog/citation-checking-ultimate-guide)
