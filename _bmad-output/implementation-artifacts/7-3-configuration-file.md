# Story 7.3: Configuration File

Status: done

<!-- Note: Validation is optional. Run validate-create-story for quality check before dev-story. -->

## Story

As a **user**,
I want **to set default options in a config file**,
so that **I don't repeat common flags**.

## Acceptance Criteria

1. **AC1: Load config defaults from standard path**
   - **Given** a config file at `~/.config/downloader/config.toml`
   - **When** the tool runs
   - **Then** config values are loaded as defaults

2. **AC2: Respect XDG config home override**
   - **Given** `XDG_CONFIG_HOME` is set
   - **When** the tool runs
   - **Then** config is loaded from `$XDG_CONFIG_HOME/downloader/config.toml`

3. **AC3: CLI flags override config values**
   - **Given** both config defaults and CLI flags are present
   - **When** a command executes
   - **Then** CLI-provided values take precedence

4. **AC4: Support required configuration settings**
   - **Given** a config file
   - **When** values are parsed
   - **Then** supported settings include `output_dir`, `concurrency`, `rate_limit`, and `verbosity`

5. **AC5: Add `downloader config show` effective-config command**
   - **Given** `downloader config show`
   - **When** the command runs
   - **Then** it displays current effective config values

6. **AC6: Missing config file is non-fatal**
   - **Given** no config file exists
   - **When** the tool runs
   - **Then** execution continues with built-in defaults (no error)

## Tasks / Subtasks

- [x] Task 1: Add config command surface and config schema plumbing (AC: 4, 5)
  - [x] 1.1 Add `config` command namespace with `show` subcommand in `src/cli.rs`
  - [x] 1.2 Introduce typed config-file schema for supported keys (`output_dir`, `concurrency`, `rate_limit`, `verbosity`)

- [x] Task 2: Implement config file discovery/loading and effective-args merge (AC: 1, 2, 3, 6)
  - [x] 2.1 Resolve config path from `XDG_CONFIG_HOME` then HOME fallback (`~/.config/...`)
  - [x] 2.2 Load/parse TOML config only when file exists; treat missing file as defaults
  - [x] 2.3 Merge config defaults into download args while preserving CLI-overrides precedence
  - [x] 2.4 Apply config-driven verbosity only when verbosity flags are not supplied on CLI

- [x] Task 3: Implement `downloader config show` output and add coverage (AC: 5)
  - [x] 3.1 Print effective values for supported config keys
  - [x] 3.2 Include visibility into resolved config path and whether file was loaded

- [x] Task 4: Add deterministic tests for config behavior (AC: 1-6)
  - [x] 4.1 CLI parser tests for `config show`
  - [x] 4.2 E2E test: missing config file uses defaults without error
  - [x] 4.3 E2E test: XDG path config is loaded
  - [x] 4.4 E2E test: CLI flags override config defaults

### Review Follow-ups (AI)

- [x] [AI-Audit][Medium] Ensure CLI override precedence is implemented using actual argument source detection (not value-comparison heuristics on defaults).
- [x] [AI-Audit][Medium] Ensure config parsing validates numeric ranges (`concurrency`, `rate_limit`) with actionable error messages.
- [x] [AI-Audit][Low] Add coverage for `downloader config show` output determinism (stable key ordering/format).

## Dev Notes

### Architecture Context

- CLI argument definitions are centralized in `src/cli.rs`; runtime orchestration is in `src/main.rs`.
- Existing execution paths already support command namespaces (`auth`, `log`), so `config show` should align with established command handling.
- Download behavior currently derives defaults from clap; config merge must not regress current semantics or validation constraints.

### Implementation Guidance

- Keep config loading lightweight and deterministic with typed parsing.
- Preserve zero-config behavior: no config file should never block execution.
- Avoid side effects for `config show`; this command is read-only and should not touch queue state.

### Testing Notes

- Prefer temp directories + `XDG_CONFIG_HOME` env in E2E tests for isolated config behavior.
- Use non-network deterministic inputs for command-path tests.
- Validate both positive load cases and override precedence behavior.

### Project Structure Notes

- Expected touch points: `src/cli.rs`, `src/main.rs`, `tests/cli_e2e.rs`.
- Likely new module/file for config parsing helpers in `src/`.

### References

- [Source: _bmad-output/planning-artifacts/epics.md#Story-7.3-Configuration-File]
- [Source: _bmad-output/planning-artifacts/prd.md#FR-5-CLI-Interface]
- [Source: _bmad-output/planning-artifacts/architecture.md#Requirements-Overview]
- [Source: _bmad-output/project-context.md#Framework-Specific-Rules]

## Dev Agent Record

### Agent Model Used

GPT-5 Codex (CLI)

### Debug Log References

- create-story via epic-auto-flow
- cargo fmt
- cargo test --locked --bin downloader app_config::tests -- --nocapture
- cargo test --locked --bin downloader test_cli_config -- --nocapture
- cargo test --locked --test cli_e2e config_ -- --nocapture
- cargo test --locked --test cli_e2e test_binary_download_cli_output_dir_overrides_config -- --nocapture
- cargo test --locked --test cli_e2e -- --nocapture
- cargo test --locked --bin downloader -- --nocapture
- cargo clippy --locked --bin downloader --test cli_e2e -- -D warnings (blocked by pre-existing `clippy::too_many_arguments` in `src/queue/history.rs`)
- code-review follow-up: hardened numeric parsing against overflow/trailing-token acceptance in config loader
- cargo test --locked --bin downloader app_config::tests -- --nocapture
- cargo test --locked --test cli_e2e test_binary_config_show_ -- --nocapture
- cargo test --locked --test cli_e2e test_binary_download_ -- --nocapture
- cargo test --locked --bin downloader test_cli_config -- --nocapture

### Completion Notes List

- 2026-02-17: Story created and set to ready-for-dev.
- 2026-02-17: Added `config` command namespace with `show` subcommand and parser coverage.
- 2026-02-17: Implemented config file discovery using `XDG_CONFIG_HOME` with HOME fallback and non-fatal missing-config behavior.
- 2026-02-17: Implemented effective settings merge so command-line values override config defaults based on clap value-source detection.
- 2026-02-17: Implemented effective config display via `downloader config show`.
- 2026-02-17: Added config-focused E2E coverage for defaults, XDG loading, and CLI override precedence.
- 2026-02-17: Senior code review completed with High/Medium issues auto-fixed; story accepted as done.
- 2026-02-17: Added HOME-fallback config-show E2E coverage and numeric parser hardening regressions.

### File List

- _bmad-output/implementation-artifacts/7-3-configuration-file.md
- _bmad-output/implementation-artifacts/sprint-status.yaml
- src/app_config.rs
- src/cli.rs
- src/main.rs
- tests/cli_e2e.rs

### Change Log

- 2026-02-17: Story created and marked ready-for-dev.
- 2026-02-17: Party mode audit completed with follow-up actions.
- 2026-02-17: Implemented configuration-file defaults flow and moved story to review.
- 2026-02-17: Code review completed; story marked done.
- 2026-02-17: Follow-up review fixes applied (numeric parse hardening + HOME fallback coverage).

## Party Mode Audit (AI)

Audit date: 2026-02-17  
Outcome: pass_with_actions  
Summary: High=0, Medium=2, Low=1

Findings:
- Medium: CLI-over-config precedence can be incorrectly implemented if command-line/default sources are not distinguished explicitly.
- Medium: Unvalidated config numeric values can create runtime drift versus clap constraints.
- Low: `config show` output format can become inconsistent and brittle for scripting/tests without deterministic structure.

## Senior Developer Review (AI)

Reviewer: fierce  
Date: 2026-02-17  
Outcome: Approve

### Findings Summary

- High: 1 (fixed)
- Medium: 2 (fixed)
- Low: 0

### Fixed During Review (Auto-Fix High/Medium)

- **High (fixed):** `rate_limit` parsing casted large positive `i128` values directly to `u64`, which could wrap and silently accept invalid values.  
  Fix: use checked `u64::try_from` conversion and fail with out-of-range error.
- **Medium (fixed):** Numeric parser accepted trailing garbage tokens (e.g. `concurrency = 4 trailing`) because it parsed only the first whitespace token.  
  Fix: parse the fully-trimmed value token so malformed numeric literals fail deterministically.
- **Medium (fixed):** AC1 HOME-fallback behavior lacked direct automated validation.  
  Fix: added E2E coverage for `config show` with `XDG_CONFIG_HOME` unset and `HOME/.config/downloader/config.toml` populated.

### Remaining Decision Items

- None.
