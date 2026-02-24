# Story 2.1: DOI Detection & Validation

Status: done

## Story

As a **user**,
I want **DOIs to be automatically recognized in my input**,
So that **I can paste DOIs without special formatting**.

## Acceptance Criteria

1. **AC1: DOI Pattern Detection**
   - **Given** text input containing DOIs
   - **When** the parser processes the input
   - **Then** DOI patterns matching `10.XXXX/suffix` are detected
   - **And** the regex handles registrant codes of 4+ digits, including nested registrants with dots (e.g., `10.1234/...`, `10.1000.10/...`)
   - **And** suffix can contain alphanumeric chars, `-`, `.`, `_`, `:`, `/`
   - **And** DOIs are extracted from surrounding text (mixed with prose, numbered lists, etc.)
   - **And** DOI-like sequences that are NOT DOIs are not matched (e.g., `10.5/10` in "rated 10.5/10", version numbers like `v10.1234/rc1`)
   - **And** parentheses in DOI suffix are handled: closing `)` is included only if a matching `(` exists in the suffix (e.g., `10.1002/(SICI)1097` keeps parens, but `(10.1234/example)` strips outer parens)

2. **AC2: DOI URL Recognition**
   - **Given** DOIs formatted as URLs
   - **When** the parser processes `https://doi.org/10.1234/example` or `https://dx.doi.org/10.1234/example`
   - **Then** they are recognized as DOIs (not plain URLs)
   - **And** the DOI is extracted from the URL path
   - **And** `http://` variants are also recognized
   - **And** URL-encoded DOIs are decoded (e.g., `%2F` → `/`)

3. **AC3: DOI Normalization**
   - **Given** a detected DOI in any format
   - **When** normalization is applied
   - **Then** URL prefixes (`https://doi.org/`, `https://dx.doi.org/`) are stripped
   - **And** `doi:` or `DOI:` text prefixes are stripped
   - **And** leading/trailing whitespace is trimmed
   - **And** the normalized value is the bare DOI (e.g., `10.1234/example`)
   - **And** the `raw` field preserves the original input text

4. **AC4: Invalid DOI Reporting**
   - **Given** text that looks like a DOI but is malformed
   - **When** validation fails
   - **Then** the error includes What/Why/Fix structure
   - **And** `10.` prefix without valid registrant code is reported
   - **And** DOI without suffix (nothing after `/`) is reported
   - **And** invalid DOIs are added to `ParseResult.skipped` (same pattern as invalid URLs)

5. **AC5: Integration with parse_input**
   - **Given** text containing both URLs and DOIs
   - **When** `parse_input()` processes the text
   - **Then** DOIs are extracted alongside URLs
   - **And** DOI items have `input_type == InputType::Doi`
   - **And** `doi.org` URLs are classified as DOI (not URL)
   - **And** `ParseResult` counts include DOIs
   - **And** a `dois()` iterator method is available on `ParseResult`
   - **And** duplicate DOIs in input produce duplicate `ParsedItem`s (no dedup at parser level - queue handles dedup)

## Tasks / Subtasks

**Dependency chain:** Tasks 1-5 are independent (any order). Task 6 depends on Tasks 1-5. Task 7 depends on Task 6. Tasks 8-9 depend on Tasks 1-6. Task 10 is final verification.

- [x] **Task 1: Create `src/parser/doi.rs` module** (AC: 1, 2, 3)
  - [x] Create file with module doc comment matching url.rs style
  - [x] Define `DOI_PATTERN: LazyLock<Regex>` for bare DOIs: `10\.\d{4,9}(?:\.\d+)*/[^\s<>"']+`
    - Note: `(?:\.\d+)*` handles nested registrants like `10.1000.10/example`
  - [x] Define `DOI_URL_PATTERN: LazyLock<Regex>` for doi.org URLs: `https?://(?:dx\.)?doi\.org/(10\.\d{4,9}(?:\.\d+)*/[^\s<>"']+)`
  - [x] Define `DOI_PREFIX_PATTERN: LazyLock<Regex>` for `DOI:` prefixed: `(?i)doi:\s*(10\.\d{4,9}(?:\.\d+)*/[^\s<>"']+)`
  - [x] Implement `pub fn extract_dois(input: &str) -> Vec<DoiExtractionResult>` following `extract_urls` pattern
  - [x] Use `pub type DoiExtractionResult = Result<ParsedItem, ParseError>;`

- [x] **Task 2: Implement DOI validation** (AC: 1, 4)
  - [x] Implement `fn validate_doi(raw_doi: &str) -> Result<String, ParseError>` returning normalized DOI
  - [x] Validate registrant code is 4+ digits after `10.` (including nested like `10.1000.10`)
  - [x] Validate suffix exists (non-empty after the first `/`)
  - [x] Reuse `clean_url_trailing` from url.rs: change its visibility to `pub(crate)` in url.rs and import in doi.rs (DO NOT duplicate)
  - [x] Implement `clean_doi_parens()`: if captured DOI ends with `)` but has no matching `(` in suffix, strip the trailing `)`
  - [x] Return `ParseError::InvalidDoi` for malformed DOIs (new variant needed)

- [x] **Task 3: Implement DOI normalization** (AC: 2, 3)
  - [x] Implement `fn normalize_doi(input: &str) -> String`
  - [x] Strip `https://doi.org/` and `https://dx.doi.org/` prefixes
  - [x] Strip `http://` variants of the above
  - [x] Strip `doi:` prefix (case-insensitive)
  - [x] URL-decode the DOI string (`urlencoding::decode`)
  - [x] Trim whitespace
  - [x] Return bare DOI starting with `10.`

**Call chain in `extract_dois()`:** For each regex match: `normalize_doi(raw)` → strips prefixes → `validate_doi(normalized)` → checks structure, cleans trailing chars → returns `ParsedItem::doi(raw, validated)`

- [x] **Task 4: Add DOI error variant to ParseError** (AC: 4)
  - [x] Add `InvalidDoi` variant to `ParseError` enum in `src/parser/error.rs`:
    ```rust
    #[error("invalid DOI '{doi}': {reason}\n  Suggestion: {suggestion}")]
    InvalidDoi { doi: String, reason: String, suggestion: String }
    ```
  - [x] Add helper constructors: `ParseError::invalid_doi(doi, reason)`, `ParseError::doi_no_suffix(doi)`
  - [x] Update match in `parse_input()` to handle `InvalidDoi` for skipped list

- [x] **Task 5: Add `ParsedItem::doi()` constructor** (AC: 5)
  - [x] Add to `src/parser/input.rs`:
    ```rust
    pub fn doi(raw: impl Into<String>, normalized: impl Into<String>) -> Self {
        Self::new(raw, InputType::Doi, normalized)
    }
    ```
  - [x] Add `dois()` iterator to `ParseResult`:
    ```rust
    pub fn dois(&self) -> impl Iterator<Item = &ParsedItem> {
        self.items.iter().filter(|item| item.input_type == InputType::Doi)
    }
    ```

- [x] **Task 6: Integrate DOI extraction into `parse_input()`** (AC: 5)
  - [x] Add `mod doi;` to `src/parser/mod.rs`
  - [x] Add `pub use doi::extract_dois;` to mod.rs exports
  - [x] In `parse_input()`: call `extract_dois(input)` FIRST, then `extract_urls(input)`
  - [x] **CRITICAL - DOI/URL dedup mechanism:** After both extractions complete, post-filter URL results: remove any URL item where the parsed URL host is `doi.org` or `dx.doi.org`. This is simple and avoids modifying the URL extractor's input.
    ```rust
    // Post-filter: DOIs win over doi.org URLs
    let url_results: Vec<_> = url_results.into_iter().filter(|r| {
        match r {
            Ok(item) => {
                let parsed = url::Url::parse(&item.value).ok();
                !parsed.map_or(false, |u| {
                    matches!(u.host_str(), Some("doi.org") | Some("dx.doi.org"))
                })
            }
            Err(_) => true, // Keep errors for skipped list
        }
    }).collect();
    ```
  - [x] Add `InvalidDoi` match arm to the error handling block (currently handles `InvalidUrl` and `UrlTooLong` on lines 99-105 of mod.rs):
    ```rust
    ParseError::InvalidDoi { doi, .. } => {
        result.add_skipped(doi.clone());
    }
    ```
  - [x] Update logging to include DOI counts: `info!(urls = url_count, dois = doi_count, ...)`

- [x] **Task 7: Register module in lib.rs** (AC: 5)
  - [x] Verify `extract_dois` is accessible from `downloader_core::parser::extract_dois`
  - [x] No changes to lib.rs needed (already re-exports via `parser` module)

- [x] **Task 8: Write unit tests in `src/parser/doi.rs`** (AC: 1-4)
  **Happy path tests:**
  - [x] `test_extract_dois_bare_doi_detected()` - `10.1234/example`
  - [x] `test_extract_dois_long_registrant_detected()` - `10.12345678/example`
  - [x] `test_extract_dois_nested_registrant_detected()` - `10.1000.10/example` (nested registrant with dots)
  - [x] `test_extract_dois_complex_suffix_detected()` - `10.1038/s41586-024-07386-0`
  - [x] `test_extract_dois_elsevier_suffix_detected()` - `10.1016/j.cell.2024.01.001`
  - [x] `test_extract_dois_doi_url_detected()` - `https://doi.org/10.1234/example`
  - [x] `test_extract_dois_dx_doi_url_detected()` - `https://dx.doi.org/10.1234/example`
  - [x] `test_extract_dois_http_doi_url_detected()` - `http://doi.org/10.1234/example`
  - [x] `test_extract_dois_doi_prefix_detected()` - `DOI: 10.1234/example`
  - [x] `test_extract_dois_doi_prefix_lowercase_detected()` - `doi:10.1234/example`
  - [x] `test_extract_dois_multiple_in_text()` - several DOIs in one input
  - [x] `test_extract_dois_from_mixed_text()` - DOIs embedded in prose

  **Normalization tests:**
  - [x] `test_normalize_doi_strips_url_prefix()` - input `https://doi.org/10.1234/x` → `10.1234/x`
  - [x] `test_normalize_doi_strips_doi_prefix()` - input `DOI: 10.1234/x` → `10.1234/x`
  - [x] `test_normalize_doi_trims_whitespace()` - input `  10.1234/x  ` → `10.1234/x`
  - [x] `test_normalize_doi_url_decodes()` - input `https://doi.org/10.1002%2F(SICI)1097-4636` → decoded

  **Validation error tests:**
  - [x] `test_validate_doi_rejects_no_suffix()` - `10.1234/` → error
  - [x] `test_validate_doi_rejects_short_registrant()` - `10.12/example` → error
  - [x] `test_validate_doi_rejects_no_registrant()` - `10./example` → error

  **Edge case & trailing punctuation tests:**
  - [x] `test_extract_dois_trailing_period_cleaned()` - `10.1234/example.` → strips trailing `.`
  - [x] `test_extract_dois_trailing_comma_cleaned()` - `10.1234/example,` → strips trailing `,`
  - [x] `test_extract_dois_in_parentheses()` - `(10.1234/example)` → strips outer parens
  - [x] `test_extract_dois_parens_in_suffix_preserved()` - `10.1002/(SICI)1097-4636` → keeps internal parens
  - [x] `test_extract_dois_empty_input_returns_empty()` - edge case

  **False-positive prevention tests (CRITICAL):**
  - [x] `test_extract_dois_ignores_version_number()` - `v10.1234/rc1` should NOT match
  - [x] `test_extract_dois_ignores_score_fraction()` - `rated 10.5/10` should NOT match (registrant too short)
  - [x] `test_extract_dois_ignores_ip_like_pattern()` - `192.10.1234/24` should NOT match
  - [x] `test_extract_dois_ignores_decimal_in_prose()` - `Section 10.1234/A describes...` SHOULD match (this IS a valid DOI pattern)
  - [x] `test_extract_dois_ignores_short_registrant_fraction()` - `10.12/something` should NOT match (registrant < 4 digits)

- [x] **Task 9: Write integration tests and check regressions** (AC: 5)
  - [x] **REGRESSION CHECK FIRST**: Review existing tests in `tests/parser_integration.rs` for any that pass `doi.org` URLs and assert `InputType::Url`. These will now return `InputType::Doi` and must be updated. Fix any broken assertions.
  - [x] In `tests/parser_integration.rs` (existing file):
    - [x] `test_parse_input_detects_doi_in_mixed_input()` - URLs + DOIs together, verify correct counts
    - [x] `test_parse_input_doi_url_classified_as_doi()` - `https://doi.org/10.1234/example` → DOI not URL
    - [x] `test_parse_input_non_doi_url_still_url()` - `https://example.com/paper.pdf` remains URL
    - [x] `test_parse_input_dois_iterator()` - verify `result.dois()` returns only DOI items
    - [x] `test_parse_input_duplicate_doi_both_returned()` - same DOI twice → two ParsedItems (no dedup)
    - [x] `test_parse_input_bibliography_with_dois()` - realistic bibliography with DOIs, URLs, and plain text

- [x] **Task 10: Run pre-commit checks** (AC: all)
  - [x] `cargo fmt --check`
  - [x] `cargo clippy -- -D warnings`
  - [x] `cargo test`
  - [x] All existing tests still pass (no regressions)

## Dev Notes

### Existing Code to Reuse - DO NOT Reinvent

**Parser module patterns (FOLLOW EXACTLY):**
- `src/parser/url.rs` - Pattern for `LazyLock<Regex>`, `extract_*()` function signature, `clean_url_trailing()` helper
  - **IMPORTANT:** `clean_url_trailing()` is currently private. Change to `pub(crate) fn clean_url_trailing()` and import in `doi.rs`. DO NOT duplicate this function.
- `src/parser/input.rs` - `ParsedItem::url()` constructor pattern → replicate as `ParsedItem::doi()`
- `src/parser/error.rs` - `ParseError` variant pattern with What/Why/Fix structure
- `src/parser/mod.rs` - Module registration and `parse_input()` integration point

**Key types already available:**
- `InputType::Doi` - Already defined in `input.rs` (line 11)
- `ParsedItem` - Use `ParsedItem::new(raw, InputType::Doi, normalized)`
- `ParseResult` - Already has `add_item()`, `add_skipped()`, `is_empty()`, `len()`
- `ParseError` - Add new `InvalidDoi` variant following existing pattern

**Dependencies already in Cargo.toml:**
- `regex = "1"` - For DOI pattern matching
- `url = "2"` - For URL parsing (doi.org URL detection)
- `urlencoding = "2"` - For URL-decoding DOIs
- `tracing` - For structured logging

**NO new Cargo.toml dependencies required for this story.**

### DOI Format Specification

DOI syntax per ISO 26324 / DOI Handbook:
- Prefix: `10.` followed by registrant code (4+ digits, can contain `.`)
- Separator: `/`
- Suffix: any printable Unicode characters (assigned by registrant)
- Case-insensitive but case-preserving
- Common examples:
  - `10.1234/example` (simple)
  - `10.1038/s41586-024-07386-0` (Nature)
  - `10.1016/j.cell.2024.01.001` (Elsevier/Cell)
  - `10.1371/journal.pone.0123456` (PLOS ONE)
  - `10.48550/arXiv.2301.00001` (arXiv)

### DOI Detection Priority vs URL Detection

**CRITICAL DESIGN DECISION:** When input contains `https://doi.org/10.1234/example`:
- The URL extractor will match it as a URL
- The DOI extractor will also detect it as a DOI
- **DOIs win**: classify as `InputType::Doi`, not `InputType::Url`

**Mandated approach:** Run both extractors on full input. Then **post-filter URL results**: remove any `Ok(ParsedItem)` where `url::Url::parse(&item.value).host_str()` is `"doi.org"` or `"dx.doi.org"`. This is simple, doesn't modify the URL extractor, and keeps both extractors independent.

### Processing Call Chain (within `extract_dois`)

For each regex match found by the three DOI patterns:
1. `normalize_doi(raw_match)` → strips URL/text prefixes, URL-decodes, trims → bare DOI string
2. `clean_url_trailing(normalized)` → strips trailing punctuation (reused from url.rs as `pub(crate)`)
3. `clean_doi_parens(cleaned)` → strips unmatched trailing `)` (DOI-specific)
4. `validate_doi(cleaned)` → checks registrant code length, suffix exists → returns `Result`
5. On success: `ParsedItem::doi(original_raw, validated_doi)` → into results vec
6. On failure: `Err(ParseError::InvalidDoi { ... })` → into results vec

### Parenthesis Handling Strategy

DOIs can legitimately contain parentheses in their suffix (e.g., `10.1002/(SICI)1097-4636`). But DOIs are often wrapped in parentheses in text (e.g., `(10.1234/example)`).

**Rule:** After extracting a DOI candidate, count `(` and `)` in the suffix. If the suffix ends with `)` and the count of `)` exceeds the count of `(`, strip trailing `)` characters until balanced. This handles both cases correctly.

### Architecture Compliance

**From architecture.md:**
- Parser module is pure parsing, no I/O
- `parser/` has no dependencies on other modules
- Module ownership: `src/parser/` → Input Parsing epic
- Error pattern: `thiserror` with What/Why/Fix structure
- Test pattern: inline `#[cfg(test)]`, naming `test_<unit>_<scenario>_<expected>`

**From project-context.md:**
- `#[tracing::instrument]` on public functions
- `#[must_use]` on functions returning `Result` or important values
- `LazyLock` for compiled regex (not `lazy_static`)
- Import order: std → external → internal
- Never `.unwrap()` in library code

### Project Structure Notes

**New file:**
- `src/parser/doi.rs` - DOI detection, validation, normalization

**Modified files:**
- `src/parser/mod.rs` - Add `mod doi;`, `pub use doi::extract_dois;`, integrate into `parse_input()`, add doi.org URL post-filter
- `src/parser/input.rs` - Add `ParsedItem::doi()` constructor, `ParseResult::dois()` iterator
- `src/parser/error.rs` - Add `InvalidDoi` variant with helpers
- `src/parser/url.rs` - Change `clean_url_trailing` from `fn` to `pub(crate) fn` (no other changes)
- `tests/parser_integration.rs` - Add DOI integration tests, fix any regressions from doi.org URL reclassification

**File structure after implementation:**
```
src/parser/
├── mod.rs          # parse_input() - now calls extract_dois + extract_urls
├── input.rs        # ParsedItem, ParseResult (add doi() and dois())
├── url.rs          # URL extraction (unchanged)
├── doi.rs          # NEW: DOI extraction, validation, normalization
└── error.rs        # ParseError (add InvalidDoi variant)
```

### Previous Story Intelligence

**From Epic 1 Story 1.7 (last completed story):**
- `LazyLock<Regex>` pattern works well (see url.rs line 15-19)
- `clean_url_trailing()` handles trailing punctuation - DOIs need similar handling
- Integration tests in `tests/parser_integration.rs` already exist
- Pre-commit check: `cargo fmt && cargo clippy -- -D warnings && cargo test`
- Note from 1.7: "Pre-existing parser test failure (Wikipedia parentheses) unrelated" - check if this is still present

**Key learning from Epic 1:** Tests should return `Result<(), Box<dyn Error>>` for cleaner error handling, use `assert!(matches!(...))` for enum variants.

### Git Intelligence

Recent commits show a single-crate structure with lib/bin split. All source in `src/`, tests in `tests/`. Only 2 commits so far - this is early in the project.

### Testing Strategy

**Unit tests in `src/parser/doi.rs`** (inline `#[cfg(test)]`):
- Test each DOI format variation (bare, URL, prefixed)
- Test normalization independently
- Test validation error cases
- Test edge cases (trailing punctuation, parentheses, empty input)

**Integration tests in `tests/parser_integration.rs`** (existing file):
- Test `parse_input()` with mixed URL + DOI input
- Test DOI URL classification (doi.org → DOI not URL)
- Test realistic bibliography text

### Anti-Patterns to Avoid

| Anti-Pattern | Correct Approach |
|---|---|
| Creating a new error module for DOI | Add `InvalidDoi` variant to existing `ParseError` |
| Using `lazy_static!` macro | Use `std::sync::LazyLock` (Rust 2024 edition) |
| `.unwrap()` in library code | Return `Result`, use `?` |
| Separate DOI struct instead of `ParsedItem` | Reuse `ParsedItem` with `InputType::Doi` |
| Logging with `println!` | Use `tracing::debug!`, `tracing::info!` |
| Hardcoding DOI patterns as strings | Use `LazyLock<Regex>` for compile-once |
| Testing DOI regex in isolation only | Also test integration through `parse_input()` |
| Letting doi.org URLs be classified as URL | Post-filter URL results to remove doi.org/dx.doi.org hosts |
| Duplicating `clean_url_trailing()` | Change to `pub(crate)` in url.rs and import in doi.rs |
| Deduplicating DOIs at parser level | Let the queue handle dedup - parser returns all matches |
| Only testing happy paths | Must include false-positive prevention tests (scores, versions, IPs) |
| Ignoring parenthesis edge case | Implement balanced-paren check for DOI suffix boundaries |
| Regex `10\.\d{4,9}/` only | Use `10\.\d{4,9}(?:\.\d+)*/` for nested registrants |

### References

- [Source: architecture.md#Project-Structure-&-Boundaries] - Parser module structure
- [Source: architecture.md#Implementation-Patterns-&-Consistency-Rules] - Naming, imports, errors
- [Source: architecture.md#Module-Ownership-Mapping] - Parser owns input parsing
- [Source: architecture.md#Resolver-Architecture] - Future: DOIs feed into resolver pipeline (Story 2.2+)
- [Source: project-context.md#Rust-Language-Rules] - Error handling, async, naming
- [Source: project-context.md#Testing-Rules] - Test organization, naming, coverage targets
- [Source: epics.md#Story-2.1] - Original acceptance criteria
- [Source: prd.md#FR-1.2] - "Resolve DOIs to downloadable URLs" (resolution is Story 2.3, detection is this story)
- [Source: 1-7-per-domain-rate-limiting.md] - Previous story patterns and learnings

---

## Dev Agent Record

### Agent Model Used

Claude Haiku 4.5 (claude-haiku-4-5-20251001)

### Debug Log References

- Regex lookbehind not supported by `regex` crate - replaced with preceding byte check in code
- Pre-existing clippy issues in `client.rs` and `extract_md_links.rs` fixed as part of Task 10

### Completion Notes List

- All 10 tasks completed, all subtasks checked off
- 315 tests pass (0 failures, 0 regressions)
- `cargo fmt`, `cargo clippy -- -D warnings`, `cargo test` all pass
- DOI extraction supports bare DOIs, doi.org URLs, dx.doi.org URLs, DOI: prefixed
- IP-like patterns rejected via preceding character check (no lookbehind support in regex crate)
- Balanced parenthesis handling implemented for DOI suffix boundaries
- doi.org URLs correctly reclassified as DOI type (not URL) via post-filter
- No new dependencies added

### Code Review Fixes Applied

Review found 8 issues (1 HIGH, 5 MEDIUM, 2 LOW). All fixed:
1. HIGH: Version number false positive - extended preceding byte check to `is_ascii_alphanumeric()`, test now asserts empty
2. MEDIUM: Dead code in `clean_doi_parens` - removed unused open_count/close_count variables
3. MEDIUM: Outdated `InputType::Doi` doc comment - removed "future - Epic 2"
4. MEDIUM: Outdated `parse_input()` doc comment - updated to reflect DOI support
5. MEDIUM: Missing `InvalidDoi` error message tests - added 2 tests
6. MEDIUM: Missing `\]` in DOI regex character classes - added for consistency with URL regex
7. LOW: Missing `ParsedItem::doi()` and `ParseResult::dois()` unit tests - added 3 tests

### File List

- `src/parser/doi.rs` - NEW: DOI extraction, validation, normalization, 28 unit tests
- `src/parser/mod.rs` - MODIFIED: Added DOI module, integrated into parse_input(), doi.org post-filter
- `src/parser/input.rs` - MODIFIED: Added ParsedItem::doi() constructor, ParseResult::dois() iterator
- `src/parser/error.rs` - MODIFIED: Added InvalidDoi variant with helpers
- `src/parser/url.rs` - MODIFIED: Changed clean_url_trailing to pub(crate)
- `src/download/client.rs` - MODIFIED: Fixed pre-existing clippy lint (map_unwrap_or)
- `src/bin/extract_md_links.rs` - MODIFIED: Fixed pre-existing clippy lint (if_same_then_else)
- `tests/parser_integration.rs` - MODIFIED: Updated existing tests for DOI reclassification, added 6 DOI integration tests
