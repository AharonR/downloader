# Phase Rollout Gates Audit Follow-up

Date: 2026-02-19

## Scope

Follow-up for party-audit findings on rollout gates:

- Restore hard gate truthfulness (`cargo clippy -- -D warnings`).
- Deduplicate socket-bound test guard behavior across integration and unit-test modules.
- Add CI preflight diagnostics for localhost bind readiness.
- Require proof-of-green evidence recording before marking the phase complete.

## Required evidence commands

1. `cargo fmt --all --check`
2. `cargo clippy -- -D warnings`
3. `cargo test --lib download::client::tests::test_http_client_download_invalid_url -- --nocapture`
4. `cargo test --all-targets`
5. `python` localhost bind probe
6. `DOWNLOADER_REQUIRE_SOCKET_TESTS=1 cargo test --all-targets`
7. `DOWNLOADER_REQUIRE_SOCKET_TESTS=1 cargo test --test queue_integration`
8. `DOWNLOADER_REQUIRE_SOCKET_TESTS=1 cargo test --test download_engine_integration`
9. `DOWNLOADER_REQUIRE_SOCKET_TESTS=1 cargo test --test cli_e2e`
10. `DOWNLOADER_REQUIRE_SOCKET_TESTS=1 cargo test --test critical`
11. `DOWNLOADER_REQUIRE_SOCKET_TESTS=1 cargo test --test integration_matrix`
12. `cargo test --test nonfunctional_regression_gates -- --ignored --nocapture` (optional scheduled-equivalent lane)

## Evidence log

- `cargo fmt --all --check` -> PASS
- `cargo clippy -- -D warnings` -> PASS
- `cargo test --lib download::client::tests::test_http_client_download_invalid_url -- --nocapture` -> PASS
  - `HttpClient::new()` no longer crashes on macOS `system-configuration` proxy panic path.
- `cargo test --all-targets` -> PASS
- localhost bind probe (`python`) -> FAIL (`PermissionError(1, 'Operation not permitted')`)
- `DOWNLOADER_REQUIRE_SOCKET_TESTS=1 cargo test --all-targets` -> FAIL
  - Primary blocker: strict socket-mode tests cannot bind `127.0.0.1` in this environment, causing expected fail-fast panics from socket guard.
  - Prior `system-configuration` panic is remediated in `src/download/client.rs`; current strict failures are bind-capability gating.
- `DOWNLOADER_REQUIRE_SOCKET_TESTS=1 cargo test --test queue_integration` -> PASS
- `DOWNLOADER_REQUIRE_SOCKET_TESTS=1 cargo test --test download_engine_integration` -> FAIL
  - Primary blocker: strict socket-mode tests cannot bind `127.0.0.1` in this environment.
- `DOWNLOADER_REQUIRE_SOCKET_TESTS=1 cargo test --test cli_e2e` -> PASS
- `DOWNLOADER_REQUIRE_SOCKET_TESTS=1 cargo test --test critical` -> FAIL
  - Primary blocker: strict socket-mode tests cannot bind `127.0.0.1` in this environment.
- `DOWNLOADER_REQUIRE_SOCKET_TESTS=1 cargo test --test integration_matrix` -> FAIL
  - Primary blocker: strict socket-mode tests cannot bind `127.0.0.1` in this environment.
- `cargo test --test nonfunctional_regression_gates -- --ignored --nocapture` -> PASS

## Exit decision

- Decision: `HOLD`
- Rationale: local hard gates are green (`fmt`, `clippy`, non-strict `all-targets`) and proxy panic regression is fixed, but mandatory strict socket gate remains blocked by local localhost bind restriction. Phase remains `in_progress` until strict gates pass on a compatible CI runner and evidence is attached.
