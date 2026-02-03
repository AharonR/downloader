# System-Level Test Design: Downloader

**Date:** 2026-01-27
**Author:** fierce
**Mode:** System-Level Testability Review (Phase 3 Solutioning)
**Status:** Draft

---

## Executive Summary

**Purpose:** Evaluate architecture testability and define system-wide testing strategy before implementation.

**Architecture Assessment:**
- **Overall Testability:** HIGH - Architecture decisions support comprehensive testing
- **Key Strengths:** Trait-based abstractions, Clock trait, in-memory SQLite, async test support
- **Key Concerns:** External API dependencies, cookie encryption, site-specific resolver variability

**NFR Coverage:**
| Category | Testability | Approach |
|----------|-------------|----------|
| Performance | HIGH | In-process benchmarks, controlled concurrency tests |
| Reliability | HIGH | Fault injection, mock failures, graceful degradation tests |
| Security | MEDIUM | Cookie handling tests, no actual credential testing |
| Usability | HIGH | CLI output capture, exit code validation |

**Risk Summary:**
- Total risks identified: 12
- High-priority risks (Score ≥6): 3
- Critical categories: External APIs (DATA), Cookie Security (SEC), Site Resolver Variability (TECH)

---

## Architecture Testability Review

### Testability Strengths (Positive Patterns)

| Pattern | Location | Testability Benefit |
|---------|----------|---------------------|
| Resolver Trait | `resolver/mod.rs` | Mock resolvers for deterministic tests |
| Clock Trait | `time/mod.rs` | Time-dependent code testable (rate limits, retries) |
| KeyStorage Enum | `auth/mod.rs` | Memory/Mock backends avoid keychain in tests |
| SQLite In-Memory | `storage/mod.rs` | Fast, isolated database tests |
| Error Types | `error.rs` | Structured error assertions |
| Async Runtime | Tokio | `#[tokio::test]` native support |

### Testability Concerns (Requires Mitigation)

| Concern | Impact | Mitigation Strategy |
|---------|--------|---------------------|
| **Crossref API Dependency** | External service can't be controlled | Use wiremock to mock HTTP responses |
| **Site-Specific Resolvers** | Real sites change, tests flaky | Fixture-based testing with recorded responses |
| **Cookie Encryption** | Keychain access in tests | KeyStorage::Memory for test, KeyStorage::Keychain for prod |
| **File System Operations** | Tests leave artifacts | Use tempfile crate, cleanup in test fixtures |
| **Network Timeouts** | Tests slow or flaky | Mock network layer, use Clock trait for timeouts |
| **Progress Display** | Terminal output hard to assert | Capture output via tracing test subscriber |
| **SQLite WAL Mode** | In-memory doesn't fully support WAL | Use file-based temp DB for WAL-specific tests |

### Flaky Test Prevention Strategy

| Strategy | Implementation |
|----------|----------------|
| **Deterministic ordering** | Don't rely on test execution order; each test fully isolated |
| **Isolated database per test** | Create fresh in-memory SQLite per test, not shared pool |
| **Time control** | Use `tokio::time::pause()` for time-sensitive tests |
| **Network mocking** | All HTTP via wiremock; no real network in unit/integration tests |
| **File isolation** | Each test gets own `TempDir`, auto-cleanup on drop |
| **Seed randomness** | Use fixed seeds for any randomized test data |

### Module-Level Testability Assessment

| Module | Unit Test Support | Integration Test Support | Notes |
|--------|-------------------|--------------------------|-------|
| `parser/` | EXCELLENT | N/A (pure functions) | 90%+ coverage achievable |
| `resolver/` | GOOD | GOOD (with mocks) | Trait allows injection |
| `download/` | GOOD | GOOD (with wiremock) | Stream handling needs care |
| `queue/` | EXCELLENT | EXCELLENT | In-memory SQLite |
| `auth/` | GOOD | MEDIUM | KeyStorage abstraction helps |
| `storage/` | EXCELLENT | EXCELLENT | sqlx test patterns |
| `config/` | EXCELLENT | N/A | Simple TOML parsing |
| `output/` | MEDIUM | MEDIUM | Terminal mocking needed |

---

## NFR Validation Strategy

### NFR-1: Performance Testing

**Targets from PRD:**
- Parse 150 references < 5 seconds
- 10 concurrent downloads (different domains)
- Memory < 200MB during operation
- Startup < 1 second

**Testing Approach:**

| Metric | Test Type | Tool | Threshold |
|--------|-----------|------|-----------|
| Parse latency | Benchmark | `criterion` | <5s for 150 refs |
| Concurrent downloads | Integration | wiremock + semaphore | 10 parallel verified |
| Memory usage | Benchmark | `/usr/bin/time -v` or `heaptrack` | <200MB peak |
| Startup time | Integration | `std::time::Instant` | <1s cold start |

**Test Implementation Pattern:**

```rust
#[tokio::test]
async fn test_parse_150_references_under_5_seconds() {
    let input = load_fixture("bibliography_150_refs.txt");
    let start = Instant::now();
    let result = parser::parse_input(&input).await;
    let elapsed = start.elapsed();

    assert!(elapsed < Duration::from_secs(5), "Parsing took {:?}", elapsed);
    assert_eq!(result.items.len(), 150);
}
```

### NFR-2: Reliability Testing

**Targets from PRD:**
- Download success rate ≥90%
- Auth site success rate ≥70%
- Naming accuracy ≥95%
- Never crash, always log

**Testing Approach:**

| Scenario | Test Type | Approach |
|----------|-----------|----------|
| Network failures | Unit + Integration | Mock 5xx responses, verify retry |
| Auth failures | Integration | Mock 401/403, verify graceful handling |
| Malformed input | Unit | Fuzz-like edge cases |
| Panic isolation | Integration | `std::panic::catch_unwind` for tasks |
| Graceful shutdown | Integration | Signal handling tests |

**Fault Injection Pattern:**

```rust
#[tokio::test]
async fn test_download_retries_on_transient_failure() {
    let mock = MockServer::start().await;
    let mut call_count = 0;

    Mock::given(method("GET"))
        .respond_with(move |_| {
            call_count += 1;
            if call_count < 3 {
                ResponseTemplate::new(503)
            } else {
                ResponseTemplate::new(200).set_body_bytes(b"PDF content")
            }
        })
        .mount(&mock)
        .await;

    let result = download_file(&mock.uri()).await;
    assert!(result.is_ok());
    assert_eq!(call_count, 3); // Retried twice
}
```

**Panic Recovery Pattern (Never Crash):**

```rust
#[tokio::test]
async fn test_resolver_panic_does_not_crash_queue() {
    let queue = Queue::new(test_db().await);

    // Add item that will cause resolver to panic
    queue.add("panic://trigger").await.unwrap();

    // Process queue with panic-catching wrapper
    let result = std::panic::catch_unwind(AssertUnwindSafe(|| {
        tokio::runtime::Runtime::new().unwrap().block_on(async {
            queue.process_all().await
        })
    }));

    // Queue should handle panic gracefully
    assert!(result.is_ok());

    // Item should be marked as failed, not stuck
    let item = queue.get("panic://trigger").await.unwrap();
    assert_eq!(item.status, Status::Failed);
    assert!(item.error_message.contains("panic"));
}
```

**Graceful Degradation Pattern (Crossref Down):**

```rust
#[tokio::test]
async fn test_doi_resolution_falls_back_when_crossref_down() {
    let mock = MockServer::start().await;

    // Crossref returns 503 (service unavailable)
    Mock::given(method("GET"))
        .and(path_regex("/works/.*"))
        .respond_with(ResponseTemplate::new(503))
        .mount(&mock)
        .await;

    let resolver = CrossrefResolver::new(&mock.uri());
    let result = resolver.resolve("10.1234/example").await;

    // Should return graceful error, not panic
    assert!(result.is_err());
    let error = result.unwrap_err();
    assert!(matches!(error, Error::ServiceUnavailable { .. }));
    assert!(error.to_string().contains("retry later"));
}
```

### NFR-3: Usability Testing

**Targets from PRD:**
- Zero-config start
- Clear error messages (What/Why/Fix)
- Progress visibility

**Testing Approach:**

| Scenario | Test Type | Approach |
|----------|-----------|----------|
| Default config works | Integration | CLI with no args or config file |
| Error message format | Unit | Structured error assertions |
| Exit codes | Integration | CLI execution, check exit status |
| Progress output | Integration | Capture stderr, verify format |

**CLI Output Testing Pattern:**

```rust
#[test]
fn test_error_message_follows_what_why_fix() {
    let error = Error::AuthRequired {
        domain: "sciencedirect.com".to_string(),
    };
    let message = format!("{}", error);

    assert!(message.contains("Authentication required")); // What
    assert!(message.contains("session may have expired")); // Why
    assert!(message.contains("downloader auth capture")); // Fix
}
```

### NFR-4: Maintainability Testing

**Targets from PRD:**
- Site resolver modularity
- Configuration flexibility
- Logging for debugging

**Testing Approach:**

| Scenario | Test Type | Approach |
|----------|-----------|----------|
| Resolver registration | Unit | Add mock resolver, verify discovery |
| Config override | Integration | CLI flags override config.toml |
| Structured logging | Integration | tracing-test subscriber |

---

## Risk Assessment

### High-Priority Risks (Score ≥6)

| Risk ID | Category | Description | Prob | Impact | Score | Mitigation | Owner |
|---------|----------|-------------|------|--------|-------|------------|-------|
| R-001 | DATA | Crossref API unavailable causes DOI resolution failures | 2 | 3 | 6 | Fallback to direct URL, comprehensive mocking | Dev |
| R-002 | SEC | Cookie storage encryption bypass | 2 | 3 | 6 | KeyStorage abstraction, keychain integration tests | Dev |
| R-003 | TECH | Site-specific resolvers break on site changes | 3 | 2 | 6 | Fixture-based tests, resolver version pinning | Dev |

### Medium-Priority Risks (Score 3-4)

| Risk ID | Category | Description | Prob | Impact | Score | Mitigation |
|---------|----------|-------------|------|--------|-------|------------|
| R-004 | PERF | Concurrent download tests flaky under CI load | 2 | 2 | 4 | Use controlled mock servers, not real network |
| R-005 | DATA | SQLite migration failures on upgrade | 2 | 2 | 4 | Migration tests, backup before upgrade |
| R-006 | OPS | CI environment differs from local (paths, permissions) | 2 | 2 | 4 | Docker-based CI, explicit path handling |
| R-007 | BUS | Reference parsing accuracy below target | 2 | 2 | 4 | Extensive fixture coverage, confidence tracking |

### Low-Priority Risks (Score 1-2)

| Risk ID | Category | Description | Prob | Impact | Score | Action |
|---------|----------|-------------|------|--------|-------|--------|
| R-008 | TECH | Async test isolation failures | 1 | 2 | 2 | Use test-scoped fixtures |
| R-009 | OPS | Test data fixtures become stale | 1 | 2 | 2 | Document fixture provenance |
| R-010 | PERF | Benchmark variance across machines | 1 | 2 | 2 | CI-only benchmark assertions |
| R-011 | BUS | Progress display varies by terminal | 1 | 1 | 1 | Test with mock terminal |
| R-012 | TECH | Clippy/fmt changes break CI | 1 | 1 | 1 | Pin toolchain version |

---

## Testing Patterns for Rust/Tokio Stack

### Pattern 1: Async Test with Mocked HTTP

```rust
use wiremock::{MockServer, Mock, ResponseTemplate};
use wiremock::matchers::{method, path};

#[tokio::test]
async fn test_crossref_resolver_returns_metadata() {
    let mock = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/works/10.1234/example"))
        .respond_with(ResponseTemplate::new(200)
            .set_body_json(load_fixture("crossref_response.json")))
        .mount(&mock)
        .await;

    let resolver = CrossrefResolver::new(&mock.uri());
    let result = resolver.resolve("10.1234/example").await;

    assert!(result.is_ok());
    let metadata = result.unwrap();
    assert_eq!(metadata.title, "Example Paper");
}
```

### Pattern 2: Database Test with In-Memory SQLite

```rust
use sqlx::sqlite::SqlitePoolOptions;

async fn test_db() -> SqlitePool {
    let pool = SqlitePoolOptions::new()
        .connect("sqlite::memory:")
        .await
        .unwrap();

    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .unwrap();

    pool
}

#[tokio::test]
async fn test_queue_persistence() {
    let pool = test_db().await;
    let queue = Queue::new(pool);

    queue.add("https://example.com/paper.pdf").await.unwrap();

    let items = queue.pending().await.unwrap();
    assert_eq!(items.len(), 1);
}
```

### Pattern 3: Time-Controlled Tests with Clock Trait

```rust
// Production uses SystemClock, tests use MockClock
trait Clock: Send + Sync {
    fn now(&self) -> Instant;
    async fn sleep(&self, duration: Duration);
}

struct MockClock {
    current: Arc<Mutex<Instant>>,
}

impl MockClock {
    fn advance(&self, duration: Duration) {
        let mut current = self.current.lock().unwrap();
        *current += duration;
    }
}

#[tokio::test]
async fn test_rate_limiter_waits_between_requests() {
    let clock = Arc::new(MockClock::new());
    let limiter = RateLimiter::new(clock.clone(), Duration::from_secs(1));

    limiter.acquire("example.com").await;
    clock.advance(Duration::from_millis(500));

    // Should wait additional 500ms
    let start = Instant::now();
    limiter.acquire("example.com").await;
    // With mock clock, this completes immediately but internal state is correct
}
```

### Pattern 4: Resolver Registry Priority Test

```rust
#[tokio::test]
async fn test_resolver_registry_tries_in_priority_order() {
    let mut registry = ResolverRegistry::new();

    // Track which resolvers were called
    let call_order = Arc::new(Mutex::new(Vec::new()));

    // High priority resolver that fails
    let order1 = call_order.clone();
    registry.register(MockResolver::new(
        "high-priority",
        100, // priority
        move |_| {
            order1.lock().unwrap().push("high");
            Err(Error::NotFound)
        },
    ));

    // Medium priority resolver that succeeds
    let order2 = call_order.clone();
    registry.register(MockResolver::new(
        "medium-priority",
        50, // priority
        move |_| {
            order2.lock().unwrap().push("medium");
            Ok(ResolvedUrl::new("https://example.com/paper.pdf"))
        },
    ));

    // Low priority resolver (should not be called)
    let order3 = call_order.clone();
    registry.register(MockResolver::new(
        "low-priority",
        10, // priority
        move |_| {
            order3.lock().unwrap().push("low");
            Ok(ResolvedUrl::new("https://fallback.com/paper.pdf"))
        },
    ));

    let result = registry.resolve("10.1234/example").await;

    // Should succeed with medium priority resolver
    assert!(result.is_ok());
    assert_eq!(result.unwrap().url, "https://example.com/paper.pdf");

    // Should have tried high first, then medium (not low)
    let order = call_order.lock().unwrap();
    assert_eq!(*order, vec!["high", "medium"]);
}
```

### Pattern 5: WAL Mode with File-Based Temp Database

```rust
#[tokio::test]
async fn test_wal_mode_concurrent_reads() {
    let temp_dir = TempDir::new().unwrap();
    let db_path = temp_dir.path().join("test.db");

    let pool = SqlitePoolOptions::new()
        .connect(&format!("sqlite:{}?mode=rwc", db_path.display()))
        .await
        .unwrap();

    // Enable WAL mode
    sqlx::query("PRAGMA journal_mode=WAL")
        .execute(&pool)
        .await
        .unwrap();

    sqlx::migrate!("./migrations").run(&pool).await.unwrap();

    // Concurrent reads should not block
    let pool1 = pool.clone();
    let pool2 = pool.clone();

    let (r1, r2) = tokio::join!(
        sqlx::query("SELECT COUNT(*) FROM queue").fetch_one(&pool1),
        sqlx::query("SELECT COUNT(*) FROM queue").fetch_one(&pool2),
    );

    assert!(r1.is_ok());
    assert!(r2.is_ok());
}
```

### Pattern 6: Snapshot Testing for Error Messages

```rust
use insta::assert_snapshot;

#[test]
fn test_auth_required_error_message() {
    let error = Error::AuthRequired {
        domain: "sciencedirect.com".to_string(),
        suggestion: "Run `downloader auth capture` to authenticate".to_string(),
    };

    // Snapshot ensures error message format doesn't accidentally change
    assert_snapshot!(format!("{}", error));
}

#[test]
fn test_parser_output_format() {
    let input = "10.1234/example\nhttps://arxiv.org/pdf/2401.00001";
    let result = parser::parse_input(input).unwrap();

    // Snapshot the parsed structure
    assert_snapshot!(format!("{:#?}", result));
}
```

### Pattern 7: CLI Integration Test

```rust
use assert_cmd::Command;
use predicates::prelude::*;

#[test]
fn test_cli_dry_run_shows_parsed_items() {
    let mut cmd = Command::cargo_bin("downloader").unwrap();

    cmd.arg("--dry-run")
        .write_stdin("https://example.com/paper.pdf\n10.1234/example")
        .assert()
        .success()
        .stdout(predicate::str::contains("2 items"))
        .stdout(predicate::str::contains("1 URL"))
        .stdout(predicate::str::contains("1 DOI"))
        .stdout(predicate::str::contains("Dry run"));
}

#[test]
fn test_cli_exit_codes() {
    // Exit 0: All success
    Command::cargo_bin("downloader")
        .unwrap()
        .arg("--dry-run")
        .write_stdin("https://example.com/valid.pdf")
        .assert()
        .code(0);

    // Exit 2: Complete failure (no input)
    Command::cargo_bin("downloader")
        .unwrap()
        .write_stdin("")
        .assert()
        .code(2);
}
```

---

## Test Infrastructure Requirements

### Required Crates

| Crate | Purpose | Dev Dependency |
|-------|---------|----------------|
| `wiremock` | HTTP mocking | Yes |
| `tempfile` | Temporary directories | Yes |
| `assert_cmd` | CLI testing | Yes |
| `predicates` | Assertion helpers | Yes |
| `criterion` | Benchmarking | Yes |
| `tracing-test` | Log capture in tests | Yes |
| `fake` | Test data generation | Yes |
| `tokio-test` | Time control (`pause()`, `advance()`) | Yes |
| `insta` | Snapshot testing for outputs/errors | Yes |
| `proptest` | Property-based/fuzz testing (optional) | Yes |

### Test Directory Structure

```
tests/
├── common/
│   ├── mod.rs          # Shared test utilities
│   ├── db.rs           # test_db(), test_db_with_schema()
│   └── fixtures.rs     # load_fixture()
├── fixtures/
│   ├── v1/                           # Versioned fixtures
│   │   ├── crossref_response.json
│   │   ├── bibliography_50_refs.txt
│   │   └── sciencedirect_page.html
│   ├── v2/                           # Updated API responses
│   │   └── crossref_response.json
│   └── README.md       # Fixture provenance & version notes
├── integration/
│   ├── parser_test.rs
│   ├── resolver_test.rs
│   ├── download_test.rs
│   └── queue_test.rs
├── cli_e2e.rs          # Full CLI tests
└── snapshots/          # insta snapshot files (auto-generated)

benches/                # criterion benchmarks (project root)
├── parser_bench.rs
└── download_bench.rs
```

### CI Pipeline Stages

```yaml
stages:
  - lint:      cargo fmt --check && cargo clippy -- -D warnings
  - unit:      cargo test --lib
  - integration: cargo test --test '*'
  - benchmark: cargo bench --no-run  # Verify compiles, don't run in CI
  - coverage:  cargo llvm-cov --lcov > coverage.lcov

# Platform-specific jobs
jobs:
  test-linux:
    runs-on: ubuntu-latest
    # ... standard tests

  test-macos:
    runs-on: macos-latest
    # Required for KeyStorage::Keychain integration tests
    # macOS Keychain behaves differently than Linux secret storage
```

**Note:** macOS runner required for KeyStorage::Keychain integration tests. Linux CI will only test KeyStorage::Memory variant.

---

## Coverage Targets

| Module | Target | Rationale |
|--------|--------|-----------|
| `parser/` | 90%+ | Pure functions, easy to test comprehensively |
| `resolver/` | 85%+ | Core business logic, trait-based testability |
| `auth/` | 85%+ | Security-critical, abstraction helps |
| `download/` | 80%+ | I/O heavy, mock at boundaries |
| `queue/` | 80%+ | State management, in-memory SQLite |
| `storage/` | 80%+ | Database operations, straightforward |
| `config/` | 70%+ | Simple loading, low complexity |
| `output/` | 60%+ | Display formatting, harder to test |

**Overall Target:** 80%+ library code coverage

---

## Recommendations

### Before Implementation

1. **Set up test infrastructure first** (Story 1.1)
   - Configure wiremock, tempfile, assert_cmd, tokio-test, insta
   - Create common test utilities module
   - Establish fixture directory with versioning (v1/, v2/)

   **Required Story 1.1 AC Additions:**
   - "And tests/ directory structure is created per test-design-system.md"
   - "And common test utilities (test_db, load_fixture) are implemented"
   - "And dev-dependencies include: wiremock, tempfile, assert_cmd, tokio-test, insta"
   - "And benches/ directory exists with placeholder benchmark"

2. **Implement Clock trait early** (Story 1.6)
   - Rate limiting and retry logic depend on it
   - Use `tokio::time::pause()` in tests (simpler than custom MockClock)

3. **Design KeyStorage abstraction** (Story 4.4)
   - Memory variant for tests
   - Keychain variant for production
   - Interface before implementation
   - Plan macOS CI runner for keychain tests

### Story Gaps Identified

| Gap | Recommendation |
|-----|----------------|
| Test infrastructure setup | Add AC to Story 1.1 (see above) |
| CI pipeline configuration | Add story to Epic 1 or Epic 7 |
| Coverage verification (80%) | Add story to Epic 8: "Verify test coverage meets 80% target" |
| Snapshot baseline | Add to Story 7.5 (error messages): "And snapshot tests exist for all error formats"

### During Implementation

1. **Write tests alongside code** (not after)
   - Each story should include unit tests
   - Integration tests at epic boundaries

2. **Mock external services immediately**
   - Don't write tests against real Crossref API
   - Record real responses as fixtures, test against fixtures

3. **Use #[instrument] on public functions**
   - Enables tracing-test assertions
   - Debugging support built-in

### After MVP

1. **Run coverage check**
   - `cargo llvm-cov` to verify 80% target
   - Identify gaps before Epic 8

2. **Stabilize benchmarks**
   - Run criterion benchmarks
   - Establish baseline for performance regression

---

## Quality Gate Criteria

### Test Design Approval Gate

- [x] Architecture reviewed for testability
- [x] NFR validation approaches defined
- [x] High-priority risks identified and mitigated
- [x] Testing patterns documented for tech stack
- [x] Coverage targets defined per module
- [ ] Test infrastructure requirements approved

### Pre-Implementation Gate

- [ ] Test crates added to Cargo.toml
- [ ] Common test utilities created
- [ ] Fixture directory established
- [ ] CI pipeline configured

---

## Approval

**System-Level Test Design Approved By:**

- [ ] Architect: _____________ Date: _______
- [ ] Tech Lead: _____________ Date: _______
- [ ] Developer: _____________ Date: _______

**Comments:**

---

## Appendix

### Knowledge Base References

- `nfr-criteria.md` - NFR validation approaches
- `test-levels-framework.md` - Unit/Integration/E2E selection
- `risk-governance.md` - Risk scoring methodology
- `test-quality.md` - Test quality standards

### Related Documents

- PRD: `_bmad-output/planning-artifacts/prd.md`
- Architecture: `_bmad-output/planning-artifacts/architecture.md`
- Epics: `_bmad-output/planning-artifacts/epics.md`
- Project Context: `_bmad-output/project-context.md`

---

**Generated by**: BMad TEA Agent - System-Level Testability Review
**Workflow**: `testarch-test-design` (System-Level Mode)
**Version**: 4.0 (BMad v6)
