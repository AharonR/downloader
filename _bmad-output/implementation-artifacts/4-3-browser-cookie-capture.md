# Story 4.3: Browser Cookie Capture

Status: done

<!-- Note: Validation is optional. Run validate-create-story for quality check before dev-story. -->

## Story

As a **user**,
I want **to capture cookies from my browser session**,
so that **I can use my existing institutional login**.

## Acceptance Criteria

1. **AC1: Guided capture instructions**
   - **Given** I run `downloader auth capture`
   - **When** the capture process runs
   - **Then** step-by-step instructions are displayed:
     1. Install a cookie export extension (e.g., "Get cookies.txt LOCALLY")
     2. Log into the target site
     3. Export cookies to Netscape format
     4. Paste cookie file path or pipe contents

2. **AC2: Prompt for cookie data**
   - **Given** I run `downloader auth capture`
   - **When** the command is running
   - **Then** I am prompted to provide cookie input (interactive path/data prompt or stdin piping)

3. **AC3: Netscape + JSON support**
   - **Given** cookie exports from common browser extensions
   - **When** I provide Netscape or JSON input
   - **Then** the capture parser accepts and normalizes both formats

4. **AC4: Cookie validation**
   - **Given** captured cookie records
   - **When** validation runs
   - **Then** expired cookies are filtered out
   - **And** required fields are enforced (domain/name/value)

5. **AC5: Success confirmation**
   - **Given** valid captured cookies
   - **When** the command completes
   - **Then** the output confirms: `Cookies captured for X domains`

## Tasks / Subtasks

- [x] Task 1: Add browser capture parsing module (AC: 3, 4)
  - [x] 1.1 Create `src/auth/capture.rs`
  - [x] 1.2 Add dual-format parser for Netscape and JSON
  - [x] 1.3 Add validation for required fields + expiration filtering
  - [x] 1.4 Add unique-domain counting helper
  - [x] 1.5 Add unit tests for format handling and validation failures

- [x] Task 2: Wire new capture API into auth module exports (AC: 3, 4)
  - [x] 2.1 Update `src/auth/mod.rs` exports
  - [x] 2.2 Update `src/lib.rs` re-exports
  - [x] 2.3 Add constructor helper to `CookieLine` for normalized record creation

- [x] Task 3: Implement `downloader auth capture` execution flow (AC: 1, 2, 5)
  - [x] 3.1 Add command detection path before normal CLI parsing in `src/main.rs`
  - [x] 3.2 Add guided step-by-step instruction output
  - [x] 3.3 Add interactive prompt / stdin source handling
  - [x] 3.4 Parse + validate capture payload and emit warnings
  - [x] 3.5 Emit success message with exact domain-count confirmation

- [x] Task 4: Add/extend tests for capture command (AC: 1, 3, 4, 5)
  - [x] 4.1 Add CLI E2E test for Netscape input
  - [x] 4.2 Add CLI E2E test for JSON input
  - [x] 4.3 Add CLI E2E test for expired-only failure path

### Review Follow-ups (AI)

- [x] [AI-Audit][Medium] Ensure `auth capture` path does not interfere with existing URL download flow. Implemented isolated pre-parse command dispatch for `auth capture`.
- [x] [AI-Audit][Medium] Ensure JSON cookie exports with `hostOnly` are normalized to exact-domain semantics. Implemented explicit `tailmatch` derivation from `hostOnly`.
- [x] [AI-Audit][Low] Ensure domain counting normalizes leading dot variants (e.g., `.example.com` vs `example.com`). Implemented canonical domain counting.

### Code Review Follow-ups (AI - 2026-02-16)

- [x] [AI-Review][Medium] Capture tests asserted stderr, but runtime logging emits on stdout for this binary. Updated assertions to stdout for deterministic E2E checks.
- [x] [AI-Review][Low] Clippy precision/truncation warnings in expiry conversion. Reworked conversion to string-based integer parsing to satisfy strict lint gate (`-D warnings`).

## Dev Notes

### Architecture Context

Story 4.3 introduces command-driven cookie capture guidance and robust capture parsing while preserving the existing Epic 4 download path. The capture command also integrates with the Story 4.4 encrypted storage module via `--save-cookies`.

### Implementation Notes

- Added `CapturedCookieFormat`, `CapturedCookies`, and `CaptureError` abstractions.
- Added `parse_captured_cookies()` that auto-detects Netscape vs JSON payloads.
- Added validation pass for required fields and expiration rules.
- Added `downloader auth capture` command path in `main.rs` with:
  - deterministic instruction output,
  - interactive prompt when stdin is terminal,
  - stdin/path ingestion in non-interactive mode,
  - exact success confirmation string.

### References

- [Source: _bmad-output/planning-artifacts/epics.md#Story-4.3-Browser-Cookie-Capture]
- [Source: _bmad-output/planning-artifacts/architecture.md#Authentication-&-Security]
- [Source: _bmad-output/project-context.md#Security-Rules]
- [Source: src/auth/cookies.rs]
- [Source: src/main.rs]

## Dev Agent Record

### Agent Model Used

GPT-5 Codex (CLI)

### Debug Log References

- `cargo fmt`
- `cargo clippy -- -D warnings`
- `cargo test --lib auth::capture`
- `cargo test --bin downloader`
- `cargo test --test cli_e2e`

### Completion Notes List

- Added browser-cookie capture parsing module with Netscape+JSON support.
- Added validation gate for required fields + expiration filtering.
- Added `downloader auth capture` runtime path with guided instructions and prompt/pipe input handling.
- Added E2E coverage for Netscape success, JSON success, and expired-input failure.
- Preserved existing download command path and cookie file flow.

### File List

- `src/auth/capture.rs`
- `src/auth/cookies.rs`
- `src/auth/mod.rs`
- `src/cli.rs`
- `src/lib.rs`
- `src/main.rs`
- `tests/cli_e2e.rs`
- `_bmad-output/implementation-artifacts/4-3-browser-cookie-capture.md`

### Change Log

- 2026-02-16: Story created, implemented, tested, reviewed, and marked done.
- 2026-02-17: Code review fixes applied (M1-M4, L1-L4).

## Party Mode Audit (AI)

Audit date: 2026-02-16  
Outcome: pass_with_actions  
Summary: High=0, Medium=2, Low=1

Findings:
- Medium: Ensure `auth capture` dispatch does not break regular argument parsing.
- Medium: Ensure JSON host/domain normalization preserves subdomain semantics.
- Low: Ensure canonical domain counting deduplicates dotted/non-dotted variants.

## Senior Developer Review (AI)

Reviewer: fierce
Date: 2026-02-16
Outcome: Approve

### Validation Evidence

- `cargo fmt`
- `cargo clippy -- -D warnings`
- `cargo test --lib auth::capture`
- `cargo test --bin downloader`
- `cargo test --test cli_e2e`

## Senior Developer Review #2 (AI)

Reviewer: fierce
Date: 2026-02-17
Outcome: Approve (with fixes applied)

### Findings (8 total: 4 Medium, 4 Low)

**Fixed:**
- [M1] `SystemTime::now()` in `validate_cookies` → parameterized `now: u64`, added deterministic test
- [M2] `auth capture` bypasses clap → added TODO(arch) comment documenting migration path
- [M3] Interactive prompt misleading for multi-line data → reworded to clarify file-path-only
- [M4] File List missing `src/cli.rs` → added to Dev Agent Record
- [L1] `#[allow(deprecated)]` unexplained → added comment documenting `Command::cargo_bin` deprecation
- [L2] Missing `#[instrument]` on `unique_domain_count` → added
- [L3] `normalized_expiry` overflow fallback → added clarifying comment
- [L4] Dev Notes contradicted implementation → updated architecture context

### Validation Evidence

- `cargo clippy -- -D warnings` (clean)
- `cargo test --lib auth::capture` (7/7 pass, including new parameterized validation test)
- `cargo test --test cli_e2e` (24/24 pass)
