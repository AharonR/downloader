---
stepsCompleted: [1, 2, 3, 4, 5, 6, 7, 8]
inputDocuments:
  - "_bmad-output/planning-artifacts/prd.md"
  - "_bmad-output/planning-artifacts/product-brief-Downloader-2026-01-20.md"
  - "_bmad-output/planning-artifacts/research/technical-authenticated-downloads-research-2026-01-20.md"
  - "_bmad-output/analysis/brainstorming-session-2026-01-15.md"
workflowType: 'architecture'
project_name: 'Downloader'
user_name: 'fierce'
date: '2026-01-25'
partyModeInsights: true
lastStep: 8
status: 'complete'
completedAt: '2026-01-26'
---

# Architecture Decision Document

_This document builds collaboratively through step-by-step discovery. Sections are appended as we work through each architectural decision together._

## Project Context Analysis

### Requirements Overview

**Functional Requirements:**
The system requires 5 major capability areas:
1. **Input Parsing** (6 input types) - URL detection, DOI resolution, reference parsing, bibliography extraction
2. **Download Engine** (7 requirements) - HTTP client, auth support, site resolvers, retry, concurrency, rate limiting, resume
3. **Organization** (6 requirements) - Project folders, file naming, indexing, topic detection, JSON-LD metadata
4. **Logging & Memory** (5 requirements) - Download logs, failure tracking, queryable history
5. **CLI Interface** (6 requirements) - stdin, flags, progress, summary, dry-run, config

**Non-Functional Requirements:**
| Category | Key Target | Architectural Impact |
|----------|------------|---------------------|
| Performance | Parse 150 refs < 5s, 10 concurrent | Async processing, semaphore queue |
| Reliability | 90% success, never crash | Graceful degradation, comprehensive logging |
| Usability | Zero-config start | Sensible defaults, progressive configuration |
| Maintainability | Modular resolvers | Plugin architecture, clear interfaces |

**Scale & Complexity:**
- Primary domain: Desktop CLI tool (Rust/Tauri)
- Complexity level: Medium
- Estimated architectural components: 6 major modules

### Technical Constraints & Dependencies

**Framework Decision (Pre-validated):**
- Tauri 2.0 for desktop shell
- Rust backend for core logic
- reqwest for HTTP operations

**External Dependencies:**
- Crossref API (DOI metadata)
- Unpaywall API (open access locations)
- Browser extension (auth cookie capture)

**Constraints:**
- CLI-first for MVP (no GUI)
- Single-user, local-first (no cloud sync)
- Must work offline after initial setup

### Cross-Cutting Concerns Identified

1. **Error Handling Philosophy:** Log everything, never block queue, actionable messages
2. **Authentication Flow:** Browser → Extension → Native Messaging → Cookie Store → Download
3. **Concurrency Model:** Per-domain rate limiting + global semaphore (10 concurrent)
4. **Metadata Pattern:** Envelope architecture with JSON-LD for future interoperability
5. **Extensibility:** Plugin interfaces for resolvers, input parsers, and actions

### Architectural Gaps Requiring Decisions

*Identified through multi-perspective analysis (Party Mode):*

#### Resolver Architecture (High Priority)
- **Contract undefined:** Does a resolver return a URL or handle the download?
- **Composition model:** How do multi-step resolvers chain (auth → redirect → extract)?
- **Failure semantics:** Fail fast vs. internal fallbacks?

#### Cookie/Auth Storage (High Priority)
- **Persistence model:** Memory-only (session) vs. disk (persisted)?
- **Scope:** Per-project cookies or global cookie store?
- **Security implications:** Encrypted storage requirements?

#### Queue Persistence (Medium Priority)
- **Crash recovery:** Is queue state persisted? SQLite vs. memory?
- **"Never block queue" implementation:** Constraint needs concrete design

#### Logging as Active System (Medium Priority)
- **Passive audit trail vs. active work queue:** Logs imply Model A (batch and forget), but failures need Model B (interactive triage)
- **Failure discovery:** How does user find what needs attention without reading raw logs?
- **Retry UX:** Does CLI need interactive "retry failed" mode?

#### Observability Architecture (Medium Priority)
- **Per-site success metrics:** Not just global 90%, but per-resolver tracking
- **Failure categorization:** Auth vs. network vs. parse vs. site-blocked
- **Telemetry for product decisions:** Beyond debugging logs

#### Concurrency Refinement (Low Priority)
- **Dual-axis model:** `concurrency_global: 10` + `concurrency_per_domain: 2`
- **Configurable per-domain limits:** Some sites tolerate more, some less

#### Performance Constraints Refinement (Low Priority)
- **DOI resolution latency:** 150 refs with 100 DOIs hits network; needs cache + batch strategy
- **Memory budget allocation:** Suggest per-component limits (Download: 50MB, Metadata: 50MB, Headroom: 100MB)

#### Testability Requirements (Medium Priority)
- **Resolver testing:** Mock server strategy for unit tests, "known good" corpus for integration
- **Auth flow E2E:** Consider Playwright test harness or "test mode" with file-based cookies
- **Breakage detection:** How to detect when a resolver breaks in production?

## Starter Template Evaluation

### Primary Technology Domain

Desktop CLI tool (Rust) with future GUI (Tauri 2.0)

### Starter Options Considered

| Option | Approach | MVP Fit | v2 Fit | Team Consensus |
|--------|----------|---------|--------|----------------|
| Pure Rust CLI | cargo new | Excellent | Requires migration | Too minimal |
| Tauri from Day 1 | create-tauri-app | Good | Excellent | Premature |
| Rust Workspace | Multi-crate | Excellent | Excellent | Over-engineered for solo |
| **Lib/Bin Split** | Single crate, dual targets | **Excellent** | **Good** | **Recommended** |

### Selected Approach: Single Crate with Lib/Bin Split

**Rationale (validated through multi-perspective analysis):**
- Core logic in `lib.rs` establishes clean separation without workspace overhead
- CLI binary imports library - same pattern Tauri will use later
- No feature flag gymnastics when adding GUI
- Simpler CI/CD: single `cargo test` covers everything
- Refactor to workspace is trivial when (if) GUI development starts
- Avoids "architecting for v2 before validating v1"

**Initialization Commands:**

```bash
# Create project
cargo new downloader
cd downloader
```

**Cargo.toml:**

```toml
[package]
name = "downloader"
version = "0.1.0"
edition = "2024"

[lib]
name = "downloader_core"
path = "src/lib.rs"

[[bin]]
name = "downloader"
path = "src/main.rs"

[dependencies]
# Async runtime
tokio = { version = "1", features = ["full"] }

# HTTP client
reqwest = { version = "0.13", features = ["json", "cookies", "stream", "gzip"] }

# Serialization
serde = { version = "1", features = ["derive"] }
serde_json = "1"

# Error handling
anyhow = "1"
thiserror = "2"

# CLI
clap = { version = "4.5", features = ["derive"] }
indicatif = "0.17"

# Logging
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter"] }

[dev-dependencies]
wiremock = "0.6"      # HTTP mocking for tests
tempfile = "3"        # Temp directories for test isolation
tokio-test = "0.4"    # Async test utilities
```

**Project Structure:**

```
downloader/
├── Cargo.toml
├── src/
│   ├── lib.rs              # Library root: pub mod declarations + pub use re-exports
│   ├── main.rs             # CLI entry point
│   ├── cli.rs              # CLI argument definitions (clap derive)
│   ├── app_config.rs       # App configuration
│   ├── db.rs               # Database connection + migrations
│   ├── user_agent.rs       # User-Agent string construction
│   ├── bin/                # Additional binary targets (extract_md_links, stress_sidecar_flaky)
│   ├── app/                # Application orchestration layer
│   ├── commands/           # CLI command implementations
│   ├── auth/               # Authentication & cookie management
│   ├── download/           # Download engine
│   ├── failure/            # Failure taxonomy
│   ├── output/             # CLI output formatting
│   ├── parser/             # Input parsing
│   ├── project/            # Project directory management
│   ├── queue/              # Download queue + history
│   ├── resolver/           # Resolver trait + registry + site resolvers
│   ├── search/             # Past-download search
│   ├── sidecar/            # JSON-LD sidecar generation
│   ├── test_support/       # In-lib test utilities
│   └── topics/             # Topic extraction + normalization
└── tests/
    ├── auth_integration.rs
    ├── cli_e2e.rs
    ├── critical.rs         # Entry point for adversarial failure-mode suite
    ├── critical/           # Auth bypass, encryption failures, race conditions, etc.
    ├── download_engine_integration.rs
    ├── download_integration.rs
    ├── exit_code_partial_e2e.rs
    ├── integration_matrix.rs
    ├── nonfunctional_regression_gates.rs
    ├── parser_integration.rs
    ├── queue_integration.rs
    ├── resolver_integration.rs
    └── support/            # Shared test utilities (test_db, socket_guard)
```

**Architectural Decisions Provided:**

| Decision | Choice | Rationale |
|----------|--------|-----------|
| Language | Rust 2024 edition | Latest stable, required for project |
| Async Runtime | Tokio (full features) | Industry standard, reqwest requirement |
| HTTP Client | reqwest 0.13 | Async, cookies, streaming, well-maintained |
| CLI Framework | clap 4.5 (derive) | Type-safe, excellent help generation |
| Error Handling | thiserror (lib) + anyhow (bin) | Library errors are typed, CLI errors are contextual |
| Serialization | serde + serde_json | Standard for Rust, JSON-LD compatible |
| Progress Display | indicatif | Battle-tested, terminal progress bars |
| Logging | tracing | Structured logging, async-aware |
| Test Mocking | wiremock | HTTP mocking without external servers |

**Build Commands:**

```bash
cargo build --lib              # Library only
cargo build --bin downloader   # CLI binary
cargo test --lib               # Unit + lib integration tests
cargo test --bin downloader    # CLI tests
cargo test                     # All tests
```

**Migration Path to Tauri (v2):**

v1 ships as CLI only. GUI migration is the planned next phase.

When ready for GUI, extract to workspace:
1. Create `downloader-app/` with `cargo create-tauri-app`
2. Move `src/lib.rs` tree to `downloader-core/src/`
3. Update workspace `Cargo.toml` to include both crates
4. Tauri app imports `downloader_core` as dependency

Estimated refactor effort: minimal — the lib/bin split was validated across all 8 implementation epics; the public API surface in `lib.rs` is stable and behavior contracts are tested.

**Note:** Project initialization using these commands should be the first implementation story.

## Core Architectural Decisions

### Decision Priority Analysis

**Critical Decisions (Block Implementation):**
- Resolver architecture (contract, composition, failure handling)
- Cookie/auth storage model
- Data persistence strategy

**Important Decisions (Shape Architecture):**
- Concurrency model with per-domain rules
- Logging as structured queryable system
- Retry UX with failure categorization

**Infrastructure Decisions (Enable Quality):**
- Testing infrastructure patterns
- Resilience and crash safety

**Deferred Decisions (Post-MVP):**
- JSON sidecar export for metadata portability
- Knowledge graph integration
- MCP server API design

### Resolver Architecture

| Decision | Choice | Rationale |
|----------|--------|-----------|
| **Contract** | Hybrid Pipeline | Resolvers return `ResolveStep` enum (Url, Redirect, NeedsAuth, Failed). Enables complex flows while keeping resolvers testable. |
| **Composition** | Registry with Auto-Detection | Resolvers self-register, declare `can_handle()`. Engine tries applicable resolvers. Easy to add new site support. |
| **Failure Handling** | Fallback Chain with Priority | Specialized → General → Fallback priority. Supports "never block queue" philosophy with graceful degradation. |

```rust
// Resolver contract
trait Resolver: Send + Sync {
    fn name(&self) -> &str;
    fn priority(&self) -> ResolverPriority;
    fn can_handle(&self, input: &str) -> bool;
    fn resolve(&self, input: &str, ctx: &mut ResolveContext) -> Result<ResolveStep>;
}

enum ResolveStep {
    Url(ResolvedUrl),           // Final URL, hand to download engine
    Redirect(String),           // Intermediate, continue resolving
    NeedsAuth(AuthRequirement), // Signal auth needed
    Failed(ResolveError),       // Can't resolve
}

enum ResolverPriority { Specialized, General, Fallback }
```

**Resolution Loop Pattern (Engine-Level):**
```rust
// Engine handles redirect chaining explicitly
fn resolve_to_url(&self, input: &str) -> Result<ResolvedUrl> {
    let mut current_input = input.to_string();
    let mut attempts = 0;
    const MAX_REDIRECTS: usize = 10;

    loop {
        let resolver = self.registry.find_handler(&current_input)?;
        match resolver.resolve(&current_input, &mut ctx)? {
            ResolveStep::Url(url) => return Ok(url),
            ResolveStep::Redirect(new_url) => {
                attempts += 1;
                if attempts > MAX_REDIRECTS {
                    return Err(ResolveError::TooManyRedirects);
                }
                current_input = new_url;
                // Loop continues, registry finds new handler
            },
            ResolveStep::NeedsAuth(req) => return Err(ResolveError::AuthRequired(req)),
            ResolveStep::Failed(e) => {
                // Try next resolver in fallback chain
                if let Some(fallback) = self.registry.next_fallback(resolver.priority()) {
                    current_input = input.to_string(); // Reset to original
                    continue;
                }
                return Err(e);
            }
        }
    }
}
```

### Authentication & Security

| Decision | Choice | Rationale |
|----------|--------|-----------|
| **Cookie Persistence** | Hybrid (Session Default, Opt-in Persist) | Safe default, power users can save sessions. Respects security while enabling convenience. |
| **Cookie Scope** | Global Default + Per-Project Override | Common auth is global, project-specific when needed. Balances simplicity with flexibility. |
| **Credential Security** | Keychain for Key, File for Data | OS keychain protects encryption key. Encrypted cookie data in file (avoids keychain size limits). |
| **Cookie Debugging** | `--debug-cookies` flag | Shows cookie sources during auth troubleshooting. Essential for diagnosing auth failures. |

```rust
// Cookie storage architecture
struct CookieManager {
    global_store: EncryptedCookieStore,
    project_stores: HashMap<PathBuf, EncryptedCookieStore>,
    session_only: CookieJar,
}

impl CookieManager {
    fn get_cookies_for(&self, domain: &str, project: Option<&Path>) -> CookieJar {
        // Merge order: session → global → project (later overrides earlier)
        // With --debug-cookies: log source of each cookie
    }
}

// Key storage abstraction for testability
enum KeyStorage {
    OsKeychain,                    // Production: macOS Keychain, Windows Credential Manager
    InMemory(String),              // Testing: key provided directly
    Environment,                   // CI: key from DOWNLOADER_MASTER_KEY env var
}
```

### Data Architecture

| Decision | Choice | Rationale |
|----------|--------|-----------|
| **Metadata Storage** | SQLite Database | Fast queries for "show all failed", single file backup. Supports FR-4.5 (query past downloads). |
| **Queue Persistence** | Full Persistence (SQLite) | Crash recovery, resume interrupted downloads. "Never lose information" philosophy. |
| **SQLite Mode** | WAL + NORMAL sync | Write-Ahead Logging for concurrent reads during writes. NORMAL sync balances safety and performance. |
| **Event Batching** | Buffer + periodic flush | Reduce write contention: flush every 100ms or 10 events, whichever comes first. |

```rust
// Database initialization
fn init_database(path: &Path) -> Result<Database> {
    let pool = SqlitePool::connect(path)?;

    // Performance tuning for write-heavy workload
    sqlx::query("PRAGMA journal_mode=WAL").execute(&pool).await?;
    sqlx::query("PRAGMA synchronous=NORMAL").execute(&pool).await?;
    sqlx::query("PRAGMA cache_size=10000").execute(&pool).await?;

    Ok(Database { pool, event_buffer: EventBuffer::new() })
}

// Event batching to reduce write contention
struct EventBuffer {
    events: Vec<Event>,
    last_flush: Instant,
}

impl EventBuffer {
    fn add(&mut self, event: Event) {
        self.events.push(event);
        if self.events.len() >= 10 || self.last_flush.elapsed() > Duration::from_millis(100) {
            self.flush();
        }
    }
}
```

### Concurrency Model

| Decision | Choice | Rationale |
|----------|--------|-----------|
| **Limit Structure** | Configurable Per-Domain Rules | Global max (10) + per-domain default (2) + domain overrides. Prevents hammering strict sites. |

```rust
struct ConcurrencyConfig {
    global_max: usize,                             // 10
    per_domain_default: usize,                     // 2
    domain_overrides: HashMap<String, DomainConfig>,
}

struct DomainConfig {
    max_concurrent: usize,
    request_delay: Duration,
    respect_retry_after: bool,
}
```

### Logging & Observability

| Decision | Choice | Rationale |
|----------|--------|-----------|
| **Log Model** | Structured Log + Query Layer | SQLite stores events, CLI provides queries. Enables audit trail AND active workflows. |
| **Retry UX** | Hybrid Auto-Retry + Manual Escalation | Auto-retry transient errors. Surface permanent/auth errors for user action. |

```rust
enum FailureType {
    Transient,   // Network timeout, 5xx → auto-retry
    Permanent,   // 404, 410 → mark failed
    NeedsAuth,   // 401, 403 → prompt user
    RateLimited, // 429 → respect Retry-After
}

struct RetryPolicy {
    max_attempts: u32,        // 3
    backoff_base: Duration,   // 5s
    backoff_max: Duration,    // 5min
    backoff_multiplier: f32,  // 2.0
}
```

### Resilience & Crash Safety

| Decision | Choice | Rationale |
|----------|--------|-----------|
| **Panic Handling** | Catch at task level, log, continue | "Never crash" - one bad resolver doesn't kill the queue. Isolate failures per download. |
| **Graceful Shutdown** | SIGINT handler with queue checkpoint | Ctrl+C saves queue state, allows clean resume. |

```rust
// Task-level panic isolation
async fn process_download(item: QueueItem) -> DownloadResult {
    let result = tokio::task::spawn(async move {
        std::panic::catch_unwind(AssertUnwindSafe(|| {
            download_item_inner(item).await
        }))
    }).await;

    match result {
        Ok(Ok(success)) => success,
        Ok(Err(e)) => {
            log_error(&e);
            DownloadResult::Failed(FailureType::Permanent, e.to_string())
        },
        Err(panic) => {
            log_panic(&panic);
            DownloadResult::Failed(FailureType::InternalError, "Internal error - see logs")
        }
    }
}

// Graceful shutdown
async fn run_with_shutdown(queue: Queue) {
    let ctrl_c = tokio::signal::ctrl_c();

    tokio::select! {
        _ = queue.process_all() => {},
        _ = ctrl_c => {
            info!("Shutdown requested, saving queue state...");
            queue.checkpoint().await?;
            info!("Queue saved. Safe to exit.");
        }
    }
}
```

### Testing Infrastructure

| Decision | Choice | Rationale |
|----------|--------|-----------|
| **Database Testing** | In-memory for unit, temp file for integration | `:memory:` is fast for unit tests. Temp files test real I/O for integration. |
| **Keychain Testing** | KeyStorage enum abstraction | Allows InMemory/Environment modes for CI where OS keychain unavailable. |
| **Time Control** | Clock trait abstraction | Testable retry/backoff without real delays. MockClock advances instantly. |
| **HTTP Mocking** | wiremock for resolver tests | No external network in tests. Predictable, fast, reproducible. |

```rust
// Clock abstraction for testable timing
trait Clock: Send + Sync {
    fn now(&self) -> Instant;
    async fn sleep(&self, duration: Duration);
}

struct SystemClock;
impl Clock for SystemClock {
    fn now(&self) -> Instant { Instant::now() }
    async fn sleep(&self, duration: Duration) { tokio::time::sleep(duration).await }
}

#[cfg(test)]
struct MockClock {
    current: AtomicU64,
}

#[cfg(test)]
impl Clock for MockClock {
    fn now(&self) -> Instant { /* return based on current */ }
    async fn sleep(&self, duration: Duration) {
        self.current.fetch_add(duration.as_millis() as u64, Ordering::SeqCst);
        // Returns immediately - no actual waiting
    }
}

// Test database helper
#[cfg(test)]
fn test_db() -> Database {
    Database::open_in_memory().expect("test db")
}
```

### Decision Impact Analysis

**Implementation Sequence:**
1. SQLite schema and database module (foundation for everything)
2. Resolver trait and registry (enables download engine)
3. Download engine with concurrency control
4. Queue manager with persistence
5. Cookie manager with encryption
6. Retry handler with failure categorization
7. CLI commands with log queries

**Cross-Component Dependencies:**
```
SQLite Database (WAL mode, event batching)
    ↑
    ├── Metadata Storage (envelopes)
    ├── Queue Persistence
    ├── Event Log (structured, batched)
    └── Resolver Metrics

Resolver Registry
    ↑
    └── Download Engine (Resolution Loop)
            ↑
            ├── Concurrency Manager (per-domain semaphores)
            ├── Cookie Manager (with debug flag)
            ├── Retry Handler (Clock trait for testing)
            └── Panic Isolation (task-level catch)
```

## Implementation Patterns & Consistency Rules

These patterns ensure all AI agents and developers write compatible, consistent code.

### Quick Reference Card

| Thing | Convention |
|-------|------------|
| File names | `snake_case.rs` |
| Structs/Enums | `PascalCase` |
| Functions | `snake_case` |
| Constants | `SCREAMING_SNAKE_CASE` |
| DB tables | `snake_case` plural |
| DB columns | `snake_case` |
| JSON fields | `snake_case` |
| Config format | TOML |
| Tests | Inline `#[cfg(test)]` + `tests/` |
| Errors | `thiserror` (lib), `anyhow` (bin) |
| Logging | `tracing` with instrument spans |

### Naming Patterns

| Element | Convention | Example |
|---------|------------|---------|
| Rust modules/files | `snake_case` | `download_engine.rs` |
| Rust structs/enums | `PascalCase` | `DownloadResult` |
| Rust functions | `snake_case` | `resolve_to_url()` |
| Rust constants | `SCREAMING_SNAKE_CASE` | `MAX_REDIRECTS` |
| Database tables | `snake_case` plural | `downloads`, `queue_items` |
| Database columns | `snake_case` | `source_url`, `created_at` |
| JSON fields | `snake_case` | `{"download_id": "..."}` |
| Config keys | `snake_case` | `max_concurrent` |

**Rationale:** Consistent snake_case across Rust, database, and JSON eliminates serde rename boilerplate and reduces cognitive load.

### Import Organization

```rust
// 1. Standard library
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

// 2. External crates (alphabetized)
use reqwest::Client;
use serde::{Deserialize, Serialize};
use tokio::sync::Semaphore;
use tracing::{debug, info, instrument};

// 3. Internal modules (crate::)
use crate::resolver::Registry;
use crate::storage::Database;
```

**Rule:** Groups separated by blank line. Alphabetized within each group.

### Error Handling Patterns

**Module-Specific Errors with Unified Library Type:**

```rust
// Each module defines its own error type
pub mod resolver {
    #[derive(Debug, thiserror::Error)]
    pub enum Error {
        #[error("DOI not found: {0}")]
        DoiNotFound(String),
        #[error("Network error: {0}")]
        Network(#[from] reqwest::Error),
    }
}

// Library root unifies them
pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error(transparent)]
    Resolver(#[from] resolver::Error),
    #[error(transparent)]
    Download(#[from] download::Error),
    #[error(transparent)]
    Storage(#[from] storage::Error),
}
```

**Context Addition (Binary Only):**

```rust
// In main.rs / CLI code only
use anyhow::Context;

let result = resolver.resolve(&url)
    .context(format!("Failed to resolve {}", url))?;
```

**Rule:** Library code uses thiserror only. Binary code wraps with anyhow for user-facing context.

### Async Patterns

| Pattern | When to Use | Example |
|---------|-------------|---------|
| `await` inline | Sequential operations, need result | `let data = fetch(url).await?;` |
| `tokio::spawn` | Fire-and-forget, parallelism | `tokio::spawn(log_event(e));` |
| `tokio::select!` | Racing operations, cancellation | Shutdown handling |
| `join!` | Parallel operations, need all results | Multiple independent fetches |

```rust
// Sequential: need result before continuing
let metadata = fetch_metadata(doi).await?;
let pdf_url = extract_pdf_url(&metadata)?;

// Parallel: independent operations
let (meta, cookies) = tokio::join!(
    fetch_metadata(doi),
    load_cookies(domain)
);

// Fire-and-forget: logging, metrics
tokio::spawn(async move {
    db.record_event(event).await.ok(); // Ignore result
});
```

### Module & Test Structure

**Test Location Pattern:**

```
src/
├── resolver/
│   ├── mod.rs          # Re-exports: pub use doi::DoiResolver;
│   ├── doi.rs          # Contains #[cfg(test)] mod tests { ... }
│   └── direct.rs
tests/
├── common/
│   └── mod.rs          # Shared test utilities
├── fixtures/
│   ├── valid_doi_response.json
│   └── sciencedirect_page.html
├── resolver_integration.rs
└── cli_tests.rs
```

**Module Export Pattern:**

```rust
// src/resolver/mod.rs
mod doi;
mod direct;
mod registry;

// Flat re-exports for clean API
pub use doi::DoiResolver;
pub use direct::DirectResolver;
pub use registry::{Registry, Resolver, ResolverPriority, ResolveStep};
```

**Rule:** Internal structure hidden. Public API is flat exports from mod.rs.

### Test Patterns

**Test Naming Convention:** `test_<unit>_<scenario>_<expected>`

```rust
#[test]
fn test_doi_resolver_valid_doi_returns_url() { }

#[test]
fn test_doi_resolver_invalid_doi_returns_error() { }

#[test]
fn test_queue_empty_queue_returns_none() { }
```

**Assertion Style:**

```rust
// Good: specific, clear failure message
assert_eq!(result.status, DownloadStatus::Complete);
assert_eq!(downloads.len(), 3);

// Good: for enum variants
assert!(matches!(step, ResolveStep::Url(_)));
assert!(matches!(error, Error::Resolver(resolver::Error::DoiNotFound(_))));

// Avoid: unclear on failure
assert!(result.is_ok());  // What was the error?
```

**Test Fixtures:**

```rust
// tests/support/mod.rs
pub fn load_fixture(name: &str) -> String {
    std::fs::read_to_string(format!("tests/fixtures/{}", name))
        .expect("fixture should exist")
}

// Usage in tests
use crate::support::load_fixture;

#[test]
fn test_parser_handles_crossref_response() {
    let json = load_fixture("valid_doi_response.json");
    let result = parse_crossref_response(&json);
    assert!(result.is_ok());
}
```

### Logging Patterns

**Log Level Definitions:**

| Level | Usage | CLI Visibility |
|-------|-------|----------------|
| `error!` | Unrecoverable failures affecting user outcome | Always shown |
| `warn!` | Recoverable issues, retries, degraded behavior | Always shown |
| `info!` | Major state changes, user-relevant progress | Default |
| `debug!` | Developer-relevant details | `--verbose` |
| `trace!` | Very verbose, per-iteration details | `--debug` |

**Structured Logging with Spans:**

```rust
#[tracing::instrument(fields(url = %url, resolver = %self.name()))]
fn resolve(&self, url: &str) -> Result<ResolveStep> {
    debug!("Starting resolution");
    let metadata = self.fetch_metadata(url)?;
    info!("Resolution complete");
    Ok(ResolveStep::Url(metadata.pdf_url))
}
```

**Rule:** Use `#[tracing::instrument]` on public functions. Fields defined once, inherited by all nested operations.

### CLI Output Patterns

**Progress Display:**

```
// Default mode: Status line + summary
⠋ Downloading 3 files... (2 queued, 5 complete)
  └─ paper.pdf (1.2 MB/s)
Completed: 5 | Failed: 1 | Remaining: 7

// Verbose mode (--verbose): Multi-bar per file
[1/25] paper.pdf      [████████████░░░░] 75%
[2/25] article.pdf    [██░░░░░░░░░░░░░░] 12%
```

**Error Display Format:**

```
// MVP: Clear actionable messages
Error: Cannot access https://sciencedirect.com/paper.pdf
  This site requires authentication.
  Run `downloader auth capture` to log in via browser.

// v1.1 Polish: Add error codes for scripting
Error [AUTH_REQUIRED]: Cannot access ...
```

**Rule:** All user-facing errors include actionable suggestions when applicable. Error codes added in v1.1.

### Configuration Patterns

**Format:** TOML

**File Locations:**
```
~/.config/downloader/config.toml   # User config
./.downloader/config.toml          # Project config
```

**Hierarchy (later overrides earlier):**
1. Compiled defaults
2. User config
3. Project config
4. CLI flags

**MVP Simplification:** Later config fully overrides earlier (no deep merge). Deep merge semantics deferred to v1.1 if needed.

### MVP vs Polish

| Pattern | MVP Required | v1.1 Polish |
|---------|--------------|-------------|
| Error messages with suggestions | ✅ | - |
| Error codes (`[AUTH_REQUIRED]`) | - | ✅ |
| Config file support | ✅ | - |
| Config deep merge | - | ✅ |
| Multi-bar progress | - | ✅ (verbose mode) |

### When in Doubt

- **Naming unclear?** → Match existing code in same module
- **Test type unclear?** → Unit if testing one function, integration if cross-module
- **Log level unclear?** → Use `debug!`, promote to `info!` later if needed
- **Config vs hardcode?** → If it might vary between users, make it config
- **Spawn vs await?** → `await` unless you explicitly don't need the result

### Enforcement Guidelines

**All AI Agents MUST:**

1. Follow Rust RFC 430 naming conventions exactly
2. Organize imports: std → external → internal, alphabetized
3. Use thiserror for library errors, anyhow only in binary
4. Place unit tests inline with `#[cfg(test)]`, integration tests in `tests/`
5. Name tests: `test_<unit>_<scenario>_<expected>`
6. Use `#[tracing::instrument]` on public functions
7. Include actionable suggestions in user-facing errors
8. Use TOML for configuration files

**Code Review Checklist:**

- [ ] Names follow RFC 430?
- [ ] Imports organized (std → external → internal)?
- [ ] Errors use thiserror (lib) / anyhow (bin)?
- [ ] Unit tests inline with code?
- [ ] Test names follow convention?
- [ ] Public functions have `#[tracing::instrument]`?
- [ ] User errors have actionable messages?

### Anti-Patterns to Avoid

| Anti-Pattern | Correct Pattern |
|--------------|-----------------|
| `userId` in JSON | `user_id` (snake_case) |
| Random import order | std → external → internal |
| `panic!()` for recoverable errors | Return `Result<T, Error>` |
| Tests only in `tests/` | Unit tests inline with `#[cfg(test)]` |
| `test_it_works()` | `test_resolver_valid_input_returns_url()` |
| `println!()` for logging | `info!()`, `debug!()`, etc. |
| `assert!(x.is_ok())` | `assert_eq!(x, expected)` or check error |
| Hardcoded config values | Config file with defaults |

## Project Structure & Boundaries

### Complete Project Directory Structure

```
downloader/
├── src/
│   ├── lib.rs                        # Library root: pub mod declarations + pub use re-exports
│   ├── main.rs                       # CLI entry point (anyhow, single #[tokio::main])
│   ├── cli.rs                        # clap derive-style argument definitions
│   ├── app_config.rs                 # App configuration (replaces planned src/config/)
│   ├── db.rs                         # Database connection + migrations (replaces planned src/storage/)
│   ├── user_agent.rs                 # User-Agent string construction
│   │
│   ├── bin/                          # Additional binary targets
│   │   ├── extract_md_links.rs
│   │   └── stress_sidecar_flaky.rs
│   │
│   ├── app/                          # Application orchestration layer
│   │   ├── mod.rs
│   │   ├── command_dispatcher.rs
│   │   ├── config_manager.rs
│   │   ├── config_runtime.rs
│   │   ├── context.rs
│   │   ├── download_orchestrator.rs
│   │   ├── exit_handler.rs
│   │   ├── input_processor.rs
│   │   ├── progress_manager.rs
│   │   ├── queue_manager.rs
│   │   ├── resolution_orchestrator.rs
│   │   ├── runtime.rs
│   │   ├── terminal.rs
│   │   └── validation.rs
│   │
│   ├── commands/                     # CLI command implementations
│   │   ├── mod.rs
│   │   ├── auth.rs
│   │   ├── config.rs
│   │   ├── dry_run.rs
│   │   ├── log.rs
│   │   └── search.rs
│   │
│   ├── auth/                         # Authentication & cookie management
│   │   ├── mod.rs
│   │   ├── capture.rs
│   │   ├── cookies.rs
│   │   ├── runtime_cookies.rs
│   │   └── storage.rs                # KeyStorage enum (OsKeychain/InMemory/Environment)
│   │
│   ├── download/                     # Download engine
│   │   ├── mod.rs
│   │   ├── client.rs
│   │   ├── constants.rs
│   │   ├── error.rs                  # Module-local error type
│   │   ├── filename.rs
│   │   ├── rate_limiter.rs
│   │   ├── retry.rs
│   │   ├── robots.rs
│   │   ├── engine.rs                 # Engine module root (Rust 2018 style: engine.rs + engine/)
│   │   └── engine/
│   │       ├── error_mapping.rs
│   │       ├── persistence.rs
│   │       └── task.rs
│   │
│   ├── failure/                      # Failure taxonomy
│   │   └── mod.rs
│   │
│   ├── output/                       # CLI output formatting
│   │   └── mod.rs
│   │
│   ├── parser/                       # Input parsing
│   │   ├── mod.rs
│   │   ├── bibliography.rs
│   │   ├── bibtex.rs
│   │   ├── doi.rs
│   │   ├── error.rs                  # Module-local error type
│   │   ├── input.rs
│   │   ├── reference.rs
│   │   └── url.rs
│   │
│   ├── project/                      # Project directory management
│   │   └── mod.rs
│   │
│   ├── queue/                        # Download queue + history
│   │   ├── mod.rs
│   │   ├── error.rs                  # Module-local error type
│   │   ├── history.rs
│   │   ├── item.rs
│   │   └── repository.rs
│   │
│   ├── resolver/                     # Resolver trait + registry + site resolvers
│   │   ├── mod.rs
│   │   ├── arxiv.rs
│   │   ├── crossref.rs               # Crossref DOI resolution
│   │   ├── direct.rs
│   │   ├── error.rs                  # Module-local error type
│   │   ├── http_client.rs
│   │   ├── ieee.rs
│   │   ├── pubmed.rs
│   │   ├── registry.rs               # build_default_resolver_registry()
│   │   ├── sciencedirect.rs
│   │   ├── springer.rs
│   │   └── utils.rs
│   │
│   ├── search/                       # Past-download search
│   │   └── mod.rs
│   │
│   ├── sidecar/                      # JSON-LD sidecar generation
│   │   └── mod.rs
│   │
│   ├── test_support/                 # In-lib test utilities
│   │   ├── mod.rs
│   │   └── socket_guard.rs
│   │
│   └── topics/                       # Topic extraction + normalization
│       ├── mod.rs
│       ├── extractor.rs
│       └── normalizer.rs
│
├── tests/
│   ├── auth_integration.rs
│   ├── cli_e2e.rs
│   ├── critical.rs                   # Entry point for adversarial suite
│   ├── critical/                     # Failure-mode tests (auth bypass, encryption, races, etc.)
│   ├── download_engine_integration.rs
│   ├── download_integration.rs
│   ├── exit_code_partial_e2e.rs
│   ├── integration_matrix.rs
│   ├── nonfunctional_regression_gates.rs
│   ├── optimization_refactor_commands.rs
│   ├── parser_integration.rs
│   ├── queue_integration.rs
│   ├── resolver_integration.rs
│   └── support/                      # Shared test utilities (was tests/common/)
│       ├── mod.rs
│       ├── critical_utils.rs
│       └── socket_guard.rs
│
├── .github/
│   └── workflows/
│       ├── phase-rollout-gates.yml   # Build, test, clippy, fmt, audit
│       ├── coverage.yml              # cargo llvm-cov coverage reporting
│       └── stress-sidecar-flaky.yml  # Sidecar stress test
│
├── Cargo.toml                        # Dependencies and lib/bin config
├── Cargo.lock                        # Locked dependency versions
├── rust-toolchain.toml               # Rust version specification
├── rustfmt.toml                      # Formatter configuration
├── clippy.toml                       # Linter configuration
├── .cargo/audit.toml                 # Accepted security advisories
├── .gitignore
├── LICENSE
└── README.md
```

### Error Module Structure

Each module defines its own error type using `thiserror`. There is no unified `src/error.rs` — errors stay module-local and are converted at module boundaries via `From` impls.

```rust
// Pattern used in each module, e.g. src/resolver/error.rs
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ResolveError {
    #[error("resolution failed: {0}")]
    ResolutionFailed(String),
    // ...
}

// src/download/error.rs, src/queue/error.rs, src/parser/error.rs follow the same pattern
```

**Error handling boundary (enforced in `src/lib.rs`):**
- Library code (`src/`): `thiserror` with module-scoped error enums
- Binary code (`main.rs`, `cli.rs`): `anyhow` for ergonomic propagation
- `#[deny(clippy::expect_used)]` enforced at lib level — no `.expect()` in library code

### Module Ownership Mapping

| Directory | Primary Epic | Dependencies | Notes |
|-----------|-------------|--------------|-------|
| `src/app_config.rs` | Infrastructure | None | App configuration; replaces planned `src/config/` |
| `src/db.rs` | Infrastructure | None | SQLite connection + migrations; replaces planned `src/storage/` |
| `src/user_agent.rs` | Infrastructure | None | User-Agent string construction |
| `src/app/` | Orchestration | download, resolver, parser, queue, auth, db | Application orchestration layer; all command dispatch |
| `src/commands/` | CLI | app, auth, queue, search | Per-command implementations (auth, config, dry_run, log, search) |
| `src/parser/` | Input Parsing | None | Pure parsing, no I/O |
| `src/resolver/` | Resolver System | parser, auth | Trait + registry + all site resolvers (flat, no `sites/` subdir) |
| `src/download/` | Core Download | auth, db | HTTP operations; `engine/` submodule for task lifecycle |
| `src/queue/` | Core Download | db | Task orchestration + download history |
| `src/auth/` | Authentication | db | Cookie management; `storage.rs` holds `KeyStorage` enum |
| `src/failure/` | Reliability | queue | Failure taxonomy and categorization |
| `src/project/` | Organization | db | Project directory management; was planned in `src/storage/` |
| `src/search/` | History | db | Past-download search |
| `src/sidecar/` | Metadata | parser, topics | JSON-LD sidecar generation |
| `src/topics/` | Metadata | parser | Topic extraction + normalization |
| `src/output/` | CLI | None | Display formatting |
| `src/test_support/` | Testing | None | In-lib test utilities; replaces planned `tests/common/` |

### Architectural Boundaries

**Library Boundary (`src/lib.rs`):**
```rust
// Public API exposed by downloader_core (as-built)
pub mod auth;
pub mod db;
pub mod download;
pub mod parser;
pub mod queue;
pub mod resolver;
pub mod sidecar;
#[cfg(test)]
pub mod test_support;    // In-lib test utilities; cfg(test) only
pub mod topics;
pub(crate) mod user_agent;  // Internal only

// Convenience re-exports (selected — see src/lib.rs for full list)
pub use db::{Database, DatabaseOptions};
pub use parser::{Confidence, ParsedItem, ParseResult, parse_input};
pub use resolver::{Resolver, ResolveStep, ResolverRegistry, build_default_resolver_registry};
pub use download::{DownloadEngine, QueueProcessingOptions};
pub use queue::{Queue, QueueItem, QueueError};
pub use sidecar::{SidecarConfig, generate_sidecar};
pub use topics::{TopicExtractor, extract_keywords};
```

**Dependency Direction (No Cycles):**
```
main.rs (binary)
    │
    └── cli.rs
            │
            ├── config
            ├── parser
            ├── queue ─────────────┐
            ├── download ──────────┤
            │       │              │
            │       └── resolver ──┤
            │               │      │
            │               └──────┼── auth
            │                      │     │
            └── output             └─────┴── storage
                                              │
                                         error (all modules)
```

### Data Flow Architecture

**Input Processing Flow:**
```
User Input (stdin/file/args)
    │
    ▼
┌─────────────────────┐
│  parser::parse()    │  → ParsedInput { input_type, raw }
└─────────────────────┘
    │
    ▼
┌─────────────────────┐
│  queue::enqueue()   │  → QueueItem stored in SQLite
└─────────────────────┘
    │
    ▼
┌─────────────────────┐
│  queue::process()   │  → Spawns download tasks
└─────────────────────┘
    │
    ▼
┌─────────────────────┐
│ resolver::resolve() │  → ResolveStep::Url | NeedsAuth | Failed
└─────────────────────┘
    │
    ▼
┌─────────────────────┐
│ download::fetch()   │  → Stream to file
└─────────────────────┘
    │
    ▼
┌─────────────────────┐
│ storage::record()   │  → Metadata envelope in SQLite
└─────────────────────┘
```

**Authentication Flow:**
```
Browser Extension
    │
    ▼ (native messaging)
┌─────────────────────┐
│ auth::capture()     │  → Receive cookies
└─────────────────────┘
    │
    ▼
┌─────────────────────┐
│ auth::storage       │  → Encrypt and persist (optional)
└─────────────────────┘
    │
    ▼
┌─────────────────────┐
│ download::client    │  → Attach cookies to requests
└─────────────────────┘
```

### File Organization Patterns

**Configuration Files:**
```
~/.config/downloader/
├── config.toml           # User configuration
└── cookies.enc           # Encrypted cookie storage (opt-in)

./.downloader/            # Project-level (optional)
├── config.toml           # Project configuration
└── cookies.enc           # Project-specific cookies
```

**Data Files:**
```
~/.local/share/downloader/
└── downloader.db         # SQLite database (metadata, queue, logs)

./output_dir/             # User-specified or default Downloads/
├── Author_2024_Title.pdf
└── .downloader/
    └── project.db        # Project-specific database (if project mode)
```

### CLI Command Structure

```
downloader
├── (default)             # Read from stdin, download to current dir
├── download <inputs...>  # Download specified URLs/DOIs
├── project               # Project management
│   ├── init              # Initialize project in current dir
│   ├── status            # Show project download status
│   └── retry             # Retry failed downloads
├── auth                  # Authentication management
│   ├── capture           # Start cookie capture from browser
│   ├── status            # Show auth status per domain
│   └── clear             # Clear stored credentials
├── log                   # Query download history
│   ├── show              # Show recent activity
│   ├── search            # Search by URL, title, status
│   └── export            # Export to JSON/CSV
├── config                # Configuration management
│   ├── show              # Show effective configuration
│   ├── edit              # Open config in editor
│   └── reset             # Reset to defaults
└── version               # Show version info
```

### Test Utilities

```rust
// tests/support/mod.rs  (was planned as tests/common/mod.rs)
// src/test_support/mod.rs  — in-lib utilities for unit tests

use downloader::db::Database;

/// Create an in-memory database for unit tests (see src/test_support/mod.rs for actual helper)
pub fn test_db() -> Database {
    Database::open_in_memory().expect("test database should initialize")
}

/// Create a database with seed data for integration tests
pub fn test_db_with_fixtures() -> Database {
    let db = test_db();
    db.execute(include_str!("../fixtures/seed_data.sql"))
        .expect("seed data should load");
    db
}

/// Load a fixture file as string
pub fn load_fixture(name: &str) -> String {
    std::fs::read_to_string(format!("tests/fixtures/{}", name))
        .expect("fixture file should exist")
}
```

### Development Workflow Integration

**Development Commands:**
```bash
cargo build --lib              # Build library only
cargo build --bin downloader   # Build CLI binary
cargo test                     # Run all tests
cargo test --lib               # Library tests only
cargo clippy                   # Lint check
cargo fmt                      # Format code
```

**CI Pipeline (`.github/workflows/phase-rollout-gates.yml`):**
```yaml
jobs:
  check:
    - cargo fmt --check
    - cargo clippy -- -D warnings
    - cargo test --all-features
  build:
    - cargo build --release
    matrix:
      os: [ubuntu-latest, macos-latest, windows-latest]
```

## Architecture Validation Results

### Coherence Validation

| Check | Status | Notes |
|-------|--------|-------|
| No circular dependencies | PASS | Dependency graph is DAG |
| Error handling consistent | PASS | thiserror in lib, anyhow in bin |
| Naming conventions uniform | PASS | RFC 430 throughout |
| Configuration hierarchy clear | PASS | defaults → user → project → CLI |
| Module boundaries respected | PASS | Clear public API in lib.rs |
| Test strategy coherent | PASS | Unit inline, integration in tests/ |

### Requirements Coverage

**Functional Requirements (19/19 covered):**

| FR Category | Requirements | Covered By |
|-------------|--------------|------------|
| Input Parsing (6) | URL, DOI, reference, bibliography, batch, validation | `parser/` module |
| Download Engine (7) | HTTP, auth, resolvers, retry, concurrency, rate limit, resume | `download/`, `resolver/`, `queue/` |
| Organization (6) | Projects, naming, indexing, topics, metadata, dedup | `project/`, `sidecar/`, `topics/`, `app_config.rs` |
| Logging & Memory (5) | Download log, failure tracking, queryable history | `queue/history.rs`, `failure/`, SQLite via `db.rs` |
| CLI Interface (6) | stdin, flags, progress, summary, dry-run, config | `cli.rs`, `output/` |

**Non-Functional Requirements (10/10 covered):**

| NFR | Target | Solution |
|-----|--------|----------|
| Parse 150 refs < 5s | Performance | Async parser, no I/O blocking |
| 10 concurrent downloads | Performance | Semaphore-controlled queue |
| 90% success rate | Reliability | Retry with backoff, fallback resolvers |
| Never crash on bad input | Reliability | Result<T> everywhere, no panic |
| Zero-config start | Usability | Compiled defaults, sensible paths |
| Cross-platform | Portability | Pure Rust, dirs crate for paths |
| Modular resolvers | Maintainability | Resolver trait + registry pattern |
| Extensible authentication | Maintainability | KeyStorage enum, pluggable storage |
| Queryable history | Usability | SQLite with structured events |
| Graceful degradation | Reliability | Per-item failure isolation |

### Implementation Readiness

| Aspect | Status | Evidence |
|--------|--------|----------|
| Clear module boundaries | READY | Directory structure and ownership table |
| Defined public APIs | READY | lib.rs exports documented |
| Error handling strategy | READY | Unified Error enum with From impls |
| Test infrastructure | READY | test_db(), fixtures, common utilities |
| Configuration system | READY | TOML format, merge hierarchy |
| Dependency list | READY | Cargo.toml dependencies defined |

### SQLite Schema Overview

```sql
-- Core tables for MVP

CREATE TABLE downloads (
    id INTEGER PRIMARY KEY,
    url TEXT NOT NULL,
    doi TEXT,
    title TEXT,
    status TEXT NOT NULL DEFAULT 'pending',  -- pending, active, complete, failed
    file_path TEXT,
    file_hash TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now')),
    metadata JSON  -- JSON-LD envelope
);

CREATE TABLE queue_items (
    id INTEGER PRIMARY KEY,
    download_id INTEGER NOT NULL REFERENCES downloads(id),
    priority INTEGER NOT NULL DEFAULT 0,
    retry_count INTEGER NOT NULL DEFAULT 0,
    next_retry_at TEXT,
    domain TEXT NOT NULL,
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE TABLE events (
    id INTEGER PRIMARY KEY,
    download_id INTEGER REFERENCES downloads(id),
    event_type TEXT NOT NULL,  -- started, progress, completed, failed, retried
    details JSON,
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);

-- Note: cookies are NOT stored in SQLite.
-- Cookie data is stored in an XChaCha20Poly1305-encrypted file.
-- The master encryption key is held in the OS keychain (macOS Keychain /
-- Windows Credential Manager). See src/auth/storage.rs — KeyStorage enum.
-- This is a better security decision: cookie data never transits SQLite unencrypted.

-- Indexes for common queries
CREATE INDEX idx_downloads_status ON downloads(status);
CREATE INDEX idx_downloads_doi ON downloads(doi);
CREATE INDEX idx_queue_priority ON queue_items(priority DESC, created_at ASC);
CREATE INDEX idx_queue_domain ON queue_items(domain);
CREATE INDEX idx_events_download ON events(download_id);
```

### Test Coverage Guidelines

**Target: 80%+ line coverage for library code**

| Module | Priority | Coverage Target | Focus Areas |
|--------|----------|-----------------|-------------|
| `parser/` | HIGH | 90%+ | All input formats, edge cases |
| `resolver/` | HIGH | 85%+ | Each resolver, fallback chains |
| `auth/` | HIGH | 85%+ | Cookie handling, encryption |
| `download/` | MEDIUM | 80%+ | Retry logic, error handling |
| `queue/` | MEDIUM | 80%+ | Concurrency, persistence, history |
| `db.rs` | MEDIUM | 80%+ | Migrations, WAL mode, query correctness |
| `sidecar/` | MEDIUM | 75%+ | JSON-LD generation, field mapping |
| `topics/` | MEDIUM | 75%+ | Extraction, normalization |
| `search/` | MEDIUM | 75%+ | Query correctness, ranking |
| `app_config.rs` | LOW | 70%+ | Merge logic, defaults |
| `output/` | LOW | 60%+ | Format correctness |

**Critical Paths Requiring Integration Tests:**
1. DOI → Crossref → publisher → PDF (happy path)
2. Auth-required site with cookie capture
3. Queue persistence across restart
4. Retry after transient failure
5. Concurrent downloads with rate limiting

### Architecture Completeness Checklist

- [x] All PRD functional requirements mapped to modules
- [x] All PRD non-functional requirements have technical solutions
- [x] Error handling strategy defined and consistent
- [x] Configuration system designed with clear hierarchy
- [x] Test strategy covers unit, integration, and E2E
- [x] Module dependencies form acyclic graph
- [x] Public API surface documented
- [x] Database schema supports all persistence needs
- [x] CLI command structure supports all user workflows
- [x] CI/CD pipeline defined

### Recommended First Stories

Based on dependency analysis, optimal story sequence for first sprint:

| Order | Story | Rationale |
|-------|-------|-----------|
| 1 | Project scaffolding | Creates Cargo.toml, directory structure, CI |
| 2 | Config module | Zero external dependencies, enables all other modules |
| 3 | Parser module | Pure functions, no I/O, enables resolver testing |
| 4 | Storage foundation | SQLite setup, schema, enables queue and logs |
| 5 | Direct URL download | Minimal resolver, proves download pipeline |

### Implementation Handoff

**For AI Agents Starting Implementation:**

1. **Read this document completely** before writing any code
2. **Follow the module ownership table** to understand boundaries
3. **Use the error patterns** exactly as specified
4. **Run `cargo clippy` and `cargo fmt`** before every commit
5. **Write tests first** per red-green-refactor cycle
6. **Reference PRD** for user-facing behavior details

**Quick Reference Card:**

| When You Need | Look At |
|---------------|---------|
| Error handling | Error Module Structure section |
| File naming | RFC 430 conventions |
| Database queries | SQLite Schema Overview |
| Test utilities | Test Utilities section |
| Module dependencies | Dependency Direction diagram |
| Configuration | Configuration Patterns section |

### Quick Start for AI Agents

```bash
# Verify project setup
cargo check
cargo clippy -- -D warnings
cargo fmt --check
cargo test

# Development cycle
1. Read the story file for exact requirements
2. Identify target module from ownership table
3. Write failing test first
4. Implement to pass test
5. Run clippy + fmt
6. Mark subtask complete
```

**Key Files to Reference:**
- `src/lib.rs` - All public exports live here; module-local `error.rs` files own each module's error type
- `src/app_config.rs` - App configuration (replaces planned `src/config/`)
- `src/db.rs` - Database connection + migrations (replaces planned `src/storage/`)
- `src/test_support/mod.rs` - Use `test_db()` and `SocketGuard` for tests (replaces planned `tests/common/`)
- `tests/support/mod.rs` - Shared integration test utilities

## Architecture Completion Summary

### Workflow Completion

**Architecture Decision Workflow:** COMPLETED
**Total Steps Completed:** 8
**Date Completed:** 2026-01-26
**Document Location:** `_bmad-output/planning-artifacts/architecture.md`

### Final Architecture Deliverables

**Complete Architecture Document**
- All architectural decisions documented with specific versions
- Implementation patterns ensuring AI agent consistency
- Complete project structure with all files and directories
- Requirements to architecture mapping
- Validation confirming coherence and completeness

**Implementation Ready Foundation**
- 25+ architectural decisions made
- 15+ implementation patterns defined
- 10 major architectural components specified
- 29 requirements fully supported (19 FR + 10 NFR)

**AI Agent Implementation Guide**
- Technology stack: Rust + Tokio + reqwest + SQLite (sqlx)
- Consistency rules that prevent implementation conflicts
- Project structure with clear boundaries
- Integration patterns and communication standards

### Implementation Handoff

**For AI Agents:**
This architecture document is your complete guide for implementing Downloader. Follow all decisions, patterns, and structures exactly as documented.

**First Implementation Priority:**
```bash
cargo new downloader --lib
# Then set up lib/bin split per Cargo.toml structure
```

**Development Sequence:**
1. Initialize project using documented starter template
2. Set up development environment per architecture
3. Implement core architectural foundations (db.rs, app_config.rs, module-local error types)
4. Build features following established patterns
5. Maintain consistency with documented rules

### Quality Assurance Checklist

**Architecture Coherence**
- [x] All decisions work together without conflicts
- [x] Technology choices are compatible
- [x] Patterns support the architectural decisions
- [x] Structure aligns with all choices

**Requirements Coverage**
- [x] All functional requirements are supported
- [x] All non-functional requirements are addressed
- [x] Cross-cutting concerns are handled
- [x] Integration points are defined

**Implementation Readiness**
- [x] Decisions are specific and actionable
- [x] Patterns prevent agent conflicts
- [x] Structure is complete and unambiguous
- [x] Examples are provided for clarity

### Project Success Factors

**Clear Decision Framework**
Every technology choice was made collaboratively with clear rationale, ensuring all stakeholders understand the architectural direction.

**Consistency Guarantee**
Implementation patterns and rules ensure that multiple AI agents will produce compatible, consistent code that works together seamlessly.

**Complete Coverage**
All project requirements are architecturally supported, with clear mapping from business needs to technical implementation.

**Solid Foundation**
The chosen lib/bin split architecture and established patterns provide a production-ready foundation following Rust best practices.

---

**Architecture Status:** READY FOR IMPLEMENTATION

**Next Phase:** Begin implementation using the architectural decisions and patterns documented herein.

**Document Maintenance:** Update this architecture when major technical decisions are made during implementation.

