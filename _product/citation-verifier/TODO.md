# Citation Verifier — Build TODO

> Phase 0 implementation checklist. Work top to bottom; tasks marked with
> dependencies cannot start until their prerequisites are checked off.
> Full spec: `h4-phase0-spec.md`

---

## #1 — Create repo skeleton
> No prerequisites.

- [ ] Create new GitHub repo `citation-verifier` (MIT license)
- [ ] Add `CLAUDE.md` registering the `/cite-verify` skill
- [ ] Add `.claude/skills/citation-verifier.md` (pipeline stub)
- [ ] Add `verify-template.md` (input template)
- [ ] Verify skill loads in Claude Code without errors

---

## #2 — Test Zotero MCP retrieval quality
> Requires: #1

- [ ] Run `zotero_get_item_fulltext` on 5 real Zotero items — does it return usable text (≥500 words)?
- [ ] Run `zotero_get_annotations` on same 5 items — how many have annotations?
- [ ] Run `zotero_get_item` on same 5 items — does metadata reliably include DOI?
- [ ] Document findings: which retrieval path will dominate in practice (Zotero fulltext, annotations, or Downloader)?

---

## #3 — Test claim extraction prompt on 10 synthetic examples
> Requires: #1 (can run in parallel with #2)

- [ ] Assemble 10 (draft paragraph, citation marker) pairs manually
- [ ] Run claim extraction prompt (spec §7.1) on all 10
- [ ] Check: does it extract the right claim? Is it specific enough (quantitative preferred over qualitative)?
- [ ] Fix prompt if needed; document any edge cases (multi-claim sentences, implicit attribution)

---

## #4 — Test verdict classification prompt on 20 synthetic examples
> Requires: #3

- [ ] Assemble 20 (claim, paper excerpt) pairs:
  - 8 `supported`
  - 6 `not_found`
  - 4 `contradicted`
  - 2 scope-mismatch (expect `not_found`)
- [ ] Run verdict prompt (spec §7.2) on all 20
- [ ] Verify: 0 false `supported` on the 4 `contradicted` cases
- [ ] Verify: ≤1 wrong on the 2 scope-mismatch cases
- [ ] Tune prompt if targets not met; document changes

---

## #5 — Wire up Downloader fallback and test on 3 paywalled papers
> Requires: #2

- [ ] Implement retrieval cascade (spec §8): Zotero fulltext → annotations → Downloader CLI → Read tool
- [ ] Test on 1 Wiley paper — PDF saves to `/tmp/cv_<slug>.pdf`, Read tool extracts usable text
- [ ] Test on 1 Springer paper
- [ ] Test on 1 Elsevier paper
- [ ] Verify: `unverifiable` gate fires correctly on retrieval failure (no abstract-only classification)
- [ ] Document Downloader error strings → map to `unverifiable` reason messages

---

## #6 — Run end-to-end on a real paragraph with known citation issues
> Requires: #4 and #5

- [ ] Find a Hawthorne effect paper that mis-cites the original (155/196 articles did — see `pain-discovery-report.md`)
- [ ] Run full pipeline: claim extraction → retrieval → verdict → report
- [ ] Record per citation: retrieval path taken, verdict, latency
- [ ] Check verdict accuracy against known ground truth
- [ ] Verify `unverifiable` gate holds throughout (no abstract-only classifications)
- [ ] Fix any prompt or retrieval issues before proceeding

---

## #7 — Write README and ship to GitHub
> Requires: #6

- [ ] Write `README.md`: prerequisites, setup steps, usage instructions
- [ ] Add worked example: real paragraph + real `verify-report.md` output
- [ ] Push repo public
- [ ] Add "Built with Downloader" link to Downloader repo README

---

## Dependency map

```
#1 Create repo skeleton
  ├── #2 Test Zotero MCP retrieval        (parallel)
  │     └── #5 Wire Downloader fallback
  │             └── #6 End-to-end test ◄─── both #4 and #5 required
  └── #3 Test claim extraction prompt     (parallel)
        └── #4 Test verdict prompt
                └── #6 End-to-end test
                      └── #7 README + ship
```
