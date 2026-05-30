# H13 Phase 0 — Working Plan

**Source:** brainstorm status memory through 2026-04-13.
**Today:** 2026-05-05.
**Primary operating frame:** llmwiki.app race; ~1-2 weeks remaining of the 4-6 week window opened 2026-04-13.

---

## How to use this document

- L1 → L3 hierarchy. Every leaf is an actionable task with a delegation tag and time estimate.
- Check items off as you go (`- [ ]` → `- [x]`).
- "Open questions" at top must be answered before W1-W2 start; everything else has defaults.
- When delegating to a future Claude Code session, point it at this file plus the referenced spec(s).

---

## Resolved (2026-05-07)

1. **Race frame:** Deferred. Build B1 either way; decide release timing (W7-W8) and iteration cap (T4.6) after B1 is built and quality is visible. Rationale: H17 differentiation persists regardless of whether llmwiki.app adds Zotero, so the race isn't binary.
2. **Annotations:** User has single-digit annotations across own library — cannot self-validate H17. **Strategy = Path A + B in parallel:**
   - Path A: self-annotate 3 papers fresh (2-3h) as a development fixture — see new T2.0a
   - Path B: recruit 1-3 heavy-annotator researchers from network for real validation corpus — see new T2.0b
3. **Repo location:** Subfolder of this Downloader repo, *for now*. Proposed path: `skills/research-wiki-compiler/`. Easy to extract to standalone repo later if open-sourced.

Defaults still in effect:
- Skill-pack model: Claude Code skill (SKILL.md + scripts + prompts), not standalone CLI.
- Time budget: estimates assume ~5 hours/day; halve if fewer.
- "Shipped" for B1 = working demo + README on the chosen test corpus. Public-release decision deferred per resolution 1.
- `ANTHROPIC_API_KEY` assumed available; budget ~$10-30 for 20-30 papers.

## One pre-build market check (5 minutes, before W1)

- [ ] **T0.1** `[H]` **Where do you put marginalia today?** — paper, Obsidian, Hypothes.is, Marginalia, KOReader, none? If your annotation habit lives outside Zotero, H13's Zotero-MCP dependency is leaving signal on the table. Worth knowing now, not after building. Doesn't change B1 build (Zotero is still the easiest single integration point) but informs whether v0.2 should add a second annotation source. *(5m)*

---

## Delegation legend

```
[H]              You only — judgment, taste, external action, strategic decision
[A]              AI agent (Claude Code in future session) — one-shot, low supervision
[A, H drives]    AI does the work; you must drive the loop (prompt tuning, eval reads, iteration)
[H+A]            AI drafts, you review/decide
[A*]             Other AI tool (image, video, voice)
```

`[A]` tasks still require you to start the session, supervise the run, and accept the result. The tag means "no creative judgment needed mid-task" — not "fully autonomous."

---

## L1 — Three blocks (primary, race-frame path)

| Block | Goal | Calendar |
|---|---|---|
| **B1: Build** | Working H13 demo on 20-30 papers, public repo, MIT license | ~5 working days |
| **B2: Validate** | Run on own library, judge quality, decide next step | 2-3 days |
| **B3: Defer** | Lint, H19-on-hypotheses, H18, Phase B interviews | Triggered later |

---

## Alternate frame — race not live (if Q1 = no)

If the window has effectively closed or stopped mattering:

- Drop B1's "ship this week" urgency.
- Still build T3-T5 (ingest + compile + orchestration) on a 5-paper test — validates the architecture without race pressure.
- Promote B3 items: H19-on-hypotheses (1 week), H18 analytics (2 days), lint when wiki ≥50 entries.
- Consider Phase B stimulus interviews if H17 remains ambiguous after V3.
- Pace becomes 4-6 weeks of research+build, not a 2-week sprint.

W1-W6 leaf tasks survive; W7-W8 (release + announcement) become "later" rather than "this week."

---

## B1: Build — workstreams

| ID | Workstream | Depends on |
|---|---|---|
| W1 | Repo + project setup | Q1, Q3 |
| W2 | Zotero library readiness | Q2 |
| W3 | Ingest layer | W1, W2 |
| W4 | Compile layer | W3 |
| W5 | Orchestration layer | W4 |
| W6 | Observability — session-start report | W5 |
| W7 | README + release | W6 |
| W8 | Public surface — demo, announcement | W7 |

Sequence: W1 + W2 in parallel → W3 → W4 → W5 → W6 → W7 → W8.

---

## B1: Build — detailed tasks

### W1 — Repo + project setup

- [x] **T1.1** `[H]` ~~Decide repo location~~ — **resolved:** subfolder `skills/research-wiki-compiler/` of Downloader for now, with symlink at `.claude/skills/` for in-session discoverability.
- [x] **T1.2** `[A]` ~~Initialize subfolder~~ — **done 2026-05-07.** Created `skills/research-wiki-compiler/` with `README.md` (skeleton), `LICENSE` (MIT), `.gitignore` (skill-pack-local).
- [x] **T1.3** `[A]` ~~Scaffold skill pack~~ — **done 2026-05-07.** `SKILL.md` written in canonical Cursor/Anthropic-shared format (frontmatter with `name`, `description`, `disable-model-invocation: true`; body with what/quickstart/required env/reference). Subdirs (`scripts/`, `prompts/`, `examples/`) deferred until they have content.
- [ ] **T1.4** `[H]` Confirm `ANTHROPIC_API_KEY` set in env. *(5m)*
- [ ] **T1.5** `[A]` End-to-end smoke loop: a script that runs ingest → compile → wiki on one paper. Will be filled in by W3-W5; this is the orchestrator skeleton. *(1h)*
- [x] **T1.6** `[A]` ~~Promote specs from memory to repo~~ — **done 2026-05-07.** Copied `spec_compile_prompt.md`, `spec_session_start_report.md`, `spec_lint_phase.md` from memory to `_product/research-wiki-compiler/`. Memory copies left in place. Memory frontmatter (`name`, `description`, `type`, `originSessionId`) stripped — repo files start with the markdown title to match the convention of other files in this directory.
- [x] **T1.7** `[A]` ~~Symlink for skill discoverability~~ — **done 2026-05-07.** Created `.claude/skills/research-wiki-compiler` → `../../skills/research-wiki-compiler` symlink; added `.claude/skills/` to root `.gitignore`. Skill discoverable by Claude Code in this repo after session restart.

### W2 — Zotero library readiness

> Modified per Q2 resolution: you can't self-validate H17 with single-digit annotations. Path A (self-annotate dev fixture) + Path B (recruit annotator-researcher) run in parallel.

- [ ] **T2.0a** `[H]` **Path A — dev fixture.** Pick 3 papers from your library you've recently read or want to read. Annotate genuinely: highlights + at least 1-2 sticky notes per paper. Aim for ~10-15 annotations per paper across types (claim, method, disagreement, question). Time-box to 2-3h. These become the test corpus for T3-T5 development. *(2-3h)*
- [ ] **T2.0b** `[H]` **Path B — recruit external validator.** Identify 3-5 heavy-annotators in your network (academic colleagues, PKM forum acquaintances). Send a short ask: "Would you let me run an experimental Zotero compile tool on your library when v0.1 is ready (~1 week)? In exchange you get the wiki output + a 30-min interview about what you saw." Goal: 1 yes by end of week 1. *(30m send, 0-2 weeks until response)*
- [ ] **T2.1** `[H]` Confirm Zotero Desktop installed and library exists. *(5m)*
- [ ] **T2.2** `[H, guided by A]` Install `54yyyu/zotero-mcp`, register in Claude Code MCP config, verify `zotero_get_recent` returns items. *(30m–1h)*
- [ ] **T2.3** `[H]` Confirm test corpus: the 3 papers from T2.0a with their fresh annotations. (Replaces the original "5-paper corpus with mix.") *(5m)*
- [ ] **T2.4** `[H]` Pick a small cold-start corpus: 5-7 unannotated papers from same library, empirical fields, for testing the cold-start path independently. *(15m)*
- [ ] **T2.5** `[A, H drives]` Verify `zotero_get_annotations` returns expected fields (page, color, text, comment, type) on the T2.0a annotations; catch parsing edge cases before W3. *(30m)*

### W3 — Ingest layer

- [ ] **T3.1** `[A]` Implement `ingest.py` (or bash-driven) calling Zotero MCP, writing `raw/<slug>/{metadata.md, fulltext.txt, annotations.md}` per paper. Slug = author-year-shortitle. *(2h)*
- [ ] **T3.2** `[A]` Hash-tracked `_index.md` manifest: title, doi, zotero_id, fulltext_hash, annotations_hash, last_ingested_at. Reruns skip if all hashes match. *(1h)*
- [ ] **T3.3** `[A]` Page-provenance format in `annotations.md`: `[p.4] (yellow) "quote" — sticky note text`. Match spec format exactly (downstream prompts depend on it). *(30m)*
- [ ] **T3.4** `[A]` Run on 5-paper test corpus. *(15m)*
- [ ] **T3.5** `[H]` Read `raw/<slug>/` for 2-3 papers. Verify quote integrity, page numbers, sticky-note attribution. Adjust ingest if not. *(30m)*

### W4 — Compile layer

> Load-bearing workstream. Spec: `_product/research-wiki-compiler/spec_compile_prompt.md`. Where this fails, the product fails.

- [ ] **T4.1** `[A, H drives]` Draft annotation-grounded compile prompt per spec (5 core constraints, claim extraction with type + confidence, page provenance, "researcher annotations" section). *(1-2h)*
- [ ] **T4.2** `[A, H drives]` Draft cold-start (model-summarized) compile prompt. Same output structure, no annotation input. *(30m–1h)*
- [ ] **T4.3** `[A]` Subagent dispatch: one Sonnet subagent per paper per path; both paths run in parallel when annotations exist. *(1h)*
- [ ] **T4.4** `[A]` Diff between annotation-grounded and cold-start outputs. Surface "researcher caught X, model didn't" and "model surfaced Y, you didn't annotate" — **the diff is the product**. *(2h)*
- [ ] **T4.5** `[A]` Confidence + provenance labels (annotation-grounded vs model-surfaced) on each claim per spec output format. *(30m)*
- [ ] **T4.6** `[A, H drives]` Run on 5-paper corpus. **Iterate:** read output, refine prompts, rerun. Plan ≥3 iterations. **This is where quality is won or lost.** *(4-8h spread over 2-3 sessions)*
- [ ] **T4.7** `[A]` Annotation-only fallback path (no fulltext available). *(1h)*

### W5 — Orchestration layer

- [ ] **T5.1** `[A]` Opus orchestration pass: read raw + compile outputs, decide which entity pages to create/update, dispatch updates. *(2h)*
- [ ] **T5.2** `[A]` `wiki/papers/<slug>.md` writer per spec format. *(1h)*
- [ ] **T5.3** `[A]` `wiki/concepts/<slug>.md` writer per spec format. *(1h)*
- [ ] **T5.4** `[A]` Bidirectional backlinks: paper P claims method M → both `papers/P` and `concepts/M` reference each other. *(1h)*
- [ ] **T5.5** `[A, H drives]` Entity consolidation across papers — same concept from different sources merges into one page. **Per memory: "hardest problem."** Expect heuristic iteration. *(3-5h)*
- [ ] **T5.6** `[A]` `_index.md` and `_sources.md` master files; auto-updated each run. *(1h)*

### W6 — Observability (session-start report)

> Spec: `_product/research-wiki-compiler/spec_session_start_report.md`. **First-class, day-one — not a later feature.** Without it the user goes blind, trust erodes (per pilot learnings).

- [ ] **T6.1** `[A]` `_session.md` generator implementing 7 signals from spec. *(2-3h)*
- [ ] **T6.2** `[A]` `_queue.md`: papers annotated but not compiled; papers needing recompile (annotation hash changed). *(30m)*

### W7 — README + release

- [ ] **T7.1** `[H+A]` README: what it does, why it differs from llm-wiki-compiler / llmwiki.app / NotebookLM, quickstart, requirements. **Lead with annotation-aware compile + diff-as-product** — that's the differentiated framing. *(1-2h)*
- [ ] **T7.2** `[A]` Quickstart commands: install zotero-mcp, set API key, point at Zotero, run. Concrete. *(30m)*
- [ ] **T7.3** `[A* or H+A]` Architecture diagram. ASCII pipeline from spec is sufficient at v0.1. PNG via Mermaid or `bmad:bmm:workflows:create-excalidraw-diagram` is upgrade. *(15m ASCII / 1h diagram)*
- [ ] **T7.4** `[H]` Make repo public; add topics: `claude-code`, `zotero`, `llm`, `research-tools`, `knowledge-management`. *(15m)*
- [ ] **T7.5** `[A]` Tag v0.1.0; release notes = README scope summary. *(15m)*
- [ ] **T7.6** `[A]` `CONTRIBUTING.md`, `CODE_OF_CONDUCT.md` (boilerplate). *(15m)*

### W8 — Public surface

> Separate gate from B1 completion. Repo can be public (T7.4) without you having announced it.

- [ ] **T8.1** `[H or A*]` 60-90s screen recording: Zotero library → command → wiki output → diff highlight. QuickTime, Loom, OBS, or Loom AI for auto-edit. *(1-2h)*
- [ ] **T8.2** `[H+A]` HN Show post draft: title, body (problem + what's different), link. *(30m draft + 30m polish)*
- [ ] **T8.3** `[H+A]` X / Twitter post. Lead with diff-as-product framing. *(30m)*
- [ ] **T8.4** `[H]` Decide announcement timing: same day as repo public vs delay 24-48h. *(5m)*
- [ ] **T8.5** `[H]` Outreach: lucasastorian (llmwiki.app), Karpathy circle, llm-wiki-compiler maintainer. Frame as complementary, not competitive. *(30m–1h)*

---

## B2: Validate — tasks

> Updated per Q2 resolution. Self-validation (V2) is partial — only on T2.0a fixture and the cold-start corpus. **Real H17 validation lives in V3-external** with the recruited annotator.

| ID | Task | Tag | Time |
|---|---|---|---|
| **V1** | Run pipeline on T2.0a fixture (3 annotated) + T2.4 cold-start corpus end to end. | `[A, H drives]` | 1-2h compute |
| **V2** | Read all paper pages. Score 1-5: does each match your reading? Note: H17 evaluation here is weak (you wrote both the annotations and the test). | `[H]` | 2-3h |
| **V3-self** | Side-by-side annotation-grounded vs cold-start on T2.0a papers. **First sanity check** — diff should surface meaningful structural differences, not noise. If even your own annotations produce no signal, prompts are broken. | `[H]` | 1h |
| **V3-external** | **Real H17 validation.** Run pipeline on T2.0b participant's library (with consent). Show them annotation-grounded vs cold-start versions of 5 papers they know well. Ask: "Which feels more like YOUR understanding of this paper?" | `[H]` | 30-min interview + 1h prep/notes |
| **V4** | 1-page memo: what works, what's broken, decision tree — (a) iterate compile, (b) recruit more participants, (c) public release. | `[H+A]` | 1h |
| **V5** | If decision is "more participants": recruit 3-5 more from PKM forums (Zettelkasten.de, r/ObsidianMD, Zotero forum). | `[H]` | spread 1-2 weeks |

---

## B3: Defer — triggers

| ID | Item | Tag | Trigger |
|---|---|---|---|
| **D1** | Lint phase per `spec_lint_phase.md` | `[A, H drives]` | Wiki ≥50 entries OR compile output starts degrading |
| **D2** | H19 stress-test on hypothesis files (apply argument graph to `memory/`) | `[A]` | After B2 ships, before any H19 product work |
| **D3** | H18 git-diff analytics on `memory/entity_*.md` | `[A]` | Anytime; data exists; ~2 days |
| **D4** | H17 Phase B stimulus interviews | `[H]` | Only if V3 says H17 ambiguous |

---

## Risk register

| Risk | Likelihood | Impact | Mitigation |
|---|---|---|---|
| Library has too few annotations for H17 | Q2-dependent | Critical — kills differentiation | Resolve Q2 before W3 |
| Compile prompt iteration burns timeline | High | Medium | Cap at 3 iterations; ship "good enough" if hitting wall |
| Entity consolidation fragments at scale | Medium | Med-High | Ship simple heuristic in B1; flag for D1/lint |
| zotero-mcp install friction | Medium | Low | Document workaround in README |
| llmwiki.app ships Zotero before B1 done | Race-dependent | Medium | Pivot framing fully to annotation-aware compile (H17); still differentiated |
| Cost overrun on Sonnet/Opus | Low | Low | 20-30 papers ≈ low cost; cap if needed |

---

## Calendar (race-frame, ~5 working days)

| Day | Focus | Deliverable |
|---|---|---|
| Day 1 | W1 + W2 | Repo scaffolded, Zotero MCP working, test corpus chosen |
| Day 2 | W3 + start W4 | Ingest works on 5 papers; compile prompts drafted |
| Day 3 | W4 iteration | Compile output passes V3-style read on 3 papers |
| Day 4 | W5 + W6 | Wiki output, backlinks, session report working |
| Day 5 | W7 (+ W8 prep) | README, release, repo public |

B2 in days 6-8. W8 announcement when comfortable, can be later.

---

## What I (Claude Code in this session) can do right now

After Q1-Q3 are answered:

- T1.2, T1.3 — repo scaffolding
- T4.1, T4.2 drafts — compile prompts (independent of Zotero state, can start before W2 done)
- Refine this plan if Q1 = "race not the frame"

If you say "I'm not ready to build yet, refine the plan first," I'll iterate this document instead.
