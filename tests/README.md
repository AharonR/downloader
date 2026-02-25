# Test Suite Guide

This project uses Rust's built-in `cargo test` framework.

## Test Layout

- `tests/` integration tests for CLI, resolver, parser, queue, and download flows
- `tests/critical.rs` + `tests/critical/*.rs` — **critical test matrix** (Phases 1–5): data integrity, network resilience, auth security, resource management, system recovery
- `tests/support/critical_utils.rs` — shared helpers for critical tests: `corrupted_database()`, `flaky_network_mock()`, `exhausted_file_descriptors()`, `concurrent_load_generator()`
- `tests/integration_matrix.rs` — explicit **integration test matrix**: Engine+Queue, DB+Queue (WAL), parser validation, failure recovery
- `tests/optimization_refactor_commands.rs` — integration tests for the optimization-refactor command-handler extraction (config show, auth clear, log, search)
- `src/*` inline unit tests for module-level behavior

## Running Tests

```bash
# all tests
cargo test

# focused sidecar coverage
cargo test --bin downloader sidecar

# run sidecar batch-generation tests only
cargo test --bin downloader generate_sidecars_for_completed

# run optimization-refactor integration tests (command-handler extraction)
cargo test optimization_refactor

# run non-functional regression gates (ignored by default)
cargo test --test nonfunctional_regression_gates -- --ignored --nocapture

# critical test matrix (Phase 1–3 P0 run by default; Phase 4–5 and env-dependent tests ignored)
cargo test --test critical

# run all critical tests including ignored (nightly / full CI)
cargo test --test critical -- --ignored

# integration matrix (cross-module and E2E scenarios)
cargo test --test integration_matrix
```

## Coverage (`cargo-llvm-cov`)

Execution scope controls which tests run; denominator is separately constrained at report time via
filename filtering.

### Phase Scope and Denominator Policy

- Phase 1 coverage suite (deterministic baseline):
  - `--lib`
  - `--bin downloader`
  - `--test queue_integration`
  - `--test download_engine_integration`
  - `--test cli_e2e`
  - `--test integration_matrix`
- Phase 2 expansion: replace the explicit integration test list with `--tests` while keeping
  `--lib --bin downloader`.
- Ignored tests are excluded in both phases (no `-- --ignored` in coverage commands).
- Denominator policy: include library + `downloader` binary source files only.
- Explicit denominator exclusions (filtered at report time even if compiled):
  - `src/bin/extract_md_links.rs`
  - `src/bin/stress_sidecar_flaky.rs`
- Maintenance rule: when a new utility binary is added under `src/bin/` and should remain out of
  coverage denominator, update `COVERAGE_IGNORE_REGEX` and the workflow validation checks in
  `.github/workflows/coverage.yml`.

### CI Coverage Workflow

- Workflow: `.github/workflows/coverage.yml` (`Coverage`, job `coverage`)
- Reporting: HTML + LCOV artifacts (`coverage-html-<run_id>`, `coverage-lcov-<run_id>`)
- Enforcement: informational only (no coverage threshold in this rollout)
- Branch protection: do not mark the coverage check as required in Phase 1
- Socket behavior: strict (`DOWNLOADER_REQUIRE_SOCKET_TESTS=1`) to match CI gate behavior
- Tool version: `cargo-llvm-cov` is intentionally unpinned in CI; the workflow records
  `cargo llvm-cov --version` in logs and the job summary for traceability

### Quick Local Coverage (non-strict; socket-bound tests may skip)

```bash
rustup component add llvm-tools-preview
cargo install cargo-llvm-cov --locked

export CARGO_TARGET_DIR=target/coverage-local
export COVERAGE_IGNORE_REGEX='(^|[\\/])src[\\/]bin[\\/](extract_md_links|stress_sidecar_flaky)\.rs$'

cargo llvm-cov clean --workspace
cargo llvm-cov \
  --workspace \
  --lib \
  --bin downloader \
  --test queue_integration \
  --test download_engine_integration \
  --test cli_e2e \
  --test integration_matrix \
  --no-report

cargo llvm-cov report --summary-only --ignore-filename-regex "$COVERAGE_IGNORE_REGEX"
cargo llvm-cov report --html --output-dir target/coverage-local/llvm-cov-html --ignore-filename-regex "$COVERAGE_IGNORE_REGEX"
cargo llvm-cov report --lcov --output-path target/coverage-local/lcov.info --ignore-filename-regex "$COVERAGE_IGNORE_REGEX"
```

Quick local coverage percentages may differ from CI if socket-bound tests skip in your local
environment.

### CI-Parity Local Coverage (strict; fails if localhost bind is unavailable)

```bash
rustup component add llvm-tools-preview
cargo install cargo-llvm-cov --locked

export DOWNLOADER_REQUIRE_SOCKET_TESTS=1
export CARGO_TARGET_DIR=target/coverage-local
export COVERAGE_IGNORE_REGEX='(^|[\\/])src[\\/]bin[\\/](extract_md_links|stress_sidecar_flaky)\.rs$'

cargo llvm-cov clean --workspace
cargo llvm-cov \
  --workspace \
  --lib \
  --bin downloader \
  --test queue_integration \
  --test download_engine_integration \
  --test cli_e2e \
  --test integration_matrix \
  --no-report

cargo llvm-cov report --summary-only --ignore-filename-regex "$COVERAGE_IGNORE_REGEX"
cargo llvm-cov report --html --output-dir target/coverage-local/llvm-cov-html --ignore-filename-regex "$COVERAGE_IGNORE_REGEX"
cargo llvm-cov report --lcov --output-path target/coverage-local/lcov.info --ignore-filename-regex "$COVERAGE_IGNORE_REGEX"
```

Open the HTML report at `target/coverage-local/llvm-cov-html/index.html`.

### Troubleshooting

- `error: no such command: llvm-cov`
  - Install the tool: `cargo install cargo-llvm-cov --locked`
- Missing LLVM tools / report-generation failures
  - Install the Rust component: `rustup component add llvm-tools-preview`
- Strict socket bind failures (`DOWNLOADER_REQUIRE_SOCKET_TESTS=1`)
  - Use the non-strict local coverage mode above, or run in a full environment that allows
    `127.0.0.1` socket binding

## Priority Convention

Use a prefix in test names to indicate priority:

- `p0_` critical path
- `p1_` high value
- `p2_` medium value
- `p3_` low value

Example:

```rust
#[test]
fn test_p1_sidecar_skip_existing_file() { /* ... */ }
```

Run by prefix:

```bash
cargo test p0_
```

## Socket-Bound Tests

Some tests use `wiremock` and require localhost socket binding.

- Default behavior in constrained local sandboxes: tests print a skip reason and return early.
- Required-gate behavior (CI/strict local runs): set `DOWNLOADER_REQUIRE_SOCKET_TESTS=1` to fail fast instead of skipping.
- Quick preflight:

```bash
python - <<'PY'
import socket
s = socket.socket()
try:
    s.bind(("127.0.0.1", 0))
    print("bind-ok", s.getsockname()[1])
except Exception as e:
    print("bind-fail", repr(e))
finally:
    s.close()
PY
```

- If preflight reports `bind-fail`, treat strict failures as environment capability blockers and capture strict evidence on CI.

Example strict run:

```bash
DOWNLOADER_REQUIRE_SOCKET_TESTS=1 cargo test --all-targets
```

## Critical Tests and CI

- **Pre-merge:** Run Phase 1–3 critical tests (data integrity, network resilience, auth security) plus existing integration tests. Use `DOWNLOADER_REQUIRE_SOCKET_TESTS=1` for socket-bound tests in CI.
- **Nightly:** Run full critical suite including ignored tests: `cargo test --test critical -- --ignored`. Covers file-DB, FD exhaustion, interruptible process, and env-isolated encryption tests.
- **Release:** Full integration matrix and critical suite; optionally chaos-style scenarios.

Some critical tests are marked `#[ignore]` because they need file DB, lowered FD limits, or env isolation; they pass when run with `--ignored` in a full environment.

## Proof-of-Green Requirements

Before closing a rollout-gate phase, capture pass/fail evidence for:

```bash
cargo fmt --all --check
cargo clippy -- -D warnings
DOWNLOADER_REQUIRE_SOCKET_TESTS=1 cargo test --all-targets
DOWNLOADER_REQUIRE_SOCKET_TESTS=1 cargo test --test queue_integration
DOWNLOADER_REQUIRE_SOCKET_TESTS=1 cargo test --test download_engine_integration
DOWNLOADER_REQUIRE_SOCKET_TESTS=1 cargo test --test cli_e2e
DOWNLOADER_REQUIRE_SOCKET_TESTS=1 cargo test --test critical
DOWNLOADER_REQUIRE_SOCKET_TESTS=1 cargo test --test integration_matrix
```

Optional scheduled-equivalent lane:

```bash
cargo test --test nonfunctional_regression_gates -- --ignored --nocapture
cargo test --test critical -- --ignored
```

Record outputs in:

- `docs/complexity-refactor/phase-rollout-gates-audit-followup.md`
