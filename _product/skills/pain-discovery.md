# Skill: Pain Discovery (Automated Phase A Research)

## Purpose

Mine public forums, GitHub, and blog content to validate a product pain hypothesis before
any user interviews or building. Produces a structured pain signal report and a sharpened
set of questions for human validation (Phase B).

This skill replaces the "do you have this problem?" portion of user interviews.
It does not replace stimulus-based validation or trust calibration — those require a live artifact.

---

## How to Invoke

Point the skill at a `research-process.md` file. The file defines:
- The pain hypothesis to test
- The target user
- The competitors to audit
- The search channels (subreddits, GitHub repos, forums, blogs)
- The signal taxonomy (what counts as evidence of each gap type)
- The key discriminant question the skill must answer

```
Run the pain discovery skill using the context defined in
_product/[product-folder]/research-process.md
```

---

## Execution Steps

### Step 1 — Load context

Read the research process file. Extract:
- `pain_hypothesis` — the specific claim being tested
- `target_user` — who experiences the pain
- `competitors` — tools that partially address the pain
- `search_channels` — where to look
- `signal_taxonomy` — how to classify what you find
- `discriminant_question` — the one question this run must answer

### Step 2 — Forum and community search

For each channel in `search_channels.forums`:
- Search for the pain hypothesis keywords + target user keywords
- Collect: thread title, post excerpt (the complaint or question), URL, upvote/engagement signal if visible
- Filter: only posts where a specific workflow breakdown is described (not just "X tool sucks")
- Target: 15–25 posts per channel, prioritise posts with replies (community validation)

For each channel in `search_channels.github`:
- Search open and closed issues for pain keywords
- Collect: issue title, body excerpt, comment count, label
- Filter: feature requests and bug reports that reveal workflow assumptions
- Note: closed issues marked "won't fix" or "by design" are especially informative

For each channel in `search_channels.blogs`:
- Search for workflow writeups, tool comparisons, and abandonment post-mortems
- Collect: post title, relevant paragraph, URL, publication date
- Weight: posts from the last 18 months more heavily; older posts useful for persistence signal

### Step 3 — Classify signals

For each collected item, classify using the signal taxonomy defined in the research process.
Default taxonomy if not overridden:

| Gap type | Definition | Example |
|----------|-----------|---------|
| `retrieval_gap` | Can't find something that exists | "I know I read this but can't find it" |
| `synthesis_gap` | Can't connect across sources | "I have 200 papers and can't see patterns" |
| `reasoning_gap` | Can't evaluate logic or trace evidence | "I don't know what the counterargument is" |
| `trust_gap` | Doesn't trust AI output enough to use it | "AI summaries are plausible but wrong" |
| `workflow_gap` | Tool doesn't fit into existing process | "I'd have to change too much to use this" |
| `abandonment` | Tried a tool and stopped | "I used X for 2 months then gave up because..." |

Each item gets one primary classification and an optional secondary.

### Step 4 — Competitor audit

For each competitor in the competitor list:
- Search for: "[competitor] sucks", "[competitor] limitation", "[competitor] missing", "[competitor] vs"
- Collect the top complaints and feature gaps
- Note: complaints about competitors are direct evidence of unmet pain the product could claim

### Step 5 — Frequency and specificity scoring

Two dimensions matter:

**Frequency:** How many independent sources mention this pain? (not retweets of the same post)
- High: 10+ independent sources
- Medium: 4–9
- Low: 1–3

**Specificity:** How concrete is the description?
- High: describes a specific workflow step that fails ("when I go to cite a paper I read 6 months ago, I have to re-read the whole thing")
- Medium: names the pain but not the moment ("I can never synthesize across my library")
- Low: general frustration without workflow detail ("research tools are bad")

Only High/High and High/Medium signals should drive build decisions.

### Step 6 — Answer the discriminant question

The research process defines one question the skill must answer before handing off to Phase B.
Construct a direct answer: yes/no/unclear, with the 3 strongest supporting quotes.

If the answer is "unclear", flag what additional search would resolve it.

### Step 7 — Generate Phase B interview guide

Based on what corpus mining cannot answer (stimulus reaction, trust calibration, workflow fit),
produce:
- 3–5 sharpened interview questions specific to this product
- The recommended stimulus to show (what prototype output to prepare before interviews)
- The specific moment in the interview where the stimulus should be introduced
- The 2 signals that would constitute a "go" vs "no-go" from the human phase

---

## Output Format

```markdown
# Pain Discovery Report — [Product Name]
Date: [YYYY-MM-DD]

## Discriminant Question Answer
[Yes / No / Unclear] — [one paragraph with the 3 strongest supporting quotes and URLs]

## Signal Summary

| Gap type | Frequency | Specificity | Strongest quote |
|----------|-----------|-------------|-----------------|
| retrieval_gap | High | Medium | "..." (source) |
| synthesis_gap | Medium | High | "..." (source) |
| ... | | | |

## Top Pain Signals (5–8 items)
For each: gap type, quote, source URL, classification rationale

## Competitor Gap Map
For each competitor: top 3 complaints; what they reveal about unmet pain

## What Corpus Mining Cannot Answer
[Bulleted list — these become the Phase B agenda]

## Phase B Interview Guide
- Recommended stimulus: [what to build/prepare]
- Sharpened questions: [3–5 questions that corpus mining raised but couldn't answer]
- Go signal: [what a positive human validation looks like]
- No-go signal: [what would cause a pivot]

## Raw Sources
[Full list of URLs collected, grouped by channel]
```

---

## Limitations

This skill answers whether a pain exists and how it's described in public.
It does not answer:
- Whether your specific artifact crosses the trust threshold
- Whether the output format fits into the researcher's workflow
- Whether the "surprise property" holds (argument graph producing unexpected connections)
- Whether researchers will pay for or recommend the tool

Those questions belong to Phase B. Do not use corpus mining output to skip Phase B entirely.
