# Test Suite Guide — downloader-core

This project uses Rust's built-in `cargo test` framework.

## Test Layout

- `downloader-core/tests/` — integration tests for resolver, parser, queue, download, and auth flows
- `downloader-core/tests/critical.rs` + `tests/critical/*.rs` — **critical test matrix** (Phases 1–5): data integrity, network resilience, auth security, resource management, system recovery
- `downloader-core/tests/support/critical_utils.rs` — shared helpers for critical tests: `corrupted_database()`, `flaky_network_mock()`, `exhausted_file_descriptors()`, `concurrent_load_generator()`
- `downloader-core/tests/integration_matrix.rs` — explicit **integration test matrix**: Engine+Queue, DB+Queue (WAL), parser validation, failure recovery
- `downloader-cli/tests/` — CLI end-to-end tests: `cli_e2e.rs`, `exit_code_partial_e2e.rs`, `optimization_refactor_commands.rs`
- `downloader-core/src/*` — inline unit tests for module-level behavior

## Running Tests

```bash
# all workspace tests
cargo test --workspace

# downloader-core only
cargo test -p downloader-core

# downloader-cli only
cargo test -p downloader-cli

# focused queue integration
cargo test --test queue_integration -p downloader-core

# run non-functional regression gates (ignored by default)
cargo test --test nonfunctional_regression_gates -p downloader-core -- --ignored --nocapture

# critical test matrix (Phase 1–3 P0 run by default)
cargo test --test critical -p downloader-core

# run all critical tests including ignored (nightly / full CI)
cargo test --test critical -p downloader-core -- --ignored

# integration matrix (cross-module and E2E scenarios)
cargo test --test integration_matrix -p downloader-core
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
  - `downloader-cli/src/bin/extract_md_links.rs`
  - `downloader-cli/src/bin/stress_sidecar_flaky.rs`

### CI Coverage Workflow

- Workflow: `.github/workflows/coverage.yml` (`Coverage`, job `coverage`)
- Reporting: HTML + LCOV artifacts (`coverage-html-<run_id>`, `coverage-lcov-<run_id>`)
- Enforcement: informational only (no coverage threshold in this rollout)

### Quick Local Coverage

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
```

## Priority Convention

Use a prefix in test names to indicate priority:

- `p0_` critical path
- `p1_` high value
- `p2_` medium value
- `p3_` low value

## Socket-Bound Tests

Some tests use `wiremock` and require localhost socket binding.

- Default behavior in constrained local sandboxes: tests print a skip reason and return early.
- Required-gate behavior (CI/strict local runs): set `DOWNLOADER_REQUIRE_SOCKET_TESTS=1`.

```bash
DOWNLOADER_REQUIRE_SOCKET_TESTS=1 cargo test --workspace --all-targets
```

## Proof-of-Green Requirements

Before closing a rollout-gate phase:

```bash
cargo fmt --all --check
cargo clippy --workspace -- -D warnings
DOWNLOADER_REQUIRE_SOCKET_TESTS=1 cargo test --workspace --all-targets
DOWNLOADER_REQUIRE_SOCKET_TESTS=1 cargo test --test queue_integration -p downloader-core
DOWNLOADER_REQUIRE_SOCKET_TESTS=1 cargo test --test download_engine_integration -p downloader-core
DOWNLOADER_REQUIRE_SOCKET_TESTS=1 cargo test --test cli_e2e -p downloader-cli
DOWNLOADER_REQUIRE_SOCKET_TESTS=1 cargo test --test critical -p downloader-core
DOWNLOADER_REQUIRE_SOCKET_TESTS=1 cargo test --test integration_matrix -p downloader-core
```
