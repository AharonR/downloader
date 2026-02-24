# Story 4.5: ScienceDirect Resolver

Status: done

<!-- Note: Validation is optional. Run validate-create-story for quality check before dev-story. -->

## Story

As a **user**,
I want **to download papers from ScienceDirect**,
so that **I can access my institutional subscriptions**.

## Acceptance Criteria

1. **AC1: Resolve ScienceDirect PDF URL**
   - **Given** a ScienceDirect URL or DOI
   - **When** the resolver processes it with valid cookies
   - **Then** the PDF download URL is resolved

2. **AC2: Apply authentication cookies to resolver requests**
   - **Given** cookies are provided or persisted
   - **When** ScienceDirect pages are requested during resolution
   - **Then** cookie jar state is applied to resolver HTTP requests

3. **AC3: Extract article metadata from page**
   - **Given** a ScienceDirect article page response
   - **When** resolution succeeds
   - **Then** metadata fields are extracted from page tags (title, authors, DOI, etc.)

4. **AC4: Recognize common ScienceDirect URL patterns**
   - **Given** common ScienceDirect article URL forms
   - **When** resolver matching runs
   - **Then** patterns such as `/science/article/pii/...` and `/science/article/abs/pii/...` are recognized

5. **AC5: Expired-auth guidance**
   - **Given** auth appears expired (login page or auth status)
   - **When** resolution fails for auth reasons
   - **Then** the failure guidance suggests refreshing cookies

## Tasks / Subtasks

- [x] Task 1: Add ScienceDirect specialized resolver module (AC: 1, 3, 4, 5)
  - [x] 1.1 Create `src/resolver/sciencedirect.rs`
  - [x] 1.2 Implement URL/DOI matching for ScienceDirect patterns
  - [x] 1.3 Implement article-page fetch + PDF URL extraction
  - [x] 1.4 Implement metadata extraction from citation meta tags
  - [x] 1.5 Implement auth-required detection with cookie-refresh messaging

- [x] Task 2: Wire resolver into resolver exports and runtime (AC: 1, 2, 4)
  - [x] 2.1 Export `ScienceDirectResolver` in `src/resolver/mod.rs`
  - [x] 2.2 Re-export in `src/lib.rs`
  - [x] 2.3 Register resolver in `src/main.rs` before general/fallback resolvers

- [x] Task 3: Route URL inputs through resolver pipeline (AC: 1, 4)
  - [x] 3.1 Remove direct URL bypass in parse/enqueue path
  - [x] 3.2 Resolve all parsed inputs through registry (with direct fallback)
  - [x] 3.3 Preserve queue behavior and dedupe checks

- [x] Task 4: Add tests for ScienceDirect behavior (AC: 1-5)
  - [x] 4.1 Unit tests for matching, PII extraction, and metadata parsing
  - [x] 4.2 Resolver integration test: URL + cookies -> resolved PDF URL + metadata
  - [x] 4.3 Resolver integration test: Elsevier DOI -> ScienceDirect resolution
  - [x] 4.4 Resolver integration test: auth/login page -> refresh-cookie guidance

### Review Follow-ups (AI)

- [x] [AI-Audit][Medium] Ensure resolver requests can carry auth cookies. Implemented resolver constructor with optional `Arc<Jar>` and cookie-provider client wiring.
- [x] [AI-Audit][Medium] Ensure direct URL behavior does not regress while adding site-specific logic. Routed all inputs through resolver registry with `DirectResolver` fallback.
- [x] [AI-Audit][Low] Ensure auth failures provide actionable remediation. Added refresh-cookie guidance in `AuthRequirement` message path.

### Code Review Follow-ups (AI - 2026-02-16)

- [x] [AI-Review][Medium] Add dedicated integration coverage for ScienceDirect URL + DOI + auth-expired scenarios.
- [x] [AI-Review][Low] Ensure metadata extraction handles attribute order variations in meta tags (`name`/`property` before or after `content`).

### Code Review Follow-ups (AI - 2026-02-17)

- [x] [AI-Review][High] Remove `.no_proxy()` from resolver HTTP client - was blocking institutional proxy users.
- [x] [AI-Review][High] Add User-Agent header to resolver HTTP requests - avoids bot detection on ScienceDirect.
- [x] [AI-Review][Medium] Replace `panic!()` in `build_client` with `Result` propagation - library code must not panic.
- [x] [AI-Review][Medium] Expand `html_unescape_basic` with typographic entities (`&ndash;`, `&mdash;`, `&nbsp;`, numeric).
- [x] [AI-Review][Medium] Add `#[tracing::instrument]` to `new()`, `with_base_urls()`, `can_handle()` per project conventions.
- [x] [AI-Review][Medium] Raise `is_auth_page` marker threshold from 2 to 3 to reduce false positives.
- [x] [AI-Review][Medium] Accept `linkinghub.elsevier.com` as valid Elsevier redirect host during DOI resolution.
- [x] [AI-Review][Medium] Add unit tests for `html_unescape_basic`, single-quoted meta attributes, auth threshold, and Elsevier host matching.

## Dev Notes

### Architecture Context

Story 4.5 introduces the first site-specific authenticated resolver in the URL resolution pipeline. It builds on Story 4.3/4.4 cookie capture/storage by reusing cookie-jar state for resolver HTTP requests.

### Implementation Notes

- Added `ScienceDirectResolver` with:
  - `ResolverPriority::Specialized` registration,
  - support for common ScienceDirect URL patterns,
  - support for Elsevier DOI prefix (`10.1016/...`) via DOI endpoint normalization,
  - page-level metadata extraction from citation meta tags,
  - PDF URL extraction from citation tags/JSON hints with PII-based fallback.
- Updated main queue preparation path to run all parsed items through the resolver registry (with direct fallback), allowing specialized URL handlers to execute for URL inputs.
- Added auth-expired detection for login-style HTML and auth status codes with cookie-refresh guidance.

### References

- [Source: _bmad-output/planning-artifacts/epics.md#Story-4.5-ScienceDirect-Resolver]
- [Source: _bmad-output/planning-artifacts/architecture.md#Resolver-Pipeline]
- [Source: _bmad-output/project-context.md#Auth-Flow]
- [Source: src/resolver/sciencedirect.rs]
- [Source: src/main.rs]

## Dev Agent Record

### Agent Model Used

GPT-5 Codex (CLI)

### Debug Log References

- `cargo fmt`
- `cargo clippy -- -D warnings`
- `cargo test --lib resolver::sciencedirect`
- `cargo test --test resolver_integration`
- `cargo test --bin downloader`
- `cargo test --test cli_e2e`

### Completion Notes List

- Added specialized `ScienceDirectResolver` for URL and Elsevier DOI flows.
- Applied cookie-jar state during resolver HTTP requests.
- Added page metadata extraction and PDF URL resolution logic.
- Added auth-expired detection with explicit cookie-refresh suggestion.
- Routed URL inputs through resolver pipeline so site-specific resolvers run for URL inputs.
- Added unit and integration test coverage for core 4.5 behavior.

### File List

- `src/resolver/sciencedirect.rs`
- `src/resolver/mod.rs`
- `src/lib.rs`
- `src/main.rs`
- `tests/resolver_integration.rs`
- `_bmad-output/implementation-artifacts/4-5-sciencedirect-resolver.md`

### Change Log

- 2026-02-16: Story created, implemented, validated, and marked done.
- 2026-02-17: Adversarial code review (Claude Opus 4.6) - 8 fixes applied (2 High, 6 Medium).

## Party Mode Audit (AI)

Audit date: 2026-02-16  
Outcome: pass_with_actions  
Summary: High=0, Medium=2, Low=1

Findings:
- Medium: Ensure resolver can process authenticated URL resolution with cookie jar support.
- Medium: Ensure direct URL handling remains stable after pipeline routing changes.
- Low: Ensure auth-failure copy gives explicit refresh guidance.

## Senior Developer Review (AI)

Reviewer: fierce
Date: 2026-02-16
Outcome: Approve

### Validation Evidence

- `cargo fmt`
- `cargo clippy -- -D warnings`
- `cargo test --lib resolver::sciencedirect`
- `cargo test --test resolver_integration`
- `cargo test --bin downloader`
- `cargo test --test cli_e2e`

## Adversarial Code Review (AI - 2026-02-17)

Reviewer: Claude Opus 4.6
Date: 2026-02-17
Outcome: Approve (after fixes)

### Findings Summary

Issues found: 2 High, 6 Medium, 3 Low = 11 total
Issues fixed: 2 High, 6 Medium = 8 fixed
Issues deferred (Low): 3 (L1: absolutize_url scheme validation, L2: misleading Panics doc, L3: lowercase PII edge case)

### Fixes Applied

- H1: Removed `.no_proxy()` from resolver HTTP client
- H2: Added `User-Agent` header to resolver requests
- M1: Changed `build_client` from `panic!()` to `Result` propagation
- M2: Expanded `html_unescape_basic` with 6 additional entities
- M3: Added `#[tracing::instrument]` to `new()`, `with_base_urls()`, `can_handle()`
- M4: Raised auth-page marker threshold from 2 to 3
- M5: Added `linkinghub.elsevier.com` as accepted Elsevier redirect host
- M6: Added 5 new unit tests (html unescape, single-quoted attrs, auth threshold, host matching)

### Validation Evidence

- `cargo fmt` - clean
- `cargo clippy -- -D warnings` - clean
- `cargo test` - 652 tests passed, 0 failures
- `cargo test --lib resolver::sciencedirect` - 12 tests passed (5 new)
- `cargo test --test resolver_integration` - 11 tests passed
- `cargo test --bin downloader` - 71 tests passed
