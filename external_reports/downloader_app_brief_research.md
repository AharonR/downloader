Plan: I’ll restate the core “Downloader” concept I’m hearing (identity, architecture, workflow), then propose research directions that tighten the technical spec, validate domains/market, and de-risk legal/security.


Here’s what I understand you’re designing:

* **“Downloader” isn’t a file grabber**. It’s an **information ingestion and normalization engine**: you give it messy inputs (URLs, DOI/BibTeX/ISBN, lists, folders), it fetches what it’s allowed to fetch, and it outputs **accessible information** (files *plus* structured representations) with strong **provenance** and **memory**.
* It has a **two-layer identity**:

  1. a **tool layer** (CLI/GUI) that reliably fetches + stores “file + envelope (metadata)”, and
  2. a **product/knowledge layer** that continuously extracts, enriches, dedups, and makes the corpus queryable and project-scoped.
* Your core primitive is **“list + actions”**: jobs flow through a pipeline, actions are composable, and plugins extend edges without bloating the core.
* The brand promises are operational: **“remembers everything,” “set it and trust it,” “one input, everything handled.”**

One important constraint: I can’t help design or implement flows that fetch copyrighted content from pirate sources (your Sci-Hub/LibGen example). I *can* help you build the same UX and reliability using **legal** sources and the user’s legitimate access (publisher links, institutional proxies, open-access discovery, etc.).

---

## Research directions to tighten the technical spec (highest leverage)

### 1) Define the “Envelope” as the atomic unit (your real product)

Turn “metadata is first-class” into a crisp contract:

**Envelope should minimally include**

* Input normalization: `original_input`, `normalized_identifier` (doi/isbn/url/etc), `resolver_steps`
* Retrieval provenance: timestamps, request method, redirects, response headers (scrub secrets), final URL, content-type, byte size
* Integrity: hashes (content + normalized text), duplicate keys
* Rights & policy: license if known, OA status if known, allowed/blocked reason
* Artifact pointers: where the raw file lives + where derived representations live

This becomes the backbone for your **structured result manifest** and “universal history.”

### 2) Build a resolver graph for scholarly content (legally)

Your “source chaos → information” idea becomes very concrete if you treat retrieval as a **graph of resolvers**:

* DOI → metadata via Crossref (include polite identification and respect rate limits) ([www.crossref.org][1])
* DOI → OA locations via Unpaywall ([unpaywall.org][2])
* Title/author query → candidate works via OpenAlex search/filter ([docs.openalex.org][3])

Then your fallback chain becomes: **OA copy → publisher (if user has access) → “needs credentials / not accessible”** (actionable error, not a dead end).

### 3) Reliability research: make jobs idempotent and observable

To fulfill “set it and trust it,” your spec should explicitly define:

* **Idempotency key** per job (based on normalized identifier + requested actions)
* **State machine**: queued → resolving → fetching → verifying → extracting → enriching → done/failed
* **Retry semantics**: exponential backoff + “do not retry” classes (401/403, robots policy, explicit user cancel)
* **Per-site and per-API rate limiting** (with policy files per connector), and queue-aware concurrency

Bonus: store a **semantic progress model** (“rate-limited”, “waiting on resolver”, “blocked by auth”) rather than only bytes/time.

### 4) Dedup research: content-aware, not filename-aware

You’ll want layered dedup:

* **Binary hash** (exact duplicates)
* **Text fingerprint** (near duplicates across different PDFs/versions)
* **Metadata-level equivalence** (same DOI, same work-id)

For PDFs, expect extraction noise—design your fingerprinting to be resilient.

### 5) Extraction & “LLM-ready” outputs that stay grounded

Your AI cluster becomes safe and powerful if you anchor it to the envelope:

* Derived artifacts: `normalized_text`, `markdown`, `sections`, `citations`, `chunks[]`
* Each chunk carries **source attribution pointers** back to provenance (URL, page range, offsets, extraction method)

That lets you do RAG/agents without losing traceability.

---

## Agent integration research (forward-looking, but worth speccing now)

### MCP server / Tool API as your “external brain port”

If you want agents to call Downloader, MCP is a good target: it standardizes how clients discover tools and call them. Start by defining a small tool surface (e.g., `resolve(input)`, `fetch(job)`, `status(id)`, `query(kb, filters)`). ([Model Context Protocol][4])

Key research topics here:

* Tool design that returns **structured manifests** (your #62) and streams progress
* Auth boundaries if you ever support cloud or shared KBs (even if “no account” remains default)

---

## Market/domain research (even if this stays personal-use)

### Map where you sit in the landscape

You’re converging three existing categories:

1. download managers (fast + reliable)
2. reference managers / research tooling (metadata + organization)
3. personal knowledge bases (query + memory)

Your differentiator is the **end-to-end pipeline + provenance-first memory**.

### Research questions worth answering early

* **Primary wedge**: academic PDFs first, or “any web resource”? (Wedges drive connector priorities.)
* **User mental model**: “I paste a DOI/URL/list” vs “I drop files in a folder” vs “I run saved searches weekly”
* **Switching costs**: how you coexist with existing tools (Zotero/Obsidian/etc.) via export formats and watch folders

---

## Other beneficial domains to research (often overlooked, high payoff)

### Security & privacy (especially if “remembers everything”)

* Threat model: downloaded files are untrusted; extraction should be sandboxed.
* Secret handling: never store cookies/tokens inside envelopes; separate secure store.
* “Universal history” should support **redaction** and **scoped retention** per project.

### Policy & compliance (keeps the project future-proof)

* Respect robots.txt / ToS for web sources
* Make “why this failed” user-legible: *blocked by access*, *no OA location*, *rate-limited*, *source changed*, etc.

### Community rules ecosystem (even for solo use)

You’ve already implied “rule infrastructure.” Spec a format that’s:

* versioned
* testable (fixtures)
* safe to share without leaking personal tokens

---

## A practical next step for your spec (so it stays crisp)

Write a 1–2 page “Downloader Core Contract” containing only:

* Envelope schema (fields + invariants)
* Job state machine + retry/rate-limit rules
* Plugin interface (connectors, actions, destinations)
* Manifest output format (machine- and human-readable)

If you want, paste your current rough tech stack preference (language, target OS, CLI-only vs GUI) and I’ll translate the above into a concrete module layout and minimal v0 feature set.

[1]: https://www.crossref.org/blog/announcing-changes-to-rest-api-rate-limits/?utm_source=chatgpt.com "Blog - Announcing changes to REST API rate limits"
[2]: https://unpaywall.org/products/api?utm_source=chatgpt.com "REST API"
[3]: https://docs.openalex.org/api-entities/works?utm_source=chatgpt.com "Works | OpenAlex technical documentation"
[4]: https://modelcontextprotocol.io/specification/2025-06-18?utm_source=chatgpt.com "Specification"
