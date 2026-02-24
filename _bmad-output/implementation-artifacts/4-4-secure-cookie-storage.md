# Story 4.4: Secure Cookie Storage

Status: done

<!-- Note: Validation is optional. Run validate-create-story for quality check before dev-story. -->

## Story

As a **user**,
I want **my cookies stored securely (opt-in)**,
so that **I don't need to re-capture for every session**.

## Acceptance Criteria

1. **AC1: Opt-in persistence flag**
   - **Given** captured cookies and `--save-cookies`
   - **When** capture or cookie-file loading completes
   - **Then** cookies are persisted only when explicitly requested

2. **AC2: Encrypted-at-rest storage**
   - **Given** persisted cookies
   - **When** data is written to disk
   - **Then** cookie payload is encrypted at rest

3. **AC3: Keychain-backed key handling**
   - **Given** secure persistence is enabled
   - **When** encryption key is resolved
   - **Then** keychain is used by default for key storage
   - **And** env fallback (`DOWNLOADER_MASTER_KEY`) is supported for CI/non-interactive environments

4. **AC4: Standard storage location**
   - **Given** persisted cookies are saved
   - **When** default storage path is resolved
   - **Then** file location is `~/.config/downloader/cookies.enc` (or `$XDG_CONFIG_HOME/downloader/cookies.enc`)

5. **AC5: Clear persisted cookies**
   - **Given** cookies were persisted
   - **When** user runs `downloader auth clear`
   - **Then** persisted cookie file is removed and success is reported

6. **AC6: Reuse persisted cookies**
   - **Given** encrypted cookies exist
   - **When** user runs downloader without `--cookies`
   - **Then** persisted cookies are loaded automatically for authenticated requests

## Tasks / Subtasks

- [x] Task 1: Implement secure cookie storage module (AC: 2, 3, 4)
  - [x] 1.1 Create `src/auth/storage.rs`
  - [x] 1.2 Implement encrypted payload format and read/write flow
  - [x] 1.3 Implement default path resolution with XDG/HOME support
  - [x] 1.4 Add key resolution with keychain default + env fallback
  - [x] 1.5 Add unit tests for round-trip, wrong key, invalid payload

- [x] Task 2: Export storage APIs via auth/core modules (AC: 2, 3, 4)
  - [x] 2.1 Update `src/auth/mod.rs`
  - [x] 2.2 Update `src/lib.rs` re-exports

- [x] Task 3: Add opt-in save flag to normal CLI flow (AC: 1, 2, 4)
  - [x] 3.1 Add `--save-cookies` to `src/cli.rs`
  - [x] 3.2 Enforce explicit usage semantics in `main.rs`
  - [x] 3.3 Persist `--cookies` input only when flag is set

- [x] Task 4: Extend auth commands (AC: 1, 5)
  - [x] 4.1 Support `downloader auth capture --save-cookies`
  - [x] 4.2 Implement `downloader auth clear`
  - [x] 4.3 Improve auth namespace guidance for unknown subcommands

- [x] Task 5: Auto-load persisted cookies in download mode (AC: 6)
  - [x] 5.1 Load encrypted cookies when `--cookies` is not provided
  - [x] 5.2 Continue without persisted cookies on recoverable load failures

- [x] Task 6: Add E2E + unit verification (AC: 1-6)
  - [x] 6.1 Add CLI tests for save, clear, and auto-load behavior
  - [x] 6.2 Add storage unit tests for encryption/decryption correctness
  - [x] 6.3 Run fmt, clippy, and targeted test suites

### Review Follow-ups (AI)

- [x] [AI-Audit][Medium] Ensure opt-in behavior is strict. Implemented hard guard: `--save-cookies` requires `--cookies` in download mode or `auth capture --save-cookies`.
- [x] [AI-Audit][Medium] Ensure persisted cookies can be reused to satisfy “don’t re-capture each run.” Implemented automatic secure cookie load when no `--cookies` is passed.
- [x] [AI-Audit][Low] Ensure clear path is deterministic and testable with XDG overrides. Implemented and verified with E2E coverage.

### Code Review Follow-ups (AI - 2026-02-16)

- [x] [AI-Review][Medium] Add explicit tests covering `auth clear` and persisted-cookie auto-load path. Added E2E tests in `tests/cli_e2e.rs`.
- [x] [AI-Review][Low] Ensure key material generation is stable and avoids external encoding deps. Implemented internal hex encoding for generated key material.

### Code Review Follow-ups (AI - 2026-02-17)

- [x] [AI-Review][Medium] Migrate auth command parsing to clap subcommands so unknown flags fail consistently. Implemented explicit `auth capture|clear` subcommand model in `src/cli.rs` + `src/main.rs`.
- [x] [AI-Review][Medium] Enforce strict auth namespace routing when users place download flags before `auth`. Added runtime guard rejecting misplaced `auth capture|clear` positional patterns and E2E coverage.

## Dev Notes

### Architecture Context

Story 4.4 adds persistence and secure storage on top of Story 4.2/4.3 cookie ingestion:
- opt-in persistence policy,
- encrypted file storage,
- keychain-backed key management with practical CI fallback.

### Implementation Notes

- Added `src/auth/storage.rs`:
  - encrypted payload format (`DLC1` magic + nonce + ciphertext),
  - key derivation via SHA-256 over key material,
  - XChaCha20Poly1305 encryption/decryption,
  - key lookup from keychain or `DOWNLOADER_MASTER_KEY`.
- Added command support:
  - `downloader auth capture --save-cookies`
  - `downloader auth clear`
- Added runtime loading of persisted cookies when `--cookies` is absent.

### References

- [Source: _bmad-output/planning-artifacts/epics.md#Story-4.4-Secure-Cookie-Storage]
- [Source: _bmad-output/planning-artifacts/architecture.md#Authentication-&-Security]
- [Source: _bmad-output/project-context.md#Security-Rules]
- [Source: src/auth/storage.rs]
- [Source: src/main.rs]

## Dev Agent Record

### Agent Model Used

GPT-5 Codex (CLI)

### Debug Log References

- `cargo fmt --check`
- `cargo clippy -- -D warnings`
- `cargo test --lib auth::storage`
- `cargo test --bin downloader`
- `cargo test --test cli_e2e`
- manual runtime check:
  - `downloader auth capture --save-cookies`
  - `downloader auth clear`

### Completion Notes List

- Added encrypted-at-rest cookie persistence with keychain-first key management.
- Added opt-in persistence flow via `--save-cookies`.
- Added `auth clear` command and persisted-cookie auto-load behavior.
- Added/updated unit and E2E tests for save/clear/auto-load paths.
- Hardened auth CLI contract with clap subcommands (`auth capture|clear`) and strict unknown-flag rejection behavior.

### File List

- `Cargo.toml`
- `src/auth/storage.rs`
- `src/auth/mod.rs`
- `src/lib.rs`
- `src/cli.rs`
- `src/main.rs`
- `tests/cli_e2e.rs`
- `_bmad-output/implementation-artifacts/4-4-secure-cookie-storage.md`

### Change Log

- 2026-02-16: Story created, implemented, validated, and marked done.
- 2026-02-17: Follow-up hardening pass migrated auth parsing to clap subcommands and added strict auth-namespace regression coverage.

## Party Mode Audit (AI)

Audit date: 2026-02-16  
Outcome: pass_with_actions  
Summary: High=0, Medium=2, Low=1

Findings:
- Medium: Ensure strict opt-in semantics for persistence.
- Medium: Ensure persisted-cookie auto-load does not break non-auth usage.
- Low: Ensure `auth clear` path is deterministic with custom config-home values.

## Senior Developer Review (AI)

Reviewer: fierce  
Date: 2026-02-16  
Outcome: Approve

### Validation Evidence

- `cargo fmt --check`
- `cargo clippy -- -D warnings`
- `cargo test --lib auth::storage`
- `cargo test --bin downloader`
- `cargo test --test cli_e2e`
