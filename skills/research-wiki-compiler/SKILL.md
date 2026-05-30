---
name: research-wiki-compiler
description: Compiles a Zotero library into a personal LLM wiki with annotation-aware paper pages and entity backlinks. Use when the user has a Zotero library and wants to build a research wiki, compile annotations into entity pages, generate paper summaries with claim provenance, or invoke the H13 compile pipeline. Adapts the Karpathy LLM wiki pattern for academic PDFs with annotation-grounded compile and diff-as-product output.
disable-model-invocation: true
---

# Research Wiki Compiler

> **Status:** v0.0.1 alpha — scaffold only, pipeline not yet implemented. See working plan for sequencing.

## What it does

Three-stage pipeline:

1. **Ingest** — pull metadata, fulltext, annotations from Zotero MCP into `raw/<slug>/`
2. **Compile** — run parallel annotation-grounded + cold-start compile prompts per paper, diff outputs
3. **Orchestrate** — write/update `wiki/papers/` and `wiki/concepts/`, maintain bidirectional backlinks, run entity consolidation

The diff between annotation-grounded and cold-start outputs is the differentiated surface: "you may have missed" (model surfaced, you didn't annotate) and "researcher caught X" (annotation surfaced, model missed).

## Quick start

> TODO: implementation pending W3-W6. See working plan.

## Required environment

- `ANTHROPIC_API_KEY` — Sonnet for compile subagents, Opus for orchestration
- `zotero-mcp` server (`54yyyu/zotero-mcp`) registered in Claude Code MCP config
- Zotero Desktop running locally with target library open

## Reference

Architecture and specs:

- Pipeline spec: `../../_product/research-wiki-compiler/h13-phase0-spec.md`
- Compile prompt spec: `../../_product/research-wiki-compiler/spec_compile_prompt.md`
- Session-start report spec: `../../_product/research-wiki-compiler/spec_session_start_report.md`
- Lint phase spec: `../../_product/research-wiki-compiler/spec_lint_phase.md`
- Working plan: `../../_product/research-wiki-compiler/h13-working-plan.md`
- Background and rationale: `../../_product/research-wiki-compiler/h13-phase0-background.md`
