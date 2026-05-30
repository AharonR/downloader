# Research Wiki Compiler

> Annotation-aware Zotero → LLM wiki compiler. Builds a personal markdown wiki from your library: paper pages, entity pages, claim provenance, and bidirectional backlinks.

> **Status:** v0.0.1 alpha — scaffold only, pipeline not implemented.

## What it does

Three-stage pipeline:

1. **Ingest** — pulls metadata, fulltext, and annotations from Zotero MCP into `raw/<slug>/`
2. **Compile** — runs parallel annotation-grounded + cold-start prompts per paper, diffs the outputs
3. **Orchestrate** — writes `wiki/papers/` and `wiki/concepts/` with bidirectional backlinks and entity consolidation

## Why it's different

- **Annotation-grounded compile** — your highlights and marginalia are first-class compile input, not decoration. The wiki carries *your* reading voice.
- **Diff-as-product** — annotation-grounded and cold-start outputs are diffed; the difference surfaces "you may have missed" (model-only) and "researcher caught X" (annotation-only) sections.
- Compared to llm-wiki-compiler / NotebookLM / ZotAI: persistent wiki structure, Zotero-native, annotation-aware compile.

## Quickstart

> TODO — pipeline not yet implemented. See [working plan](../../_product/research-wiki-compiler/h13-working-plan.md) for status and sequencing.

## Architecture

See [pipeline spec](../../_product/research-wiki-compiler/h13-phase0-spec.md).

Detailed specs:

- [Compile prompt spec](../../_product/research-wiki-compiler/spec_compile_prompt.md)
- [Session-start report spec](../../_product/research-wiki-compiler/spec_session_start_report.md)
- [Lint phase spec](../../_product/research-wiki-compiler/spec_lint_phase.md)

## Status

Scaffold authored 2026-05-07. Implementation tracked in [working plan](../../_product/research-wiki-compiler/h13-working-plan.md).

## License

MIT — see [LICENSE](LICENSE).
