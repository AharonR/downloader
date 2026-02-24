# Story 2.3: Crossref DOI Resolution

Status: done

## Story

As a **user**,
I want **DOIs to resolve to downloadable URLs**,
So that **I can download papers by pasting their DOI**.

## Acceptance Criteria

1. **AC1: Crossref API Call**
   - **Given** a valid DOI (detected as `InputType::Doi` by parser)
   - **When** the CrossrefResolver processes it
   - **Then** the Crossref REST API is called at `https://api.crossref.org/works/{doi}`
   - **And** the DOI is URL-encoded in the path
   - **And** the request includes `mailto` query parameter for polite pool access
   - **And** the User-Agent header identifies the application with contact info

2. **AC2: PDF URL Extraction**
   - **Given** a successful Crossref API response
   - **When** the response JSON is parsed
   - **Then** the `message.link` array is searched for PDF URLs
   - **And** links with `content-type: "application/pdf"` are preferred
   - **And** links with `intended-application: "similarity-checking"` are used as fallback
   - **And** if no PDF link is found, `ResolveStep::Redirect` is returned with the DOI URL (`https://doi.org/{doi}`) for the DirectResolver to handle
   - **And** if a PDF link is found, `ResolveStep::Url(ResolvedUrl)` is returned with metadata attached

3. **AC3: Metadata Capture**
   - **Given** a successful Crossref API response
   - **When** metadata is extracted
   - **Then** `title` is captured from `message.title[0]` (first element)
   - **And** `authors` is captured from `message.author` array (formatted as "LastName, FirstName; ...")
   - **And** `year` is captured from `message.published.date-parts[0][0]` (or `message.published-print` / `message.published-online` as fallback)
   - **And** `doi` is stored in metadata
   - **And** metadata is attached to `ResolvedUrl` via `ResolvedUrl::with_metadata()`

4. **AC4: Error Handling with Retry Hints**
   - **Given** the Crossref API returns an error
   - **When** the error is processed
   - **Then** 404 returns `ResolveStep::Failed` with "DOI not found in Crossref" message
   - **And** 429 returns `ResolveStep::Failed` with "Crossref rate limit exceeded" message and retry hint
   - **And** 5xx returns `ResolveStep::Failed` with "Crossref API unavailable" message and retry suggestion
   - **And** network errors return `ResolveStep::Failed` with "Cannot reach Crossref API" message
   - **And** JSON parse errors return `ResolveStep::Failed` with "Unexpected Crossref response" message

5. **AC5: Polite Pool Compliance**
   - **Given** the CrossrefResolver is configured
   - **When** requests are made to Crossref
   - **Then** every request includes `?mailto=contact@example.com` parameter (configurable)
   - **And** User-Agent is set to `downloader/{version} (mailto:{email})`
   - **And** rate limiting headers (`x-rate-limit-limit`, `x-rate-limit-interval`) are logged at debug level

6. **AC6: Resolver Registration**
   - **Given** the CrossrefResolver is registered in the registry
   - **When** a DOI input is resolved
   - **Then** the CrossrefResolver has `General` priority (between Specialized and Fallback)
   - **And** `can_handle()` returns true for `InputType::Doi` only
   - **And** it integrates with the existing registry and resolution loop from Story 2.2

## Tasks / Subtasks

**Dependency chain:** Task 1 is independent. Task 2 depends on Task 1. Tasks 3-4 depend on Task 2. Task 5 depends on Tasks 3-4. Task 6 depends on Task 5. Tasks 7-8 depend on Tasks 3-6. Task 9 is final verification.

- [x] **Task 1: Define Crossref API response types** (AC: 2, 3)
  - [x]Create `src/resolver/crossref.rs` with module doc comment
  - [x]Define serde deserialization structs for Crossref response:
    - `CrossrefResponse { status: String, message: CrossrefMessage }`
    - `CrossrefMessage { title: Option<Vec<String>>, author: Option<Vec<CrossrefAuthor>>, link: Option<Vec<CrossrefLink>>, published: Option<CrossrefDate>, published_print: Option<CrossrefDate>, published_online: Option<CrossrefDate> }`
    - `CrossrefAuthor { given: Option<String>, family: Option<String> }`
    - `CrossrefLink { url: String, content_type: Option<String>, content_version: Option<String>, intended_application: Option<String> }`
    - `CrossrefDate { date_parts: Option<Vec<Vec<Option<i32>>>> }`
  - [x]Use `#[serde(rename_all = "kebab-case")]` where Crossref uses hyphenated field names
  - [x]Use `#[serde(rename = "URL")]` for the uppercase `URL` field in `CrossrefLink`

- [x] **Task 2: Implement CrossrefResolver struct** (AC: 1, 5, 6)
  - [x]Define `CrossrefResolver` struct with fields:
    - `client: reqwest::Client` (owns its own client for API calls)
    - `base_url: String` (default `https://api.crossref.org`, overridable for tests)
    - `mailto: String` (for polite pool)
  - [x]Implement `CrossrefResolver::new(mailto: impl Into<String>) -> Self`
    - Creates reqwest::Client with User-Agent: `downloader/0.1.0 (mailto:{mailto})`
    - Sets `base_url` to `https://api.crossref.org`
    - Connect timeout: 10s, read timeout: 30s
  - [x]Implement `CrossrefResolver::with_base_url(mailto: impl Into<String>, base_url: impl Into<String>) -> Self` for testing
  - [x]Implement `Resolver` trait:
    - `name()` returns `"crossref"`
    - `priority()` returns `ResolverPriority::General`
    - `can_handle()` returns `true` only for `InputType::Doi`
    - `resolve()` calls Crossref API and processes response

- [x] **Task 3: Implement Crossref API call in resolve()** (AC: 1, 4, 5)
  - [x]Build URL: `{base_url}/works/{url_encoded_doi}?mailto={mailto}`
  - [x]URL-encode the DOI using `urlencoding::encode()`
  - [x]Send GET request with the resolver's client
  - [x]Handle HTTP status errors:
    - 404 → `ResolveStep::Failed(ResolveError::resolution_failed(input, "DOI not found in Crossref database"))`
    - 429 → `ResolveStep::Failed(ResolveError::resolution_failed(input, "Crossref rate limit exceeded. Try again in a few seconds."))`
    - 5xx → `ResolveStep::Failed(ResolveError::resolution_failed(input, "Crossref API unavailable. Try again later."))`
  - [x]Handle network errors → `ResolveStep::Failed(ResolveError::resolution_failed(input, "Cannot reach Crossref API"))`
  - [x]Handle JSON parse errors → `ResolveStep::Failed(ResolveError::resolution_failed(input, "Unexpected Crossref API response format"))`
  - [x]Log rate limit headers at debug level: `x-rate-limit-limit`, `x-rate-limit-interval`
  - [x]Add `#[tracing::instrument(skip(self, ctx), fields(doi = %input))]` on `resolve()`

- [x] **Task 4: Implement PDF URL extraction and metadata** (AC: 2, 3)
  - [x]Implement `fn extract_pdf_url(links: &[CrossrefLink]) -> Option<String>`
    - Priority: `content-type == "application/pdf"` first
    - Fallback: `intended-application == "similarity-checking"` or `"text-mining"` with any content-type
    - Return URL of best match
  - [x]Implement `fn extract_metadata(message: &CrossrefMessage) -> HashMap<String, String>`
    - Extract `title` from `message.title[0]`
    - Extract `authors` from `message.author` as "Family, Given; Family2, Given2"
    - Extract `year` from `message.published.date_parts[0][0]`, falling back to `published_print` then `published_online`
    - Include `doi` key
  - [x]In `resolve()`: if PDF URL found, return `ResolveStep::Url(ResolvedUrl::with_metadata(pdf_url, metadata))`
  - [x]In `resolve()`: if no PDF URL found, return `ResolveStep::Redirect(format!("https://doi.org/{doi}"))` so the DOI URL can be tried by DirectResolver
  - [x]Log extraction results at debug level

- [x] **Task 5: Register CrossrefResolver module** (AC: 6)
  - [x]Add `mod crossref;` to `src/resolver/mod.rs`
  - [x]Add `pub use crossref::CrossrefResolver;` to mod.rs
  - [x]Add `CrossrefResolver` to re-exports in `src/lib.rs`
  - [x]Verify the CrossrefResolver can be registered: `registry.register(Box::new(CrossrefResolver::new("user@example.com")))`

- [x] **Task 6: Update ResolveContext (optional enhancement)** (AC: 1) — SKIPPED: no changes needed
  - [x]Evaluate if `ResolveContext` needs any changes for this story
  - [x]If no changes needed, skip this task (current context with `max_redirects` is sufficient since CrossrefResolver owns its own client)

- [x] **Task 7: Write unit tests** (AC: 1-6)
  **In `src/resolver/crossref.rs`:**

  **Serde deserialization tests:**
  - [x]`test_crossref_response_deserialize_full()` - full response with all fields
  - [x]`test_crossref_response_deserialize_minimal()` - response with only required fields
  - [x]`test_crossref_link_deserialize_with_uppercase_url()` - verify `URL` field mapping
  - [x]`test_crossref_author_deserialize_missing_given()` - author with only family name
  - [x]`test_crossref_date_deserialize_partial()` - date with only year (no month/day)

  **PDF URL extraction tests:**
  - [x]`test_extract_pdf_url_prefers_pdf_content_type()` - multiple links, picks `application/pdf`
  - [x]`test_extract_pdf_url_fallback_text_mining()` - no PDF type, uses text-mining link
  - [x]`test_extract_pdf_url_empty_links_returns_none()` - empty link array
  - [x]`test_extract_pdf_url_no_matching_links_returns_none()` - links but none usable

  **Metadata extraction tests:**
  - [x]`test_extract_metadata_full()` - title, authors, year all present
  - [x]`test_extract_metadata_missing_title()` - no title key
  - [x]`test_extract_metadata_multiple_authors()` - formats multiple authors correctly
  - [x]`test_extract_metadata_year_from_published_print()` - fallback to published-print date
  - [x]`test_extract_metadata_no_date()` - no date fields present

  **Resolver trait tests (with wiremock):**
  - [x]`test_crossref_resolver_name()` - returns "crossref"
  - [x]`test_crossref_resolver_priority()` - returns General
  - [x]`test_crossref_resolver_can_handle_doi()` - true for Doi
  - [x]`test_crossref_resolver_cannot_handle_url()` - false for Url
  - [x]`test_crossref_resolver_resolve_success_with_pdf()` - mock 200 with PDF link → ResolveStep::Url
  - [x]`test_crossref_resolver_resolve_no_pdf_redirects()` - mock 200 without PDF link → ResolveStep::Redirect to doi.org
  - [x]`test_crossref_resolver_resolve_404_fails()` - mock 404 → ResolveStep::Failed
  - [x]`test_crossref_resolver_resolve_429_fails()` - mock 429 → ResolveStep::Failed with rate limit message
  - [x]`test_crossref_resolver_resolve_500_fails()` - mock 500 → ResolveStep::Failed
  - [x]`test_crossref_resolver_resolve_includes_metadata()` - verify metadata in ResolvedUrl
  - [x]`test_crossref_resolver_sends_mailto_param()` - verify request URL includes mailto

- [x] **Task 8: Write integration tests** (AC: 6)
  - [x]In `tests/resolver_integration.rs` (existing file, add tests):
    - [x]`test_crossref_resolver_with_registry_doi_input()` - CrossrefResolver + DirectResolver in registry, mock Crossref returning PDF URL
    - [x]`test_crossref_resolver_redirect_to_direct()` - CrossrefResolver returns no PDF, redirects to doi.org URL, DirectResolver handles it
    - [x]`test_crossref_resolver_failure_falls_through()` - CrossrefResolver fails (404), no fallback for DOI → AllResolversFailed

- [x] **Task 9: Run pre-commit checks** (AC: all)
  - [x]`cargo fmt --check`
  - [x]`cargo clippy -- -D warnings`
  - [x]`cargo test` - all existing tests still pass (no regressions)
  - [x]New tests pass

## Dev Notes

### Existing Code to Reuse - DO NOT Reinvent

**Resolver framework (from Story 2.2 - import, don't recreate):**
- `crate::resolver::Resolver` trait - implement this for CrossrefResolver
- `crate::resolver::ResolverPriority` - use `General` for Crossref
- `crate::resolver::ResolveStep` - return variants from `resolve()`
- `crate::resolver::ResolvedUrl` - use `with_metadata()` for PDF URLs
- `crate::resolver::ResolveError` - use `resolution_failed()` for error cases
- `crate::resolver::ResolveContext` - accept in `resolve()` signature
- `crate::resolver::ResolverRegistry` - CrossrefResolver will be registered here
- `crate::resolver::AuthRequirement` - use if auth is detected (future)
- `crate::parser::InputType` - match `InputType::Doi` in `can_handle()`

**HTTP client pattern (from download/client.rs):**
- `reqwest::Client` with `.builder()` pattern for timeouts and User-Agent
- CrossrefResolver creates its OWN client (not reusing HttpClient which is for file downloads)
- Connection pooling via single Client instance
- `.error_for_status()` is NOT used here - we check status manually for specific error handling

**Error pattern (FOLLOW EXACTLY):**
- `ResolveError::resolution_failed(input, reason)` - the helper from error.rs
- What/Why/Fix structure in error messages
- Do NOT create new error variants - use the existing `ResolutionFailed` variant

**Dependencies already in Cargo.toml:**
- `reqwest = "0.13"` with `json` feature - for API JSON parsing
- `serde = "1"` with `derive` feature - for response deserialization
- `serde_json = "1"` - for JSON parsing
- `async-trait = "0.1"` - for Resolver trait
- `tracing` - for structured logging
- `urlencoding = "2"` - for URL-encoding DOIs in path
- `wiremock = "0.6"` (dev) - for mocking Crossref API

**NO new Cargo.toml dependencies required for this story.**

### Architecture Compliance

**From architecture.md - Resolver Architecture:**
```rust
// CrossrefResolver implements the same Resolver trait from Story 2.2
// Priority: General (between Specialized site resolvers and Fallback DirectResolver)
// Pattern: Call API → extract PDF → return ResolveStep::Url or ResolveStep::Redirect
```

**From architecture.md - Resolution Loop:**
- CrossrefResolver returns `ResolveStep::Redirect("https://doi.org/{doi}")` when no PDF link found
- The registry's resolution loop will then find DirectResolver to handle the URL
- This creates a natural DOI → URL resolution chain

**From architecture.md - External Dependencies:**
- Crossref API is listed as a known external dependency
- Polite pool access via mailto parameter is required

**From architecture.md - Module Ownership:**
- `src/resolver/` depends on: `parser` (for InputType)
- CrossrefResolver adds dependency on `reqwest` (already in Cargo.toml)
- No new module dependencies introduced

**From project-context.md:**
- `#[tracing::instrument]` on `resolve()` method
- `#[must_use]` on public constructors
- `Send + Sync` on trait object (reqwest::Client is Send + Sync)
- Import order: std → external → internal
- Never `.unwrap()` in library code
- Unit tests inline with `#[cfg(test)]`
- Use wiremock for HTTP mocking

### Key Design Decisions

**Why CrossrefResolver owns its reqwest::Client:**
The Resolver trait's `resolve()` takes `&self` and `&ResolveContext`. Rather than threading an HTTP client through the context (which would make the context complicated and couple all resolvers to HTTP), each resolver that needs HTTP creates its own client. This follows the existing pattern where `HttpClient` in download/ creates its own `reqwest::Client`. Connection pooling still works within each resolver's lifetime.

**Why `ResolveStep::Redirect` for no-PDF-link case:**
When Crossref returns metadata but no PDF link, we redirect to `https://doi.org/{doi}`. The registry's resolution loop will re-enter with this URL, and the DirectResolver (or a future site-specific resolver) will handle it. This creates a natural fallback chain: CrossrefResolver → doi.org URL → DirectResolver (or site resolver).

**Why `General` priority (not `Specialized`):**
Specialized is reserved for site-specific resolvers (e.g., future ArxivResolver that knows arXiv-specific URLs). CrossrefResolver is a general-purpose DOI resolver that works for any DOI registered with Crossref. Fallback is the DirectResolver. So the priority ordering is: Site-specific → Crossref → Direct URL.

**Why not Unpaywall API:**
The architecture mentions Unpaywall as a future enhancement. Story 2.3 focuses on Crossref only. Unpaywall could be added as another `General` priority resolver later.

**Serde field naming:**
Crossref API uses hyphenated field names (`date-parts`, `content-type`, `intended-application`). Use `#[serde(rename_all = "kebab-case")]` on structs, and `#[serde(rename = "URL")]` for the uppercase URL field in links. This avoids manual renaming of every field.

### Crossref API Reference

**Endpoint:** `GET https://api.crossref.org/works/{doi}?mailto={email}`

**Response structure (key fields):**
```json
{
  "status": "ok",
  "message": {
    "title": ["Paper Title Here"],
    "author": [
      {"given": "John", "family": "Smith"},
      {"given": "Jane", "family": "Doe"}
    ],
    "link": [
      {
        "URL": "https://publisher.com/paper.pdf",
        "content-type": "application/pdf",
        "content-version": "vor",
        "intended-application": "text-mining"
      }
    ],
    "published": {
      "date-parts": [[2024, 1, 15]]
    },
    "published-print": {
      "date-parts": [[2024, 2, 1]]
    }
  }
}
```

**Rate limits (polite pool with mailto):**
- Single records: 10 req/s, 3 concurrent
- Response headers: `x-rate-limit-limit`, `x-rate-limit-interval`

**Error responses:**
- 404: DOI not found → `{"status": "not-found", "message": "..."}`
- 429: Rate limited (Retry-After header may be present)
- 5xx: Server error

### Project Structure Notes

**New files:**
```
src/resolver/
├── crossref.rs         # NEW: CrossrefResolver, API types, extraction logic
```

**Modified files:**
- `src/resolver/mod.rs` - Add `mod crossref;` and `pub use crossref::CrossrefResolver;`
- `src/lib.rs` - Add `CrossrefResolver` to re-exports
- `tests/resolver_integration.rs` - Add Crossref integration tests

**File structure after implementation:**
```
src/resolver/
├── mod.rs          # Add: mod crossref; pub use crossref::CrossrefResolver;
├── error.rs        # UNCHANGED
├── registry.rs     # UNCHANGED
├── direct.rs       # UNCHANGED
└── crossref.rs     # NEW: CrossrefResolver + API types + extraction
```

### Previous Story Intelligence

**From Story 2.2 (Resolver Trait & Registry):**
- `MockResolver` pattern established for testing registry behavior without HTTP
- `ResolveStep::Failed` carries `ResolveError` for informational failure messages
- Resolution loop correctly handles `Redirect` → re-enters loop with new input
- `#[tracing::instrument]` with `skip(self, ctx)` on resolve methods
- `async_trait` required for `Box<dyn Resolver>` dispatch
- Tests organized with `// ==================== Section ====================` headers
- `#[allow(clippy::unwrap_used)]` on test modules

**From Story 2.2 - Code review learnings:**
- Always add `#[tracing::instrument]` on `register()` and `find_handlers()` methods (M2 fix)
- Manual Debug impl for structs containing `Box<dyn Trait>` (M3 fix)
- Include constructors for all public types (L1 fix: AuthRequirement::new())
- Add debug logs showing handler counts before resolution loop (L2 fix)

**From Story 2.1 (DOI Detection & Validation):**
- `InputType::Doi` is the input type for DOIs
- DOIs are normalized to bare format: `10.xxxx/suffix`
- `urlencoding` crate available for URL encoding/decoding
- thiserror error enums with What/Why/Fix structure
- `LazyLock<Regex>` pattern for compile-once patterns (not needed here)

**From Story 2.1 - Code review learnings:**
- Update doc comments that say "future" when implementing the feature
- All tests should use specific assertions, not just `is_ok()`/`is_err()`

### Git Intelligence

Only 2 commits. All resolver code is in uncommitted changes. Files to be aware of:
- `src/resolver/mod.rs` - Has the Resolver trait, types, re-exports
- `src/resolver/error.rs` - Has ResolveError with helper constructors
- `src/resolver/registry.rs` - Has ResolverRegistry with resolution loop
- `src/resolver/direct.rs` - Has DirectResolver (URL passthrough)

### Testing Strategy

**Unit tests (inline `#[cfg(test)]` in `crossref.rs`):**
- Serde deserialization: test JSON → struct mapping for various response shapes
- PDF extraction: test link filtering logic independently
- Metadata extraction: test metadata parsing independently
- Resolver trait: use wiremock to mock Crossref API endpoint
  - Success paths (with/without PDF)
  - Error paths (404, 429, 500)
  - Verify request URL includes mailto parameter

**Integration tests (in `tests/resolver_integration.rs`):**
- Full registry flow: CrossrefResolver + DirectResolver
- Redirect chain: Crossref → doi.org URL → DirectResolver
- Failure cascade: Crossref fails, no fallback

**wiremock pattern for Crossref API:**
```rust
use wiremock::{MockServer, Mock, ResponseTemplate};
use wiremock::matchers::{method, path_regex, query_param};

let mock_server = MockServer::start().await;

Mock::given(method("GET"))
    .and(path_regex(r"/works/10\..+"))
    .and(query_param("mailto", "test@example.com"))
    .respond_with(
        ResponseTemplate::new(200)
            .set_body_json(serde_json::json!({
                "status": "ok",
                "message": {
                    "title": ["Test Paper"],
                    "author": [{"given": "John", "family": "Smith"}],
                    "link": [{
                        "URL": "https://publisher.com/paper.pdf",
                        "content-type": "application/pdf",
                        "content-version": "vor",
                        "intended-application": "text-mining"
                    }],
                    "published": {"date-parts": [[2024, 6, 15]]}
                }
            }))
    )
    .mount(&mock_server)
    .await;

let resolver = CrossrefResolver::with_base_url("test@example.com", mock_server.uri());
```

### Anti-Patterns to Avoid

| Anti-Pattern | Correct Approach |
|---|---|
| Reusing `HttpClient` from download module | Create a dedicated reqwest::Client in CrossrefResolver (different timeout/UA needs) |
| Adding HTTP client to `ResolveContext` | Let CrossrefResolver own its own client |
| Using `.unwrap()` on API response fields | All fields use `Option<T>` and graceful fallback |
| Hardcoding Crossref base URL | Use constructor with default, allow override for tests |
| Creating new error variants for Crossref | Use existing `ResolveError::resolution_failed()` helper |
| Testing with real Crossref API | Use wiremock for all API tests |
| Blocking on rate limit headers | Just log them at debug level, don't implement rate limiting (the download engine handles that) |
| Returning `Err()` from `resolve()` | Return `Ok(ResolveStep::Failed(...))` for expected failures, `Err()` only for unexpected errors |
| Implementing Unpaywall API | That's a future story - this story is Crossref only |
| Making CrossrefResolver handle URLs | It only handles `InputType::Doi` |
| Adding the crossref resolver to main.rs registration | That's a wiring concern for a future integration story; just ensure it CAN be registered |
| Panicking on malformed JSON | Return `ResolveStep::Failed` with parse error message |
| Ignoring `#[serde(rename)]` for hyphenated fields | Crossref uses `content-type`, `date-parts`, etc. - must use serde rename attributes |

### References

- [Source: architecture.md#Resolver-Architecture] - Resolver trait, ResolveStep, resolution loop, priority chain
- [Source: architecture.md#Implementation-Patterns-&-Consistency-Rules] - Naming, imports, errors, async patterns
- [Source: architecture.md#Module-Ownership-Mapping] - Resolver depends on parser
- [Source: architecture.md#Testing-Infrastructure] - wiremock for HTTP mocking
- [Source: project-context.md#Rust-Language-Rules] - Error handling, async, naming
- [Source: project-context.md#Testing-Rules] - Test organization, naming, wiremock patterns
- [Source: project-context.md#Framework-Specific-Rules] - reqwest client reuse, tracing
- [Source: epics.md#Story-2.3] - Original acceptance criteria
- [Source: 2-2-resolver-trait-registry.md] - Resolver framework implementation details
- [Source: 2-1-doi-detection-validation.md] - DOI detection patterns and learnings
- [Source: Crossref REST API docs] - API endpoint, response format, polite pool, rate limits

---

## Dev Agent Record

### Agent Model Used

Claude Haiku 4.5 (claude-haiku-4-5-20251001)

### Debug Log References

N/A

### Completion Notes List

- All 9 tasks completed successfully
- 32 unit tests in `src/resolver/crossref.rs` (serde, extraction, resolver trait, wiremock)
- 3 integration tests added in `tests/resolver_integration.rs` (registry flow with Crossref)
- Task 6 (ResolveContext update) skipped as unnecessary — CrossrefResolver owns its own client
- Review follow-up fixes applied:
  - case-insensitive/parameter-tolerant `content-type` matching for PDF links
  - case-insensitive fallback matching for `intended-application`
  - explicit validation for Crossref response `status == "ok"`
  - tests added for URL-encoded DOI path and User-Agent header
- Validation run after review fixes: `cargo test --lib resolver::crossref::tests::` (32 passed), `cargo test --test resolver_integration test_crossref_` (3 passed)
- Repository contained unrelated in-progress changes; review/fixes were scoped to Story 2.3 files

### Change Log

- **NEW** `src/resolver/crossref.rs` — CrossrefResolver implementation with API types, extraction helpers, 32 unit tests
- **MODIFIED** `src/resolver/mod.rs` — Added `mod crossref;` and `pub use crossref::CrossrefResolver;`
- **MODIFIED** `src/lib.rs` — Added `CrossrefResolver` to resolver re-exports
- **MODIFIED** `tests/resolver_integration.rs` — Added 3 Crossref integration tests
- **MODIFIED** `src/resolver/crossref.rs` — Review follow-up fixes for robust link matching, response status validation, and additional AC-focused tests

### File List

| File | Action | Description |
|------|--------|-------------|
| `src/resolver/crossref.rs` | NEW | CrossrefResolver, API response types, PDF/metadata extraction, 27 unit tests |
| `src/resolver/mod.rs` | MODIFIED | Added crossref module and re-export |
| `src/lib.rs` | MODIFIED | Added CrossrefResolver to public re-exports |
| `tests/resolver_integration.rs` | MODIFIED | Added 3 Crossref registry integration tests |
