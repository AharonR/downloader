# Product Options

Three candidate products derived from the hypothesis landscape (H4, H13/H17, H19).
Each folder contains a product brief and a research process that feeds the shared pain discovery skill.

| Option | Folder | Layer | Ships When |
|--------|--------|-------|------------|
| Citation Verifier | `citation-verifier/` | Independent (no prerequisites) | Now (2 weeks) |
| Research Wiki Compiler | `research-wiki-compiler/` | Foundation (H13 + H17) | After Citation Verifier |
| Argument Graph Layer | `argument-graph-layer/` | Built on wiki (H19) | After Wiki Compiler MVP |

## Sequencing Rationale

- **Citation Verifier** is independent. Ships first, validates Downloader + Zotero MCP integration.
- **Research Wiki Compiler** is the substrate everything else depends on.
- **Argument Graph Layer** is the primary research contribution. Requires the wiki to exist first.

Running customer discovery for all three in parallel is cheap. Building them must be sequential.

---

## Research Process Structure

Each product has a two-phase research process:

**Phase A — Automated (run first):**
Uses `skills/pain-discovery.md` with context defined in each `research-process.md`.
Each research process defines: pain hypothesis, competitors, search channels, signal taxonomy,
and the key discriminant question the skill must answer before Phase B begins.

**Phase B — Human validation (3–4 interviews):**
Stimulus-only. Phase A answers "does this pain exist?"
Phase B answers "does my specific artifact produce value?" — which corpus mining cannot answer.

To run Phase A for any product:
```
Run the pain discovery skill using the context defined in
_product/[product-folder]/research-process.md
```

---

## Files

```
_product/
├── README.md                                  this file
├── skills/
│   └── pain-discovery.md                      shared agent skill (Phase A)
├── citation-verifier/
│   ├── product-brief.md
│   └── research-process.md                    context + tools + Phase B guide
├── research-wiki-compiler/
│   ├── product-brief.md
│   └── research-process.md
└── argument-graph-layer/
    ├── product-brief.md
    └── research-process.md
```
