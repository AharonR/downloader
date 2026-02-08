# Implementation Effort Summary

## 1) CLI Flag Ordering with Positional URLs

- Removed trailing var-arg behavior for `urls` so flags are parsed in any position.
- Added parser coverage for:
  - flag after URL
  - flag before URL
  - flag between URLs
  - long flag after URL
  - invalid flag after URL
  - `--` separator for dash-prefixed positional tokens
- Added E2E smoke check that verifies flag-after-positional-token parsing.

Files:
- `src/cli.rs`
- `tests/cli_e2e.rs`
- `README.md`
- `CHANGELOG.md`
- `FIXES.md`

## 2) Retry Count Persistence Semantics

- Changed queue failure API to persist explicit retry counts.
- Updated engine failure path to store `attempts - 1` as retry count.
- Updated integration tests to assert exact persisted retry count semantics.

Files:
- `src/queue/mod.rs`
- `src/download/engine.rs`
- `tests/queue_integration.rs`
- `tests/download_engine_integration.rs`

## 3) Deterministic Non-Network E2E Coverage

- Added new E2E test that feeds an overlength URL from stdin, ensuring parser rejection before any download/network path.
- Kept the existing TEST-NET unreachable-host E2E test unchanged.

File:
- `tests/cli_e2e.rs`

## Validation Performed

- `cargo fmt --check` passed
- `cargo test --bin downloader` passed
- `cargo test --test queue_integration test_mark_failed` passed
- `cargo test --test download_engine_integration test_retry_count_persisted_in_database` passed
- `cargo test --test cli_e2e test_binary_flag_after_positional_url_is_parsed_as_flag` passed
- `cargo test --test cli_e2e test_binary_stdin_with_invalid_url_exits_cleanly` passed

