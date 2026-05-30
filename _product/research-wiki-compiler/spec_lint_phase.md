# Lint Phase Specification

Source: `entity_personal_research_wiki.md` (4-phase cycle), `wiki_compiler_research.md` (scale ceiling), `h13_design_constraints.md` (Constraint 5), `h13_pilot_learnings.md` (compile bottleneck).

## Why This Is Load-Bearing

At ~200 papers, a "bad wiki article becomes a prior that poisons future generations" (community quote from llm-wiki-compiler). The compile phase doesn't solve maintenance forever — at scale, silent error accumulation is the primary failure mode.

The lint phase is not optional. It is the mechanism that prevents the wiki from stratifying (Lesson 1 from PKM failure patterns) and from accumulating unchecked errors that trigger trust breaks.

## When It Runs

**Two modes:**

1. **Per-compile lint** (runs after every paper compile): lightweight checks on the newly compiled content only
2. **Full lint** (periodic, user-triggered or scheduled): comprehensive audit across the entire wiki

**Frequency guidance:**
- Per-compile lint: automatic, every time a paper is compiled
- Full lint: at least monthly, or when raw queue exceeds 10 papers, or before any significant writing project

## Per-Compile Lint (Lightweight)

Runs automatically after each paper's compile phase. Checks only the newly created/updated content.

| Check | What it catches |
|---|---|
| **Backlink integrity** | New entity page created but not linked from existing pages that mention the same concept |
| **Index update** | New file created but not added to `_index.md` |
| **Source mapping** | New paper added but not reflected in `_sources.md` |
| **Duplicate entity detection** | New entity page created for a concept that already exists under a different name |
| **Claim provenance** | Any claim in the new content lacks source paper + page number |

**Output**: Inline warnings during compile. Block publish if provenance check fails.

## Full Lint (Comprehensive)

### Structural Checks

| Check | What it catches | Severity |
|---|---|---|
| **Broken backlinks** | Links to pages that don't exist or were renamed | Error |
| **Orphan pages** | Entity pages with no inbound links from any other page | Warning |
| **Unindexed files** | Files on disk not listed in `_index.md` | Error |
| **Stale index entries** | Index entries pointing to deleted or renamed files | Error |
| **Coverage gaps** | Concepts mentioned in 3+ papers but no dedicated entity page | Suggestion |

### Content Checks

| Check | What it catches | Severity |
|---|---|---|
| **Contradictions** | Two entity pages making opposing claims about the same concept | Flag for review |
| **Stale entity pages** | Pages not updated in >30 days while new papers touching the concept were compiled | Warning |
| **Thin entity pages** | Pages with only one source paper and no annotations | Suggestion |
| **Unsupported claims** | Claims in entity pages that trace to a paper no longer in the library (deleted from Zotero) | Error |
| **Annotation drift** | Entity page "researcher perspective" section contradicts the researcher's own annotations | Flag for review |

### Entity Consolidation (Hardest Problem)

The pilot wiki revealed that the same concept gets compiled from multiple papers as separate entity pages (the "Karpathy-triplicate problem"). At 200 papers, this produces fragmentation instead of compound growth.

| Check | Action |
|---|---|
| Semantic similarity between entity page titles | Flag potential duplicates (e.g., "causal-inference" and "causal-identification") |
| Entity pages with >80% overlapping source papers | Suggest merge |
| Entity pages with the same key claims from different sources | Suggest merge with consolidated evidence |

**Merge operation**: When two entity pages are merged, all backlinks from both pages are preserved, evidence is deduplicated, and the discarded page becomes a redirect.

## Output Format

```markdown
# Lint Report — 2026-04-13

## Errors (3)
- BROKEN_LINK: concepts/panel-data.md → papers/wooldridge-2020.md (file not found)
- UNINDEXED: concepts/heteroskedasticity.md exists but not in _index.md
- UNSUPPORTED: concepts/iv-estimation.md cites papers/stock-2018 which is no longer in Zotero library

## Warnings (2)
- STALE: concepts/regression-discontinuity.md last updated 2026-03-01; 2 new papers compiled since
- ORPHAN: concepts/heteroskedasticity.md has no inbound links

## Contradictions (1)
- concepts/measurement-invariance.md:
  claim_012 (Smith 2019, p.14): "threshold of 0.05 is standard"
  claim_047 (Jones 2021, p.8): "threshold should be at least 0.01"
  → Requires researcher review

## Suggestions (2)
- COVERAGE_GAP: "fixed effects" mentioned in 4 papers but no entity page
- MERGE_CANDIDATE: concepts/causal-inference.md and concepts/causal-identification.md share 3/4 source papers

## Stats
- Total entity pages: 47
- Total paper pages: 30
- Average backlinks per page: 4.2
- Coverage: 12 high, 18 medium, 9 low, 8 stub
```

## Connection to Other Components

- **Session-start report** (`spec_session_start_report.md`): The lint report feeds the contradiction log, open questions, and index health sections of the session-start report.
- **H19 argument graph**: Once H19 is built, the contradiction check upgrades from "two pages make opposing claims" to "typed `contradicts` edge with confidence and provenance."
- **H18 epistemic versioning**: Lint changes are committed to git, producing the evolution record H18 queries.
- **Entity consolidation**: The merge operation is the hardest unsolved problem identified in the pilot. It requires semantic understanding of when two concept names refer to the same underlying idea — not just string matching.
