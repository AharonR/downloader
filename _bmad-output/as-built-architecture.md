# As-Built Architecture — Downloader

**Status:** Current as of Epic 11 (2026-03-08)
**Source:** Derived from codebase; supplements `_bmad-output/planning-artifacts/architecture.md`

---

## Module Map

### `downloader-core` — Library crate

All business logic lives here. Binary crates (`downloader-cli`, `downloader-app/src-tauri`)
depend on this crate and add zero business logic of their own.

```
downloader-core/src/
├── lib.rs                    # Public re-exports; #[deny(clippy::expect_used)]
├── auth/                     # Cookie management and browser capture
│   ├── capture.rs            # Browser cookie capture + validation
│   └── cookies.rs            # Netscape/JSON cookie parsing, in-memory jar
├── db.rs                     # SQLite connection pool (sqlx, WAL mode)
├── download/                 # HTTP download engine
│   ├── mod.rs                # DownloadEngine orchestration
│   ├── client.rs             # HttpClient (reqwest, User-Agent, bot-detection retry)
│   ├── engine.rs             # Concurrent download loop, progress callbacks
│   ├── filename.rs           # Content-Type → extension mapping, filename sanitization
│   ├── rate_limiter.rs       # Per-domain rate limiting (DashMap)
│   ├── retry.rs              # Exponential backoff, RetryPolicy
│   └── robots.rs             # robots.txt fetch + parse + check
├── parser/                   # Input parsing from text/stdin
│   ├── mod.rs                # parse_input() — top-level entry point
│   ├── bibtex.rs             # BibTeX entry extraction
│   ├── bibliography.rs       # Bibliography heading + numbered-entry detection
│   ├── doi.rs                # DOI pattern detection and validation
│   ├── error.rs              # ParseError enum (thiserror)
│   ├── input.rs              # ParsedItem, InputType, ParseResult
│   ├── reference.rs          # Academic reference string parsing + confidence scoring
│   └── url.rs                # URL extraction, backslash cleanup, validation
├── project.rs                # Project folder creation, index.md, download.log, sidecars
├── queue/                    # Download queue (SQLite-backed)
│   ├── mod.rs                # Queue struct + public API
│   ├── error.rs              # QueueError enum (thiserror)
│   ├── history.rs            # DownloadAttempt recording and query
│   ├── item.rs               # QueueItem, QueueStatus, QueueMetadata
│   └── repository.rs         # sqlx queries (compile-time checked)
├── resolver/                 # URL/DOI → downloadable URL resolution
│   ├── mod.rs                # Resolver trait, ResolverRegistry, build_default_resolver_registry()
│   ├── arxiv.rs              # ArXiv PDF normalization
│   ├── crossref.rs           # Crossref REST API, DOI → PDF URL
│   ├── direct.rs             # Pass-through fallback
│   ├── error.rs              # ResolveError enum (thiserror)
│   ├── http_client.rs        # Shared resolver HTTP client policy
│   ├── ieee.rs               # IEEE Xplore PDF extraction
│   ├── pubmed.rs             # PubMed/PMC full-text resolution
│   ├── sciencedirect.rs      # ScienceDirect PDF endpoint
│   ├── springer.rs           # Springer canonical PDF URL
│   ├── utils.rs              # Shared regex patterns (4 centralized)
│   └── youtube.rs            # YouTube oEmbed metadata + timedtext transcript
├── sidecar/                  # JSON-LD sidecar file generation
│   └── mod.rs                # generate_sidecar() — Schema.org metadata
└── topics.rs                 # Topic extraction from title/abstract (keyword list)
```

### `downloader-cli` — CLI binary crate

```
downloader-cli/src/
├── main.rs           # #[tokio::main], anyhow error chain, exit codes
└── cli.rs            # clap derive structs (Commands, DownloadArgs, HistoryArgs)
```

Thin layer: parses CLI args, calls `downloader_core` functions, formats output via `indicatif`.

### `downloader-app` — Tauri desktop app

```
downloader-app/
├── src/                          # Svelte 5 frontend (SvelteKit)
│   └── lib/
│       ├── CompletionSummary.svelte   # Download batch result with expand/collapse failures
│       ├── DownloadForm.svelte        # URL/DOI input + project + download button
│       ├── ProgressDisplay.svelte     # Per-item progress bars (real-time events)
│       ├── ProjectSelector.svelte     # <input list="..."> with native datalist suggestions
│       └── utils.ts                   # Shared frontend utilities
└── src-tauri/src/
    ├── lib.rs                    # Tauri app setup, AppState registration
    └── commands.rs               # Tauri IPC commands (thiserror boundary → anyhow in binaries)
```

---

## Key Data Flows

### URL → Download → Sidecar

```
stdin / CLI args / GUI input
        │
        ▼
  parse_input()              parser/mod.rs
  ┌─────────────────────────────────────────────┐
  │ detect InputType (URL / DOI / Reference /   │
  │ BibTeX / Bibliography)                      │
  │ → Vec<ParsedItem>                           │
  └─────────────────────────────────────────────┘
        │
        ▼
  ResolverRegistry::resolve()  resolver/mod.rs
  ┌─────────────────────────────────────────────┐
  │ Dispatch by priority:                       │
  │   Specialized (youtube, arxiv, pubmed,      │
  │               ieee, springer, sciencedirect)│
  │   General    (crossref for DOIs)            │
  │   Fallback   (direct for plain URLs)        │
  │ → ResolveStep::Url | Redirect | NeedsAuth   │
  │   | Failed                                  │
  └─────────────────────────────────────────────┘
        │
        ▼
  Queue::enqueue()           queue/repository.rs
  ┌─────────────────────────────────────────────┐
  │ Insert QueueItem (pending) into SQLite      │
  │ Attach metadata (title, authors, doi, year) │
  │ Attach parse_confidence if reference-derived│
  └─────────────────────────────────────────────┘
        │
        ▼
  DownloadEngine::run()      download/engine.rs
  ┌─────────────────────────────────────────────┐
  │ Tokio concurrency: N workers (default 10)   │
  │ Per-domain rate limiting (1s default)       │
  │ Exponential backoff retry (default 3)       │
  │ robots.txt checked (opt-in --check-robots)  │
  │ Resume support (bytes_downloaded tracking)  │
  │ Progress callbacks → indicatif / Tauri emit │
  └─────────────────────────────────────────────┘
        │
        ▼
  File saved to <output_dir>/<project>/
        │
        ▼
  generate_sidecar()         sidecar/mod.rs
  ┌─────────────────────────────────────────────┐
  │ Schema.org JSON-LD written alongside file   │
  │ .downloader/<filename>.jsonld               │
  └─────────────────────────────────────────────┘
        │
        ▼
  append_project_index()     project.rs
  append_project_download_log()
  ┌─────────────────────────────────────────────┐
  │ index.md — markdown table of completed items│
  │ download.log — session log with status rows │
  │ Session label: YYYY-MM-DD_HHhMMmSSs         │
  └─────────────────────────────────────────────┘
```

### Error Boundary

```
downloader-core  →  thiserror error enums (library)
downloader-cli   →  anyhow::Context chains (binary)
downloader-app   →  anyhow::Context chains (binary / Tauri commands)
```

User-facing errors follow the **What / Why / Fix** convention (see `project-context.md`).

---

## Architecture Invariants

| Invariant | Enforcement |
|-----------|-------------|
| `#[deny(clippy::expect_used)]` in lib code | `src/lib.rs` lint attribute |
| No `.unwrap()` in library code | clippy gate in CI |
| Zero `unsafe` blocks | verified by `cargo geiger` (informational) |
| All public API items have `///` doc comments | doc lint |
| `thiserror` in library, `anyhow` in binaries | architecture rule; no shared error module |
| `DOWNLOADER_REQUIRE_SOCKET_TESTS=1` in CI | `phase-rollout-gates.yml` env var |
| Single SQLite WAL-mode connection pool | `db.rs` |
| Resolver stateless — no instance request state | resolver trait contract |

---

## Accepted Security Advisories

See `.cargo/audit.toml` and `deny.toml` for current accepted advisories with rationale.

- **RUSTSEC-2023-0071** (rsa Marvin Attack) — transitive via sqlx-mysql; rsa never invoked
- **RUSTSEC-2025-0119** (number_prefix unmaintained) — transitive via indicatif; no CVE

---

## Deferred Items (tracked in _bmad-output/planning-artifacts/epic-12-backlog.md)

- Windows CI runner (untested; path-related changes need manual validation)
- Throughput benchmark (Criterion)
- YouTube chapter extraction
- RSS/heap profiling
- cargo geiger (zero unsafe confirmed manually)
