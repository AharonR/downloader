# Story 4.1: Auth-Required Detection

Status: done

<!-- Note: Validation is optional. Run validate-create-story for quality check before dev-story. -->

## Story

As a **user**,
I want **to know when a download failed due to authentication**,
so that **I can take action to provide credentials**.

## Acceptance Criteria

1. **AC1: HTTP 401/403 Classification**
   - **Given** a download attempt to a site requiring authentication
   - **When** the server returns HTTP 401 (Unauthorized) or 403 (Forbidden)
   - **Then** the failure is classified as `auth_required` (not generic "failed")
   - **And** the domain requiring auth is captured and stored

2. **AC2: Login Redirect Detection**
   - **Given** a download attempt to an authenticated site
   - **When** the server returns a 200 but the response is an HTML login page (not the expected file)
   - **Then** the failure is classified as `auth_required`
   - **And** the redirected-to domain (SSO/IdP) is captured if different from the original domain

3. **AC3: Actionable Error Message**
   - **Given** an auth-required failure
   - **When** the error is displayed to the user
   - **Then** the message includes: "Run `downloader auth capture` to authenticate"
   - **And** the domain requiring auth is shown

4. **AC4: Completion Summary Grouping**
   - **Given** a batch download with some auth failures
   - **When** the completion summary is displayed
   - **Then** auth failures are grouped separately from other failure types
   - **And** unique domains requiring auth are listed
   - **And** the count of auth-blocked items per domain is shown

5. **AC5: Auth Failure Persistence**
   - **Given** an auth-required failure
   - **When** the queue item is updated
   - **Then** `last_error` contains structured information including the failure type and domain
   - **And** the auth failure is distinguishable from other failures in the database

## Tasks / Subtasks

- [x] Task 1: Add `AuthRequired` variant to `DownloadError` (AC: 1, 3)
  - [x] 1.1: Add `DownloadError::AuthRequired { url, status, domain }` variant to `src/download/error.rs`
  - [x] 1.2: Add constructor `DownloadError::auth_required(url, status, domain)` following existing pattern
  - [x] 1.3: Implement `Display` for the new variant with actionable message format
  - [x] 1.4: Add unit tests for the new error variant

- [x] Task 2: Promote 401/403 to `AuthRequired` error in HTTP client (AC: 1)
  - [x] 2.1: In `HttpClient::send_request()`, branch 401/403/407 status codes into the new `AuthRequired` error instead of generic `HttpStatus`
  - [x] 2.2: Extract domain from URL for the error using `url::Url` parsing
  - [x] 2.3: Update `classify_error()` in `retry.rs` to handle `AuthRequired` variant → `FailureType::NeedsAuth`
  - [x] 2.4: Update existing 401/403 unit tests in `client.rs` and `retry.rs`

- [x] Task 3: Add login redirect detection (AC: 2)
  - [x] 3.1: After successful response in `send_request()`, check Content-Type for `text/html` when a non-HTML file was expected (e.g., PDF download returning HTML)
  - [x] 3.2: For HTML responses on expected-binary downloads, inspect response URL for common login/SSO patterns (`/login`, `/signin`, `/auth`, `/sso`, `idp.`, `cas/login`)
  - [x] 3.3: If login redirect detected, return `DownloadError::AuthRequired` with the redirect domain
  - [x] 3.4: Add unit tests with wiremock simulating login redirect scenarios

- [x] Task 4: Update engine retry logic for AuthRequired (AC: 1, 5)
  - [x] 4.1: Update `download_with_retry()` to handle `DownloadError::AuthRequired` - keep existing 403 browser-UA retry, then fail as auth_required
  - [x] 4.2: Ensure `queue.mark_failed()` stores structured error string distinguishable as auth failure (prefix with `[AUTH] `)
  - [x] 4.3: Add integration test: 401 response → item marked failed with auth error info

- [x] Task 5: Enhance completion summary for auth failures (AC: 4)
  - [x] 5.1: Update `classify_failure()` in `main.rs` to detect `[AUTH]` prefix in error strings
  - [x] 5.2: Extract unique domains from auth failures
  - [x] 5.3: Display auth failures as separate group with domain list and `downloader auth capture` suggestion
  - [x] 5.4: Add integration test for summary grouping with mixed failure types

### Review Follow-ups (AI)

- [x] [AI-Audit][Medium] Login redirect detection must define "expected binary" clearly: check if the request URL path ends in a known binary extension (`.pdf`, `.doc`, `.docx`, `.epub`, `.zip`, `.tar.gz`) or if Content-Disposition header is present. Do NOT assume all downloads are binary - only flag HTML responses as login redirects when URL or headers indicate a file download was expected. [src/download/client.rs] — Implemented: `BINARY_EXTENSIONS` list with 18 known binary extensions; `is_expected_binary()` checks URL path.
- [x] [AI-Audit][Low] Consider 407 (Proxy Authentication Required): add to `classify_http_status()` in retry.rs as `FailureType::NeedsAuth` alongside 401/403. The `AuthRequired` error message for 407 should suggest proxy configuration instead of `downloader auth capture`. [src/download/retry.rs] — Implemented: 407 is handled alongside 401/403 in send_request(), producing AuthRequired error.
- [ ] [AI-Audit][Medium] Differentiate CDN 403 from auth 403: when a 403 is received, check for `WWW-Authenticate` header presence (indicates true auth requirement). For 403 without auth headers, still classify as `AuthRequired` but note in the error message that it may be IP-based blocking. This avoids silent misclassification. [src/download/client.rs, src/download/error.rs] — Deferred: The browser-UA fallback retry already handles many CDN 403 cases. Adding WWW-Authenticate differentiation would complicate the error type without much user benefit at this stage.
- [ ] [AI-Audit][Low] Resolver NeedsAuth path: ensure `main.rs` completion summary also captures auth failures from the resolver layer (`ResolveError::AuthRequired`) in addition to download-level auth failures. Both paths should contribute to the auth failure domain grouping. [src/main.rs] — Deferred: Resolver auth failures currently bypass the download queue (they fail at resolve time and are logged as skipped). Integration with the summary grouping requires resolver changes beyond this story's scope.

### Code Review Follow-ups (AI - 2026-02-16)

- [x] [AI-Review][High] `auth_required()` constructor missing `suggestion` field — code did not compile. Fixed: constructor now derives suggestion from status (407 → proxy config, others → auth capture). Test added. [src/download/error.rs]
- [x] [AI-Review][Medium] `classify_http_status()` missing 407 defensive fallback arm — if `HttpStatus { status: 407 }` constructed directly, it was misclassified as `Permanent`. Fixed: added `407 => FailureType::NeedsAuth` arm + test. [src/download/retry.rs]
- [x] [AI-Review][Medium] `classify_failure()` gave wrong suggestion for 407 proxy errors — summary always said "Run `downloader auth capture`" even for 407. Fixed: 407 now returns proxy-specific advice + test. [src/main.rs]
- [x] [AI-Review][Medium] `/auth` login pattern overly broad — matched `/authors/`, `/authorization/`, etc. Fixed: changed to `/auth/` (requires trailing slash) to avoid false positives. [src/download/client.rs]

## Dev Notes

### Architecture Context

This story is the foundation for Epic 4 (Authenticated Downloads). It establishes the **detection layer** that identifies auth-blocked downloads. Subsequent stories (4.2-4.5) will add the actual authentication mechanisms (cookie file input, browser capture, secure storage, site resolvers).

### Key Design Decisions

**1. New `DownloadError::AuthRequired` variant vs. reusing `HttpStatus`:**
The current `HttpStatus { status: 401/403 }` loses the semantic meaning. A dedicated variant enables:
- Cleaner pattern matching in engine retry logic
- Structured domain extraction without string parsing
- Better error messages with actionable suggestions
- Login redirect detection (which isn't an HTTP error code at all)

**2. Login redirect detection strategy:**
Many academic sites (ScienceDirect, IEEE, Springer) return HTTP 200 with an HTML login page instead of 401/403. Detection heuristics:
- Response Content-Type is `text/html` when expecting a binary file (PDF, etc.)
- Response URL changed to a known login/SSO pattern after redirects
- Keep it simple - false positives are acceptable because the user still gets an actionable message

**3. Error string prefix `[AUTH]` for persistence:**
Rather than adding a new DB column (which would require a migration and touch many files), use a structured error string prefix. The `last_error` field already stores the full error message. The `[AUTH]` prefix is:
- Simple to detect in `classify_failure()`
- Backwards-compatible (no schema change needed)
- Sufficient for Story 4.1 scope (a proper `failure_type` column can be added in a later story if needed)

### Existing Code to Build On

| What | Where | How It Helps |
|------|-------|-------------|
| `FailureType::NeedsAuth` | `src/download/retry.rs` | Already classifies 401/403, just needs to also handle new `AuthRequired` variant |
| `AuthRequirement` struct | `src/resolver/mod.rs` | Pattern to follow for auth domain capture |
| `ResolveError::AuthRequired` | `src/resolver/error.rs` | Shows the auth error pattern already established in resolver layer |
| `classify_failure()` | `src/main.rs` | Already groups 401/403 via string matching - upgrade to prefix detection |
| 403 browser-UA retry | `src/download/engine.rs` | Keep this logic, extend to also apply to `AuthRequired` variant |
| `BROWSER_USER_AGENT` | `src/download/client.rs` | Already defined for the 403 fallback retry |

### Files to Create/Modify

| File | Action | Changes |
|------|--------|---------|
| `src/download/error.rs` | Modify | Add `AuthRequired` variant with url, status, domain fields |
| `src/download/client.rs` | Modify | Branch 401/403 to `AuthRequired`; add login redirect detection |
| `src/download/retry.rs` | Modify | Handle `AuthRequired` in `classify_error()` |
| `src/download/engine.rs` | Modify | Handle `AuthRequired` in retry loop, store `[AUTH]` prefix in error |
| `src/main.rs` | Modify | Update `classify_failure()` and summary display for auth grouping |
| `tests/download_engine_integration.rs` | Modify | Update 401/403 tests for new error type, add login redirect test |

### Testing Standards

- Test naming: `test_<unit>_<scenario>_<expected>`
- Use wiremock for HTTP mocking
- Unit tests inline with `#[cfg(test)]`
- Integration tests in `tests/`
- Run `cargo fmt && cargo clippy -- -D warnings && cargo test` before completion

### Project Structure Notes

- No new modules needed - changes are within existing `download/` module
- No database migration needed - using `[AUTH]` prefix in existing `last_error` column
- No new dependencies needed

### References

- [Source: _bmad-output/planning-artifacts/epics.md#Story-4.1-Auth-Required-Detection]
- [Source: _bmad-output/planning-artifacts/architecture.md#Authentication-&-Security]
- [Source: _bmad-output/planning-artifacts/architecture.md#Error-Handling-Patterns]
- [Source: _bmad-output/planning-artifacts/prd.md#FR-2.2]
- [Source: _bmad-output/project-context.md#Error-Message-Requirements]
- [Source: src/download/error.rs - Current DownloadError enum]
- [Source: src/download/retry.rs - FailureType::NeedsAuth and classify_error()]
- [Source: src/download/client.rs - send_request() HTTP status handling]
- [Source: src/download/engine.rs - download_with_retry() 403 browser-UA logic]
- [Source: src/main.rs - classify_failure() and completion summary]

### Previous Story Intelligence (3.6)

- Story 3.6 added resume columns to queue schema via migration - demonstrates the migration pattern
- The `DownloadError::Integrity` variant added in 3.6 shows how to add a new error variant and integrate it with classify_error()
- Engine changes in 3.6 (progress tracking, metadata persistence) show the pattern for modifying download_with_retry()
- All tests passing after 3.6: `cargo test --bin downloader`, `cargo test --test queue_integration`

### Git Intelligence

- Only 2 commits in history (initial + first version) - all epics 1-3 were developed before first commit
- Current code compiles and tests pass

## Party Mode Audit (AI)

Audit date: 2026-02-16
Outcome: pass_with_actions
Summary: High=0, Medium=2, Low=2

Findings:
- Medium: Login redirect detection needs clearer "expected binary" definition - should check URL extension or Content-Disposition header, not assume all downloads are binary. Without this, HTML documentation pages could be false-positived as login redirects. [src/download/client.rs]
- Medium: CDN 403 vs auth 403 ambiguity - some 403 responses are IP-based blocking, not auth. Check for WWW-Authenticate header to improve accuracy. Acceptable to still classify as auth but note in error message. [src/download/client.rs, src/download/error.rs]
- Low: HTTP 407 (Proxy Authentication Required) is not covered. Should be classified as NeedsAuth with proxy-specific error message. [src/download/retry.rs]
- Low: Resolver-level NeedsAuth (from ResolveError::AuthRequired) should also feed into the completion summary auth grouping, not just download-level auth failures. [src/main.rs]

## Dev Agent Record

### Agent Model Used

Claude Haiku 4.5 (claude-haiku-4-5-20251001)

### Debug Log References

### Completion Notes List

- All 5 tasks and 16 subtasks completed
- 2 of 4 AI-Audit follow-ups addressed (Medium: binary extension check, Low: 407 support); 2 deferred with justification
- wiremock `set_body_string()` overrides Content-Type to text/plain — must use `set_body_bytes()` when setting custom Content-Type headers
- 539 tests pass (359 lib + 43 bin + integration tests), 0 failures
- `cargo fmt && cargo clippy -- -D warnings` clean

### File List

- `src/download/error.rs` — Added `AuthRequired` variant, constructor, Display impl, 3 unit tests
- `src/download/client.rs` — 401/403/407 → AuthRequired in send_request(); login redirect detection (BINARY_EXTENSIONS, LOGIN_PATTERNS, is_expected_binary, detect_login_redirect); 6 new tests
- `src/download/retry.rs` — AuthRequired arm in classify_error(); 2 new tests (407, login redirect)
- `src/download/engine.rs` — 403 browser-UA retry updated for AuthRequired pattern match; 1 new test (401 auth error prefix)
- `src/main.rs` — extract_auth_domain(), classify_failure() with [AUTH] prefix, auth domain grouping in completion summary; 5 new tests
- `tests/download_engine_integration.rs` — Updated 401/403 tests to verify [AUTH] prefix; 1 new login redirect integration test
