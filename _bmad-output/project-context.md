---
project_name: 'Downloader'
user_name: 'fierce'
date: '2026-01-26'
sections_completed: ['technology_stack', 'language_rules', 'framework_rules', 'testing_rules', 'code_quality', 'workflow_rules', 'critical_rules']
status: 'complete'
rule_count: 85
optimized_for_llm: true
---

# Project Context for AI Agents

_This file contains critical rules and patterns that AI agents must follow when implementing code in this project. Focus on unobvious details that agents might otherwise miss._

---

## Technology Stack & Versions

**Core Runtime:**
- Rust 2024 edition (don't downgrade to 2021 - new language features in use)
- Tokio 1.x with `features = ["full"]` - required, don't optimize away

**HTTP & Network:**
- reqwest 0.13 with `features = ["json", "cookies", "stream", "gzip"]`
- Cookie jar is session-only; persistence is opt-in per architecture

**Database:**
- SQLite via sqlx (async, compile-time query checking)
- Run `cargo sqlx prepare` before commit if queries change

**Serialization:**
- serde 1.x with `features = ["derive"]`
- serde_json 1.x

**Error Handling:**
- thiserror 2.x - Use in library code only
- anyhow 1.x - Use in binary (main.rs, cli.rs) only

**CLI:**
- clap 4.5 with `features = ["derive"]` - derive style exclusively, no builder mixing
- indicatif 0.17 for progress display

**Logging:**
- tracing 0.1 for instrumentation
- tracing-subscriber 0.3 with `features = ["env-filter"]`
- Zero `println!` or `log` crate - tracing only, even in tests

**Dev Dependencies:**
- wiremock 0.6 - async-first, use `.await` patterns from current docs
- tempfile 3.x - use `TempDir::path()`, don't hardcode paths
- tokio-test 0.4 - for manual runtime control only, use `#[tokio::test]` for standard tests

**Feature Flag Discipline:**
- Never add features to dependencies without documenting why
- Each feature increases compile time

## Rust Language Rules

### Error Handling Pattern
- Library code: `thiserror` with module-specific error enums
- Binary code: `anyhow` for ergonomic error propagation
- Never `.unwrap()` or `.expect()` in library code except:
  - Static/compile-time values (regex, config defaults)
  - Test code with descriptive `.expect("reason")`
- Use `?` operator for propagation, not manual matching

### Async Patterns
- Prefer `.await` over `spawn` unless you explicitly don't need the result
- Never block async runtime with `std::thread::sleep` - use `tokio::time::sleep`
- Cancellation-safe: assume any `.await` point may not return
- For CPU-bound work, use `tokio::task::spawn_blocking`

### Module Structure
- `mod.rs` contains only: `mod` declarations, `pub use` re-exports, trait definitions
- Implementation in separate files (e.g., `doi.rs`, `direct.rs`)
- Internal modules are `mod name;`, public API is `pub use name::Thing;`

### Import Organization
```rust
// 1. std library
use std::collections::HashMap;
use std::path::PathBuf;

// 2. External crates (alphabetized)
use reqwest::Client;
use serde::{Deserialize, Serialize};
use tokio::fs;

// 3. Internal crates/modules (alphabetized)
use crate::config::Config;
use crate::error::{Error, Result};
```

### Naming Conventions (RFC 430)
- Types: `PascalCase` (structs, enums, traits)
- Functions/methods: `snake_case`
- Constants: `SCREAMING_SNAKE_CASE`
- Modules: `snake_case`
- No abbreviations in public API (`configuration` not `cfg`)

### Type Patterns
- Use `crate::error::Result<T>` - never define module-local Result aliases
- Accept `&str` or `impl AsRef<str>`, return `String` - never `&String` in signatures
- Trait objects (`Box<dyn Trait>`) for runtime polymorphism (resolvers)
- Generics for compile-time polymorphism (utility functions)

### Ownership & Borrowing
- Let compiler infer lifetimes - only annotate when required
- `.clone()` in loops is a code smell - use references or `Arc`
- Default to borrowing; only take ownership when you need it

### Code Quality Markers
- `#[must_use]` on functions returning `Result` or important values
- Visibility: private by default, `pub(crate)` for internals, `pub` only for API
- Builder pattern for structs with >3 optional fields

### Test-Specific Patterns
- Tests return `Result<(), Error>` - cleaner than `.unwrap()` chains
- `assert_eq!` over `assert!` for better error messages
- `assert!(matches!(value, Pattern))` for enum variant checking
- `#[should_panic(expected = "message")]` for panic tests

## Framework-Specific Rules

### Tokio Async Runtime
- Single runtime: `#[tokio::main]` in main.rs only
- Tests use `#[tokio::test]` - never create manual runtimes in tests
- Prefer `tokio::select!` for concurrent operations with cancellation
- Use `tokio::sync::Semaphore` for concurrency limiting (queue module)

### clap CLI Framework
- All arguments defined via derive macros in `cli.rs`
- Use `#[command(name = "...")]` for subcommands
- Use `#[arg(short, long)]` for flags - always provide both forms
- Validation in clap where possible (value_parser), not post-parse
- Help text is documentation - make it user-friendly

### tracing Logging
- `#[tracing::instrument]` on all public functions
- Skip sensitive fields: `#[instrument(skip(password, cookie))]`
- Use spans for request lifecycle: `info_span!("download", url = %url)`
- Structured fields over string interpolation:
  ```rust
  // Good
  info!(url = %url, status = ?status, "download complete");
  // Bad
  info!("download complete: {} with status {:?}", url, status);
  ```

### reqwest HTTP Client
- Single `Client` instance, reuse across requests (connection pooling)
- Configure timeouts at client level, not per-request
- Use `.error_for_status()` to convert 4xx/5xx to errors
- Stream large downloads: `.bytes_stream()` not `.bytes()`
- **Real-world deployment considerations (CRITICAL):**
  - NEVER use `.no_proxy()` - institutional users require proxy support
  - ALWAYS set a User-Agent - sites block requests without proper identification
  - Pattern: `User-Agent: Downloader/x.y.z (github.com/user/repo)`
  - Test HTTP client configuration against real-world scenarios (proxies, bot detection, rate limits)
  - These are not optional "nice-to-haves" - they are deployment blockers

### sqlx Database
- All queries compile-time checked via `sqlx::query!` macro
- Use `sqlx::query_as!` for mapping to structs
- Transactions via `pool.begin()` - always `.commit()` or auto-rollback
- Migrations in `migrations/` folder, run via `sqlx migrate run`

## Testing Rules

### Test Organization
- Unit tests: inline with code in `#[cfg(test)] mod tests { }` at file bottom
- Integration tests: `tests/` directory, one file per module boundary
- E2E tests: `tests/cli_e2e.rs` for full CLI workflow tests
- Fixtures: `tests/fixtures/` for JSON, HTML, and other test data

### Test Naming Convention
Pattern: `test_<unit>_<scenario>_<expected>`
```rust
#[test]
fn test_doi_resolver_valid_doi_returns_url() { }

#[test]
fn test_doi_resolver_malformed_doi_returns_error() { }

#[test]
fn test_queue_empty_queue_pop_returns_none() { }
```

### Test Utilities
- Use `tests/common/mod.rs` for shared utilities
- `test_db()` - returns in-memory SQLite for unit tests
- `test_db_with_schema()` - runs migrations for integration tests
- `load_fixture("name.json")` - loads from fixtures directory
- Each test gets fresh state - no shared mutable state between tests

### HTTP Mocking (wiremock)
```rust
use wiremock::{MockServer, Mock, ResponseTemplate};
use wiremock::matchers::{method, path};

#[tokio::test]
async fn test_resolver_calls_crossref() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/works/10.1234/example"))
        .respond_with(ResponseTemplate::new(200)
            .set_body_json(load_fixture("crossref_response.json")))
        .mount(&mock_server)
        .await;

    // Test against mock_server.uri()
}
```

### Test Isolation
- Every test runnable in isolation AND parallel - no shared mutable state
- Zero tolerance for flaky tests - fix race conditions or rewrite
- Use `Clock` trait for time-dependent code, never `SystemTime::now()` directly

### Assertion Best Practices
- Include context: `assert_eq!(a, b, "message with {} context", value)`
- Error case tests mandatory - roughly 1:1 ratio with happy path
- Use `assert!(matches!(...))` for enum variants, not `if let` + panic

### Test Boundaries
- Mock at module boundaries (HTTP, DB, filesystem), not internals
- Integration test files: ONE cross-module flow per file
- Fixtures include provenance comments (source, date, modifications)
- Regression tests named after issue: `test_issue_42_description()`

### Coverage Targets
| Module | Target | Rationale |
|--------|--------|-----------|
| parser/ | 90%+ | Pure functions, easy to test |
| resolver/ | 85%+ | Core business logic |
| auth/ | 85%+ | Security-critical |
| download/ | 80%+ | I/O heavy, mock boundaries |
| queue/ | 80%+ | State management |
| storage/ | 80%+ | Database operations |
| config/ | 70%+ | Simple loading logic |
| output/ | 60%+ | Display formatting |

## Code Quality & Style Rules

### Pre-Commit Checklist
Every commit must pass:
```bash
cargo fmt --check           # Formatting
cargo clippy -- -D warnings # Lints as errors
cargo test                  # All tests pass
```

### rustfmt Configuration
```toml
# rustfmt.toml
edition = "2024"
max_width = 100
imports_granularity = "Module"
group_imports = "StdExternalCrate"
```

### Clippy Configuration
- Treat all warnings as errors (`-D warnings`)
- Key lints enforced:
  - `clippy::unwrap_used` - error in lib, allowed in tests
  - `clippy::expect_used` - requires justification comment
  - `clippy::dbg_macro` - never in committed code
  - `clippy::todo` - never in committed code

### Code Hygiene
- No `#[allow(dead_code)]` without justifying comment
- No orphan TODOs - format as `// TODO(#issue): description`
- Zero `unsafe` - find another way or escalate to architect
- `_` prefix for intentionally unused variables, not attributes

### Documentation Requirements
- Public API items require doc comments (`///`)
- Module-level docs (`//!`) explaining purpose
- Examples in doc comments for non-obvious functions

### File Length Guidelines
- Single file: max ~500 lines of implementation code (split if larger); inline `#[cfg(test)]` modules are excluded from this count
- Single function: max ~50 lines (extract helpers)
- Exception: generated code, test fixtures

### Dependency Management
- New deps require justification comment in Cargo.toml
- Evaluate: maintenance status, compile time impact, alternatives
- No duplicate functionality - check existing deps first

### Git Commit Messages
Format: `<type>(<scope>): <description>`

Types: `feat`, `fix`, `refactor`, `test`, `docs`, `chore`
Scopes: module names (`resolver`, `download`, `auth`, etc.)

Examples:
- `feat(resolver): add ScienceDirect site resolver`
- `fix(download): handle timeout on large files`
- `test(parser): add edge cases for malformed DOIs`

### CI/PR Requirements
- CI must be green - no "fix later" merges
- PRs should not decrease test coverage without justification
- Breaking changes: document with `BREAKING:` in commit, update all call sites

## Development Workflow Rules

### Branch Strategy
- `main` - stable, always deployable
- `feat/<name>` - feature branches
- `fix/<name>` - bug fix branches
- `refactor/<name>` - refactoring branches

### Development Cycle
1. Create branch from `main`
2. Implement with tests (red-green-refactor)
3. Run full check: `cargo fmt && cargo clippy -- -D warnings && cargo test`
4. Commit with conventional message
5. PR with description of changes
6. Squash merge to main

### Local Development Commands
```bash
# Full validation (run before commit)
cargo fmt && cargo clippy -- -D warnings && cargo test

# Quick check during development
cargo check

# Run specific test
cargo test test_name

# Run tests for specific module
cargo test --lib resolver::

# Watch mode (requires cargo-watch)
cargo watch -x check -x test
```

### Story Execution Protocol
- Story file is single source of truth - never deviate from acceptance criteria
- Follow task/subtask sequence - dependencies exist for a reason
- Blocked >15 minutes? Escalate or create blocking task, don't spin

### Definition of Done
- [ ] All subtasks marked complete
- [ ] All acceptance criteria verified
- [ ] Tests written and passing
- [ ] No new clippy warnings
- [ ] Documentation updated if public API changed

### Database Migration Workflow
```bash
sqlx migrate add <name>    # Create migration
sqlx migrate run           # Apply migrations
cargo sqlx prepare         # Update compile-time checks
# Commit .sqlx/ directory changes
```

### Debugging with Tracing
```bash
RUST_LOG=debug cargo run -- <args>
RUST_LOG=downloader_core::resolver=trace cargo run -- <args>
```

### Change Scope Guidelines
- `pub` API changes in lib.rs → architect review required
- Changes touching >3 modules → create design task first
- Hot path changes (download, queue, resolver) → include benchmarks
- Every commit independently deployable - no "part 1 of 3"

## Critical Don't-Miss Rules

### Anti-Patterns to NEVER Use
| Don't | Do Instead |
|-------|------------|
| `println!()` for output | `info!()`, `debug!()` via tracing |
| `.unwrap()` in library code | Return `Result`, use `?` |
| `&String` in function params | `&str` or `impl AsRef<str>` |
| `clone()` in hot loops | Use references or `Arc` |
| `std::thread::sleep` | `tokio::time::sleep` |
| Manual `if let` + panic | `assert!(matches!(...))` |
| Hardcoded paths | Use `dirs` crate or config |
| `SystemTime::now()` in logic | Use `Clock` trait for testability |

### Async Pitfalls
- Never hold `MutexGuard` across `.await` - use `tokio::sync::Mutex`
- `tokio::spawn` requires `'static` - clone needed values
- Forgetting `.await` compiles but does nothing - heed compiler warnings

### reqwest Pitfalls
- Default timeout is NONE - always set explicitly
- `.json()` consumes response body - can't call twice
- Cookie jar session-only by default - persistence is opt-in

### SQLite Pitfalls
- Enable WAL mode on connection open for concurrent reads
- Keep write transactions short - they block other writes
- Set `PRAGMA busy_timeout` to avoid immediate lock errors

### Module Boundary Rules
- Import from module root, never internal files (use `crate::resolver::ArxivResolver`)
- Only `storage` module touches SQLite directly
- Config is immutable after load - never modify
- Resolvers are stateless - no instance variables for request state

### Security Rules
- Never log credentials, cookies, or auth tokens (even at debug level)
- Use `#[instrument(skip(password, cookie))]` for sensitive params
- Encrypted storage for persisted cookies - never plaintext
- Validate all external input (URLs, DOIs) before processing

### Error Message Requirements
All user-facing errors MUST include:
1. What went wrong (clear description)
2. Why it might have happened (likely cause)
3. What to do next (actionable suggestion)

```rust
// Good
Error::AuthRequired {
    domain: "sciencedirect.com",
    suggestion: "Run `downloader auth capture` to authenticate"
}

// Bad
Error::Forbidden
```

### Platform Compatibility
- Use `PathBuf` for paths, never string concatenation
- Normalize line endings in parsers
- Don't assume filesystem case sensitivity
- Sanitize filenames from URLs (remove `/`, `\`, `:`, `*`, `?`, `"`, `<`, `>`, `|`)

### Edge Cases to Handle
- Empty input (no URLs/DOIs provided)
- Unicode in DOIs and filenames
- Very long URLs (>2000 chars)
- Circular redirects (max 10 hops)
- Partial downloads (resume support)
- Disk full during write
- Network timeout vs connection refused (different retry strategies)

### Graceful Degradation
- Crossref down → try direct URL anyway
- Metadata fetch failed → download continues with less metadata
- Single item failure → doesn't abort batch, report at end
- Long operations (>2s) → must show progress indication

---

## Usage Guidelines

**For AI Agents:**
- Read this file before implementing any code
- Follow ALL rules exactly as documented
- When in doubt, prefer the more restrictive option
- Reference architecture.md for detailed design decisions

**For Humans:**
- Keep this file lean and focused on agent needs
- Update when technology stack changes
- Review quarterly for outdated rules
- Remove rules that become obvious over time

**Last Updated:** 2026-01-26

