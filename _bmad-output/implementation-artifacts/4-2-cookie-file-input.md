# Story 4.2: Cookie File Input

Status: done

<!-- Note: Validation is optional. Run validate-create-story for quality check before dev-story. -->

## Story

As a **user**,
I want **to provide cookies from a file**,
so that **I can use cookies exported from browser extensions**.

## Acceptance Criteria

1. **AC1: Netscape Cookie File Loading**
   - **Given** a valid Netscape-format cookie file at `cookies.txt`
   - **When** I run `downloader --cookies cookies.txt <urls>`
   - **Then** cookies are parsed from the file and loaded into the HTTP client
   - **And** the file's 7-field TAB-separated format is correctly parsed (domain, tailmatch, path, secure, expires, name, value)
   - **And** comment lines (starting with `#`) and blank lines are skipped
   - **And** the optional `# Netscape HTTP Cookie File` header line is accepted but not required

2. **AC2: Stdin Cookie Input**
   - **Given** cookies piped via stdin
   - **When** I run `echo "<cookie-data>" | downloader --cookies - <urls>`
   - **Then** cookies are read from stdin in Netscape format
   - **And** this is distinct from URL stdin input (cannot use both simultaneously)
   - **And** if `--cookies -` is used with no stdin data, a clear error is shown

3. **AC3: Invalid Cookie Format Error**
   - **Given** a cookie file with malformed lines
   - **When** the file is parsed
   - **Then** lines with wrong field count (not 7 TAB-separated fields) produce a clear error
   - **And** the error includes the line number and a description of the expected format
   - **And** valid lines in the file are still loaded (partial success with warnings)

4. **AC4: Domain Matching**
   - **Given** loaded cookies with domain `.example.com`
   - **When** a download request is made to `sub.example.com`
   - **Then** the cookie is sent (tailmatch/subdomain matching works correctly)
   - **And** cookies for `other.com` are NOT sent to `example.com`
   - **And** the `secure` flag is respected (HTTPS-only cookies not sent over HTTP)

5. **AC5: Cookie Security**
   - **Given** cookies are loaded from a file
   - **When** logging is active (including `--verbose` / `-vv` trace level)
   - **Then** cookie values are NEVER logged at any level
   - **And** cookie names and domains may be logged at debug level for troubleshooting
   - **And** `#[instrument(skip(...))]` is used on all functions handling cookie values

## Tasks / Subtasks

- [x] Task 1: Create `src/auth/` module with Netscape cookie parser (AC: 1, 3)
  - [x]1.1: Create `src/auth/mod.rs` with module declarations and public re-exports
  - [x]1.2: Create `src/auth/cookies.rs` with `CookieLine` struct (domain, tailmatch, path, secure, expires, name, value)
  - [x]1.3: Implement `parse_netscape_cookies(reader: impl BufRead) -> Result<Vec<CookieLine>, CookieError>` that parses Netscape format
  - [x]1.4: Implement `CookieError` enum with `InvalidLine { line_number, content, reason }` and `IoError` variants using `thiserror`
  - [x]1.5: Handle comment lines (`#`), blank lines, and the optional `# Netscape HTTP Cookie File` header
  - [x]1.6: Validate field count (exactly 7 TAB-separated), `TRUE`/`FALSE` for tailmatch and secure fields, numeric expires
  - [x]1.7: Add unit tests: valid file, comment lines, blank lines, malformed lines with line numbers, empty file, partial success with warnings

- [x] Task 2: Create cookie loading into reqwest `Jar` (AC: 1, 4)
  - [x]2.1: Implement `load_cookies_into_jar(cookies: &[CookieLine]) -> Arc<reqwest::cookie::Jar>` in `src/auth/cookies.rs`
  - [x]2.2: Convert each `CookieLine` to a `Set-Cookie` string and use `Jar::add_cookie_str(&cookie, &url)` where URL is constructed from domain + path
  - [x]2.3: Handle tailmatch: prefix domain with `.` for `TRUE`, use exact domain for `FALSE`
  - [x]2.4: Handle secure flag: use `https://` for `TRUE`, `http://` for `FALSE` when constructing the URL for `Jar::add_cookie_str`
  - [x]2.5: Handle expires: convert Unix timestamp to `Expires=<date>` in Set-Cookie format; `0` = session cookie (omit Expires)
  - [x]2.6: Add unit tests: domain matching via Jar, secure flag, subdomain matching, expiry handling

- [x] Task 3: Add `--cookies` CLI flag (AC: 1, 2)
  - [x]3.1: Add `--cookies` / `-k` argument to `Args` struct in `src/cli.rs` as `Option<String>` with help text
  - [x]3.2: Add clap validation: if value is `-`, mark as stdin mode; otherwise validate file path exists via `value_parser`
  - [x]3.3: Add conflict detection: `--cookies -` cannot be used simultaneously with stdin URL input (detect and error early in `main.rs`)
  - [x]3.4: Add unit tests for CLI flag parsing, `-` value, and file path validation

- [x] Task 4: Modify `HttpClient` to accept a cookie jar (AC: 1, 4)
  - [x]4.1: Add `HttpClient::with_cookie_jar(jar: Arc<reqwest::cookie::Jar>) -> Self` constructor in `src/download/client.rs`
  - [x]4.2: Pass the jar to `ClientBuilder::cookie_provider(jar)` when building the reqwest Client
  - [x]4.3: Keep `HttpClient::new()` unchanged (no cookies) for backward compatibility
  - [x]4.4: Add unit test: client with cookie jar sends cookies to matching domains (wiremock verification)

- [x] Task 5: Wire cookie loading into main.rs (AC: 1, 2, 5)
  - [x]5.1: After CLI parsing, load cookies from file or stdin based on `args.cookies`
  - [x]5.2: For file: open file, wrap in `BufReader`, call `parse_netscape_cookies()`
  - [x]5.3: For stdin (`-`): read stdin into `BufReader`, call `parse_netscape_cookies()`
  - [x]5.4: Detect conflict: if `--cookies -` and no positional URLs provided (both would need stdin), error with clear message
  - [x]5.5: Log cookie load summary at info level: "Loaded {count} cookies for {n} domains" (no values!)
  - [x]5.6: If parse errors occurred but some cookies loaded, log warnings with line numbers
  - [x]5.7: Create `HttpClient::with_cookie_jar(jar)` when cookies provided, else `HttpClient::new()`

- [x] Task 6: Integration tests (AC: 1, 2, 3, 4, 5)
  - [x]6.1: Integration test: load cookies from file → download succeeds with cookie sent (wiremock verifies Cookie header)
  - [x]6.2: Integration test: cookies matched to correct domains only
  - [x]6.3: Integration test: malformed cookie file produces error with line number
  - [x]6.4: E2E CLI test: `--cookies cookies.txt` flag accepted and processed
  - [x]6.5: Verify cookie values never appear in tracing output (capture tracing subscriber output in test)

## Dev Notes

### Architecture Context

This story creates the `src/auth/` module which is the foundation for all authentication stories (4.2-4.5). However, **only `mod.rs` and `cookies.rs` are needed for this story**. The `storage.rs` and `keychain.rs` files mentioned in the architecture are for stories 4.3-4.4 and should NOT be created now.

The reqwest `cookies` feature is **already enabled** in Cargo.toml (`features = ["json", "cookies", "stream", "gzip"]`), so no dependency changes are needed.

### Key Design Decisions

**1. Cookie Jar Approach: `reqwest::cookie::Jar` (not manual Cookie headers)**

The reqwest `Jar` struct handles domain matching, path matching, secure flag, and expiry automatically. Using `Jar::add_cookie_str()` with a constructed `Set-Cookie` header string and origin URL is the cleanest integration:

```rust
use reqwest::cookie::Jar;
use std::sync::Arc;

let jar = Arc::new(Jar::default());
// For a cookie: .example.com  TRUE  /  FALSE  1700000000  session_id  abc123
let set_cookie = "session_id=abc123; Domain=.example.com; Path=/; Expires=Tue, 14 Nov 2023 22:13:20 GMT";
let url = "http://example.com/".parse().unwrap();
jar.add_cookie_str(set_cookie, &url);

// Then pass to ClientBuilder
let client = Client::builder()
    .cookie_provider(jar)
    .build()?;
```

This approach:
- Leverages reqwest's built-in domain/path/secure matching (don't reinvent)
- Is thread-safe via `Arc<Jar>` (Jar implements `CookieStore`)
- Works with the existing `Client` without modifying `send_request()`

**2. Netscape Format Parsing: Custom parser (no external crate)**

The Netscape cookie format is simple (7 TAB-separated fields per line). A custom parser:
- Avoids adding a dependency for ~50 lines of parsing code
- Gives precise control over error messages with line numbers
- Follows project rules: "No duplicate functionality - check existing deps first"

**3. `HttpClient::with_cookie_jar()` vs modifying `new()`**

Adding a new constructor preserves backward compatibility. All existing code calling `HttpClient::new()` continues to work without cookies. Only `main.rs` needs to choose which constructor to call based on `--cookies` flag.

**4. Stdin conflict detection**

The current CLI reads URLs from stdin when no positional args are provided. `--cookies -` also reads from stdin. These conflict. Detection strategy:
- If `--cookies -` AND no positional URLs: error "Cannot read both cookies and URLs from stdin. Provide URLs as arguments when using --cookies -"
- If `--cookies -` AND positional URLs provided: read cookies from stdin, use positional URLs

### Netscape Cookie File Format Reference

Each line contains 7 TAB-separated fields:

| # | Field | Type | Description |
|---|-------|------|-------------|
| 1 | domain | string | `.example.com` (leading dot = subdomain match) |
| 2 | tailmatch | `TRUE`/`FALSE` | Whether subdomains match |
| 3 | path | string | URL path scope (`/`) |
| 4 | secure | `TRUE`/`FALSE` | HTTPS-only flag |
| 5 | expires | integer | Unix timestamp (0 = session) |
| 6 | name | string | Cookie name |
| 7 | value | string | Cookie value (NEVER log this) |

Example file:
```
# Netscape HTTP Cookie File
.sciencedirect.com	TRUE	/	TRUE	1700000000	session_id	abc123
.example.com	TRUE	/	FALSE	0	pref	dark_mode
```

### Existing Code to Build On

| What | Where | How It Helps |
|------|-------|-------------|
| `HttpClient` struct | `src/download/client.rs:52-54` | Wraps `reqwest::Client` — add `with_cookie_jar()` constructor |
| `HttpClient::new()` | `src/download/client.rs:91-103` | Shows `Client::builder()` pattern — add `.cookie_provider(jar)` |
| `send_request()` | `src/download/client.rs:286+` | Already builds requests with headers — cookies auto-attached by jar |
| `Args` struct | `src/cli.rs` | Add `--cookies` flag here |
| `main()` wire-up | `src/main.rs:187` | `HttpClient::new()` call site — conditionally use `with_cookie_jar()` |
| `DownloadError` | `src/download/error.rs` | Pattern for error types — follow for `CookieError` |
| `reqwest` cookies feature | `Cargo.toml` | Already enabled — `reqwest::cookie::Jar` available |
| `lib.rs` module list | `src/lib.rs:25-29` | Add `pub mod auth;` here |

### Files to Create/Modify

| File | Action | Changes |
|------|--------|---------|
| `src/auth/mod.rs` | **Create** | Module declarations, re-exports for `parse_netscape_cookies`, `CookieLine`, `CookieError` |
| `src/auth/cookies.rs` | **Create** | Netscape cookie parser, `CookieLine` struct, `CookieError` enum, `load_cookies_into_jar()` |
| `src/lib.rs` | Modify | Add `pub mod auth;` and re-export key types |
| `src/cli.rs` | Modify | Add `--cookies` / `-k` argument to `Args` struct |
| `src/download/client.rs` | Modify | Add `HttpClient::with_cookie_jar()` constructor |
| `src/main.rs` | Modify | Wire cookie loading: parse file/stdin → jar → HttpClient |
| `tests/auth_integration.rs` | **Create** | Integration tests for cookie loading + domain matching + download |

### Testing Standards

- Test naming: `test_<unit>_<scenario>_<expected>`
- Use wiremock for HTTP mocking (verify `Cookie` header received)
- Unit tests inline with `#[cfg(test)]` in `src/auth/cookies.rs`
- Integration tests in `tests/auth_integration.rs`
- Run `cargo fmt && cargo clippy -- -D warnings && cargo test` before completion

### Security Checklist

- [ ] Cookie values never logged — use `#[instrument(skip(value, cookie_value, cookies))]`
- [ ] `CookieLine` Debug impl must redact value field
- [ ] No cookie values in error messages
- [ ] Tracing fields: log domain + name only, never value
- [ ] Info log at load time: count + domains only

### Project Structure Notes

- `src/auth/` is a new module — add `pub mod auth;` to `src/lib.rs`
- Only create `mod.rs` and `cookies.rs` — do NOT create `storage.rs` or `keychain.rs` (those are stories 4.3-4.4)
- Follow module pattern: `mod.rs` has `mod` declarations + `pub use` re-exports
- `CookieError` uses `thiserror` (library code), NOT `anyhow`
- `main.rs` error handling for cookie loading uses `anyhow` (binary code)

### References

- [Source: _bmad-output/planning-artifacts/epics.md#Story-4.2-Cookie-File-Input]
- [Source: _bmad-output/planning-artifacts/architecture.md#Authentication-&-Security]
- [Source: _bmad-output/planning-artifacts/architecture.md#Cookie-Storage-Architecture]
- [Source: _bmad-output/planning-artifacts/architecture.md#Complete-Project-Directory-Structure]
- [Source: _bmad-output/project-context.md#Security-Rules]
- [Source: _bmad-output/project-context.md#Module-Structure]
- [Source: src/download/client.rs - HttpClient struct and new() constructor]
- [Source: src/download/error.rs - DownloadError pattern for thiserror usage]
- [Source: src/cli.rs - Args struct for CLI flag patterns]
- [Source: src/lib.rs - Module declarations and re-exports]
- [Source: Cargo.toml - reqwest cookies feature already enabled]
- [Source: https://curl.se/docs/http-cookies.html - Netscape cookie format spec]
- [Source: https://docs.rs/reqwest/latest/reqwest/cookie/struct.Jar.html - reqwest Jar API]

### Previous Story Intelligence (4.1)

- Story 4.1 established the `DownloadError::AuthRequired` variant with `[AUTH]` prefix for error classification
- The auth-required detection triggers the message "Run `downloader auth capture` to authenticate" — story 4.2's cookie loading is the first step toward fulfilling that promise
- wiremock `set_body_string()` overrides Content-Type to text/plain — must use `set_body_bytes()` when setting custom Content-Type headers (lesson from 4.1)
- 543 tests passing at story 4.1 completion — baseline for regression check
- Code review found overly broad login pattern `/auth` matching `/authors/` — fixed to `/auth/` — be careful with substring matching

### Git Intelligence

- Only 2 commits in history (initial + first version) — all epics 1-3 were developed before first commit
- Current code compiles and tests pass
- Story 4.1 changes are unstaged but present in working tree

### Review Follow-ups (AI)

- [ ] [AI-Audit][Medium] Short flag `-k` conflicts with curl's well-known `-k` / `--insecure` convention (skip TLS verification). Users who use curl regularly will expect `-k` to mean insecure mode. Use a different short flag (e.g., `-C` for Cookies) or omit the short form entirely and only provide `--cookies`. [src/cli.rs]
- [ ] [AI-Audit][Medium] `CookieLine` struct MUST implement a custom `Debug` that redacts the `value` field. The security checklist mentions this but no task subtask explicitly covers it. Without this, any `debug!("{:?}", cookie_line)` or `format!("{:?}", ...)` will leak cookie values. Add to Task 1.2: implement `fmt::Debug` manually with `value: "[REDACTED]"`. [src/auth/cookies.rs]
- [ ] [AI-Audit][Medium] Zero-valid-cookies edge case: AC3 defines "partial success with warnings" but does not specify behavior when ALL lines in a non-empty file are malformed. The parser should detect this and return a clear error: "No valid cookies found in file ({n} lines failed to parse)" rather than silently proceeding with an empty cookie jar. [src/auth/cookies.rs, src/main.rs]
- [ ] [AI-Audit][Medium] `Jar::add_cookie_str` URL scheme is critical for secure cookies: if a cookie has `Secure` flag but the origin URL uses `http://`, the jar will accept it but may not return it for HTTPS requests (implementation-dependent behavior). Task 2.4 mentions the scheme but this needs an explicit integration test: load a secure cookie, make an HTTPS request via wiremock, verify cookie is sent. [src/auth/cookies.rs, tests/auth_integration.rs]
- [ ] [AI-Audit][Low] Windows CRLF line endings: `BufRead::lines()` strips `\n` but leaves trailing `\r` in the last field (cookie value). Parser must `.trim_end()` each line or strip `\r` explicitly, otherwise cookie values will have an invisible `\r` appended. Add a unit test with CRLF input. [src/auth/cookies.rs]
- [ ] [AI-Audit][Low] Expires field edge cases: Very large timestamps (e.g., `9999999999`), zero, and negative values from corrupted cookie exports should be handled gracefully. Ensure the `u64` or `i64` parse doesn't panic and that overflow produces a reasonable default (treat as session cookie or far-future expiry). [src/auth/cookies.rs]

## Party Mode Audit (AI)

Audit date: 2026-02-16
Outcome: pass_with_actions
Summary: High=0, Medium=4, Low=2

Findings:
- Medium: `-k` short flag conflicts with curl's `--insecure` convention. Users of curl will be confused. Use `-C` or no short flag. [src/cli.rs]
- Medium: `CookieLine` custom Debug impl for value redaction is in security checklist but not in any task subtask. Dev agent may `#[derive(Debug)]` and leak values. [src/auth/cookies.rs]
- Medium: Zero-valid-cookies from non-empty file not addressed. Parser should error rather than silently creating empty jar. [src/auth/cookies.rs]
- Medium: `Jar::add_cookie_str` URL scheme matters for secure cookies. Needs dedicated integration test to verify cookies actually sent for HTTPS. [src/auth/cookies.rs, tests/auth_integration.rs]
- Low: CRLF line endings leave `\r` in cookie value. Parser must trim. [src/auth/cookies.rs]
- Low: Expires field edge cases (very large, negative) could cause parse failures. [src/auth/cookies.rs]

## Dev Agent Record

### Agent Model Used

Claude Opus 4.6 (claude-opus-4-6)

### Debug Log References

### Completion Notes List

- All 6 tasks and 30 subtasks completed
- Created `src/auth/` module with Netscape cookie parser and reqwest Jar loader
- `CookieLine` has custom `Debug` impl that redacts cookie values (security)
- Error messages also redact cookie values via `redact_line_for_error()`
- `NoCookiesFound` error returned when all lines in non-empty file are malformed
- `--cookies` flag uses long-only form (no `-k` short flag) per audit finding
- CRLF line endings handled via `trim_end()` in parser
- `HttpClient::with_cookie_jar()` constructor uses `cookie_provider()` for automatic domain matching
- Stdin conflict detection: `--cookies -` with no positional URLs produces clear error
- 611 tests pass (405 lib + 52 bin + 6 auth_integration + others), 0 failures
- `cargo fmt && cargo clippy -- -D warnings` clean
- 4 of 6 AI-Audit follow-ups addressed; 2 Low-severity items (CRLF, large expires) also addressed in implementation

### Code Review Fixes Applied

- **MEDIUM #1**: Fixed broken doc reference on `value()` method — replaced non-existent `to_set_cookie_string` reference with security note
- **MEDIUM #2**: Added `warn!()` log in `build_set_cookie_string()` when `unix_to_http_date()` returns `None` (timestamp overflow) — was silently omitting Expires attribute
- **MEDIUM #3**: Strengthened integration test `test_cookie_jar_sends_cookie_to_matching_domain` to verify actual cookie content (`session_id=test_value_123`) instead of just `header_exists("cookie")`
- 3 LOW issues documented but not fixed (acceptable per auto-fix-high-medium mode)

### File List

- `src/auth/mod.rs` — **Created**: module declarations, re-exports for cookies types
- `src/auth/cookies.rs` — **Created**: Netscape cookie parser, CookieLine struct (with custom Debug), CookieError enum, load_cookies_into_jar(), 26 unit tests
- `src/lib.rs` — Modified: added `pub mod auth;` and auth type re-exports
- `src/cli.rs` — Modified: added `--cookies` flag (long-only, `Option<String>`), 4 new CLI tests
- `src/download/client.rs` — Modified: added `HttpClient::with_cookie_jar(Arc<Jar>)` constructor
- `src/main.rs` — Modified: cookie loading from file/stdin, stdin conflict detection, conditional `HttpClient` creation
- `tests/auth_integration.rs` — **Created**: 6 integration tests (cookie sending, domain isolation, malformed file, subdomain matching, security)
