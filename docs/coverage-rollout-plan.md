# Final Revised Plan: Decision-Complete Coverage Rollout (`cargo-llvm-cov`) for `/Users/ar2463/Documents/GitHub/Downloader`

Add a dedicated GitHub Actions coverage workflow for this Rust project that collects and publishes coverage as HTML and LCOV artifacts without changing existing quality gates. Roll out in two phases: Phase 1 establishes a stable baseline on a curated deterministic suite and constrained denominator; Phase 2 expands the executed test suite while keeping the denominator stable and adding explicit rollback criteria.

## Summary
This plan introduces a new non-blocking coverage signal lane at `/Users/ar2463/Documents/GitHub/Downloader/.github/workflows/coverage.yml` and adds exact local coverage commands to `/Users/ar2463/Documents/GitHub/Downloader/README.md` and `/Users/ar2463/Documents/GitHub/Downloader/tests/README.md`. It explicitly defines denominator filtering, cache policy, branch-protection handling, artifact retention, reporting, rollout criteria, and rollback triggers so implementation requires no additional decisions.

## Scope
- In: New coverage workflow; HTML + LCOV artifact publishing; GitHub step summary; exact local docs commands (strict and non-strict variants); denominator filtering policy; manual branch-protection rollout checklist; Phase 2 promotion and rollback criteria.
- Out: Codecov/Coveralls integration; coverage thresholds/diff-coverage enforcement; changes to `/Users/ar2463/Documents/GitHub/Downloader/.github/workflows/phase-rollout-gates.yml`; inclusion of ignored/scheduled-only suites in PR/push coverage; Rust runtime/API changes.

## Important Changes to Public APIs / Interfaces / Types
- No Rust public API, CLI flag, or runtime type changes.
- New CI workflow file: `/Users/ar2463/Documents/GitHub/Downloader/.github/workflows/coverage.yml`.
- New CI artifacts: `coverage-html-${{ github.run_id }}` and `coverage-lcov-${{ github.run_id }}`.
- New developer-facing documentation sections in `/Users/ar2463/Documents/GitHub/Downloader/README.md` and `/Users/ar2463/Documents/GitHub/Downloader/tests/README.md`.

## Coverage Policy (Decision-Complete)
- Execution scope controls which tests run; denominator is separately constrained at report time via filename filtering.
- Phase 1 execution scope (curated deterministic suite): `--lib`, `--bin downloader`, `--test queue_integration`, `--test download_engine_integration`, `--test cli_e2e`, `--test integration_matrix`.
- Phase 1 excludes `critical` and `nonfunctional_regression_gates` from PR/push coverage.
- Phase 1 denominator policy: include only library + `downloader` binary source files.
- Phase 1 explicit denominator exclusions (must be filtered at report time even if compiled): `/Users/ar2463/Documents/GitHub/Downloader/src/bin/extract_md_links.rs` and `/Users/ar2463/Documents/GitHub/Downloader/src/bin/stress_sidecar_flaky.rs`.
- Phase 2 execution scope expansion: replace curated `--test ...` list with `--tests` while retaining `--lib --bin downloader` and the same denominator exclusions.
- Ignored tests remain excluded in both phases (no `-- --ignored` in coverage commands).
- Coverage remains informational only in Phase 1 and Phase 2 under this plan (no threshold enforcement).
- Maintenance rule: whenever a new utility binary is added under `/Users/ar2463/Documents/GitHub/Downloader/src/bin/` and should remain out of the denominator, update `COVERAGE_IGNORE_REGEX` and the validation checks in the coverage workflow and docs.

## Tooling Version and Pinning Policy
- GitHub Actions are pinned by major tag (repo convention), not SHA, in this rollout.
- `cargo-llvm-cov` tool version is intentionally floating (latest available via install action) in Phase 1 and Phase 2.
- Commands are exact; tool version is intentionally unpinned by policy.
- Risk acceptance is explicit: CI validates compatibility on every run; no tool-version pin is added in this rollout.
- The coverage job must record `cargo llvm-cov --version` in logs and `$GITHUB_STEP_SUMMARY` for traceability.
- SHA pinning and tool-version pinning are deferred to a repo-wide CI hardening pass, not handled in this coverage-specific rollout.

## Cache and Build Output Policy
- Coverage workflow must set `CARGO_TARGET_DIR=target/coverage-ci` to isolate instrumented builds from default local/CI builds.
- Coverage workflow must not use `Swatinem/rust-cache@v2` in Phase 1.
- Coverage workflow must not use `Swatinem/rust-cache@v2` in Phase 2 under this plan unless a separate follow-up plan enables it.
- Rationale: reduce cache ambiguity while baselining coverage performance and reliability.

## CI Workflow Specification
- Add `/Users/ar2463/Documents/GitHub/Downloader/.github/workflows/coverage.yml`.
- Workflow name: `Coverage`.
- Triggers: `pull_request` on `main` and `master`; `push` on `main` and `master`; `workflow_dispatch`.
- Permissions: `contents: read`.
- Single job: id/name `coverage`; `runs-on: ubuntu-latest`; `timeout-minutes: 40`.
- Job env:
- `CARGO_TARGET_DIR=target/coverage-ci`
- `DOWNLOADER_REQUIRE_SOCKET_TESTS=1`
- `COVERAGE_IGNORE_REGEX='(^|[\\/])src[\\/]bin[\\/](extract_md_links|stress_sidecar_flaky)\\.rs$'`
- YAML note: use single quotes for `COVERAGE_IGNORE_REGEX` to avoid escape mangling.

## Exact CI Steps (Phase 1)
1. `actions/checkout@v4`
2. `dtolnay/rust-toolchain@stable`
3. Socket-bind preflight step copied from `/Users/ar2463/Documents/GitHub/Downloader/.github/workflows/phase-rollout-gates.yml` and kept strict.
4. `taiki-e/install-action@v2` with `tool: cargo-llvm-cov`
5. `rustup component add llvm-tools-preview`
6. `cargo llvm-cov --version`
7. `cargo llvm-cov clean --workspace`
8. Execute the coverage run (single run, no multi-run aggregation):
```bash
cargo llvm-cov \
  --workspace \
  --lib \
  --bin downloader \
  --test queue_integration \
  --test download_engine_integration \
  --test cli_e2e \
  --test integration_matrix \
  --no-report
```
9. Generate summary report with denominator filter:
```bash
cargo llvm-cov report \
  --summary-only \
  --ignore-filename-regex "$COVERAGE_IGNORE_REGEX" | tee target/coverage-ci/coverage-summary.txt
```
10. Generate HTML report with denominator filter:
```bash
cargo llvm-cov report \
  --html \
  --output-dir target/coverage-ci/llvm-cov-html \
  --ignore-filename-regex "$COVERAGE_IGNORE_REGEX"
```
11. Generate LCOV report with denominator filter:
```bash
cargo llvm-cov report \
  --lcov \
  --output-path target/coverage-ci/lcov.info \
  --ignore-filename-regex "$COVERAGE_IGNORE_REGEX"
```
12. Run automated validation checks (LCOV is canonical for inclusion/exclusion; HTML checks are spot-check validations only):
```bash
test -s target/coverage-ci/coverage-summary.txt
test -s target/coverage-ci/lcov.info
test -d target/coverage-ci/llvm-cov-html
test -f target/coverage-ci/llvm-cov-html/index.html
rg -n 'src/(cli|download_engine)\.rs' target/coverage-ci/lcov.info
! rg -n 'src/bin/(extract_md_links|stress_sidecar_flaky)\.rs' target/coverage-ci/lcov.info
rg -n 'cli\.rs|download_engine\.rs' target/coverage-ci/llvm-cov-html
! rg -n 'extract_md_links\.rs|stress_sidecar_flaky\.rs' target/coverage-ci/llvm-cov-html
```
13. Upload artifacts with `actions/upload-artifact@v4`:
- HTML path: `target/coverage-ci/llvm-cov-html/**`
- HTML artifact name: `coverage-html-${{ github.run_id }}`
- LCOV path: `target/coverage-ci/lcov.info`
- LCOV artifact name: `coverage-lcov-${{ github.run_id }}`
- `retention-days: 30`
- `if-no-files-found: error`
14. Append GitHub step summary (`$GITHUB_STEP_SUMMARY`) containing:
- phase label (`Phase 1 baseline`)
- `cargo llvm-cov --version` output
- contents of `target/coverage-ci/coverage-summary.txt`
- artifact names
- note: `Informational only (not required branch check in Phase 1)`

## Exact CI Steps (Phase 2 Change Only)
- Replace only Phase 1 Step 8 with:
```bash
cargo llvm-cov \
  --workspace \
  --lib \
  --bin downloader \
  --tests \
  --no-report
```
- Keep the same report-time denominator filter, validation checks, artifacts, retention, and summary steps.
- `critical` and `nonfunctional_regression_gates` remain excluded because ignored tests are still not requested.

## Local Developer Commands to Document (Two Variants)

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
- Note for docs: quick local coverage percentages may differ from CI if socket-bound tests skip in the local environment.

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

## Documentation Additions (Required Content)
- Add a short `Coverage` section to `/Users/ar2463/Documents/GitHub/Downloader/README.md` with: purpose of the coverage lane; quick local (non-strict) command block; CI-parity (strict) command block; HTML report path (`target/coverage-local/llvm-cov-html/index.html`); informational-only rollout note.
- Add a detailed coverage section to `/Users/ar2463/Documents/GitHub/Downloader/tests/README.md` with: exact Phase 1 suite targets; Phase 2 suite change (`--tests`); denominator policy and explicit utility-bin exclusions; strict vs non-strict local behavior and when to use each; maintenance rule for `COVERAGE_IGNORE_REGEX`; troubleshooting.
- Required troubleshooting entries:
- `error: no such command: llvm-cov` → install `cargo-llvm-cov`
- missing LLVM tools → `rustup component add llvm-tools-preview`
- strict socket bind failures → use non-strict local mode or run in a full environment
- branch-protection note → coverage is not required in Phase 1

## Branch Protection and Repo Settings (Manual Operational Step)
- Owner: repo admin or release owner with branch-protection permissions.
- Manual step after the first successful `Coverage` workflow run:
- Open a PR checks UI and copy the exact displayed check name string for the coverage status.
- Open branch protection settings for `main` and `master`.
- Confirm that exact coverage check name is not selected as a required check.
- Record the verification result in the implementation PR comment or rollout note.
- Verification criterion: branch protection still requires only pre-existing checks; coverage failures do not block merges in Phase 1.

## Action Items
- [ ] Create `/Users/ar2463/Documents/GitHub/Downloader/.github/workflows/coverage.yml` with the exact workflow name, triggers, permissions, job definition, timeout, and env variables defined above.
- [ ] Copy the socket preflight logic from `/Users/ar2463/Documents/GitHub/Downloader/.github/workflows/phase-rollout-gates.yml` into the new coverage workflow and keep strict behavior with `DOWNLOADER_REQUIRE_SOCKET_TESTS=1`.
- [ ] Add `taiki-e/install-action@v2` (`tool: cargo-llvm-cov`), run `rustup component add llvm-tools-preview`, and print `cargo llvm-cov --version`.
- [ ] Do not add `Swatinem/rust-cache@v2` to `/Users/ar2463/Documents/GitHub/Downloader/.github/workflows/coverage.yml` in this rollout.
- [ ] Implement the exact Phase 1 execution and report commands, including `COVERAGE_IGNORE_REGEX` on every report-generation step.
- [ ] Add automated validation commands that treat LCOV as canonical inclusion/exclusion validation and HTML as spot-check validation.
- [ ] Upload HTML and LCOV artifacts with `actions/upload-artifact@v4`, `if-no-files-found: error`, and `retention-days: 30`.
- [ ] Add a GitHub step summary with phase label, tool version, summary text, artifact names, and the non-enforcing notice.
- [ ] Update `/Users/ar2463/Documents/GitHub/Downloader/README.md` with exact non-strict and CI-parity local coverage commands plus the comparability note.
- [ ] Update `/Users/ar2463/Documents/GitHub/Downloader/tests/README.md` with Phase 1/Phase 2 suite definitions, denominator policy, regex maintenance rule, strict vs non-strict guidance, and troubleshooting.
- [ ] Perform the manual branch-protection verification step after the first successful coverage run and record the exact check name and verification result.
- [ ] Keep `/Users/ar2463/Documents/GitHub/Downloader/.github/workflows/phase-rollout-gates.yml` and `/Users/ar2463/Documents/GitHub/Downloader/.github/workflows/stress-sidecar-flaky.yml` unchanged in this rollout.

## Test Cases and Scenarios
- `Coverage` workflow triggers on PR, push to `main`/`master`, and manual dispatch.
- Strict socket preflight passes on supported GitHub runners and fails clearly on unsupported environments.
- `cargo llvm-cov` installs successfully and version is recorded in logs and step summary.
- Phase 1 coverage run completes and produces `target/coverage-ci/coverage-summary.txt`, `target/coverage-ci/llvm-cov-html/index.html`, and `target/coverage-ci/lcov.info`.
- LCOV report includes core product files (for example `src/cli.rs` and `src/download_engine.rs`).
- LCOV report excludes `src/bin/extract_md_links.rs` and `src/bin/stress_sidecar_flaky.rs`.
- HTML report spot-check confirms expected core files appear and excluded utility-bin files do not appear.
- Artifact uploads succeed and persist for 30 days.
- Missing outputs fail the workflow due to explicit validation commands and `if-no-files-found: error`.
- GitHub step summary includes phase label, tool version, coverage summary, and artifact names.
- Existing workflows remain unchanged and operational: `/Users/ar2463/Documents/GitHub/Downloader/.github/workflows/phase-rollout-gates.yml` and `/Users/ar2463/Documents/GitHub/Downloader/.github/workflows/stress-sidecar-flaky.yml`.
- Branch protection does not require the coverage check in Phase 1.

## Rollout, Promotion, and Rollback Criteria
- Phase 1 starts after merge of workflow + docs changes and completion of the branch-protection verification step.
- Phase 1 promotion to Phase 2 requires both: at least 10 consecutive green `Coverage` workflow runs and at least 14 calendar days of normal PR activity since rollout.
- Capture Phase 1 baseline metrics from GitHub Actions history: median duration across the last 10 green runs and non-infra coverage workflow failure rate across all runs.
- Classification owner for infra vs non-infra failures: repo maintainer or release owner (single named person in the rollout PR).
- Phase 2 change is limited to replacing the execution command with the Phase 2 command in this plan.
- Phase 2 rollback trigger (manual rollback to the Phase 1 execution command) is any one of: rolling 20-run non-infra failure rate exceeds 10%; rolling 10-run median duration exceeds 1.5x the Phase 1 median; rolling 10-run median duration exceeds 30 minutes absolute.
- Non-infra failures include test failures, coverage command failures, and timeouts.
- Non-infra failures exclude GitHub-wide outages and unrelated transient platform incidents.
- Rollback action: revert only the execution command to the Phase 1 command; keep report filters, validation, artifacts, and docs unchanged; record rollback reason in PR description or incident note.

## Assumptions and Defaults
- `cargo-llvm-cov` is the selected Rust coverage tool for this project.
- Tool version floats intentionally in this rollout; compatibility is verified by CI and traced via `cargo llvm-cov --version`.
- Major-tag action pinning (not SHA pinning) is acceptable because it matches current repo workflow conventions.
- Utility binaries are excluded from the coverage denominator via explicit report-time filename regex filtering, not only by target selection.
- Coverage workflow is additive and non-blocking; branch protection is intentionally unchanged in Phase 1.
- Coverage artifacts use 30-day retention to cover the observation window and review lag.
- No cache is used in the coverage workflow during this rollout to reduce cache ambiguity while baselining.
