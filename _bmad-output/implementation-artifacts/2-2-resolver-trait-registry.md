# Story 2.2: Resolver Trait & Registry

Status: done

## Story

As a **developer**,
I want **an extensible resolver system**,
So that **new resolvers can be added without modifying core code**.

## Acceptance Criteria

1. **AC1: Resolver Trait Definition**
   - **Given** the resolver module
   - **When** a new resolver is implemented
   - **Then** it implements the async `Resolver` trait
   - **And** the trait has `name() -> &str` returning the resolver's name
   - **And** the trait has `priority() -> ResolverPriority` returning the resolver's priority level
   - **And** the trait has `can_handle(&self, input: &str, input_type: InputType) -> bool` for auto-detection
   - **And** the trait has `async fn resolve(&self, input: &str, ctx: &ResolveContext) -> Result<ResolveStep, ResolveError>` for resolution
   - **And** the trait is object-safe (`Send + Sync`) for use with `Box<dyn Resolver>`
   - **And** `async_trait` crate is used for async method support in trait objects

2. **AC2: Priority-Ordered Registry**
   - **Given** multiple resolvers registered
   - **When** the registry is queried for a handler
   - **Then** resolvers are tried in priority order: Specialized > General > Fallback
   - **And** within the same priority level, resolvers are tried in registration order
   - **And** the first resolver where `can_handle()` returns `true` is selected

3. **AC3: Resolution with Fallback Chain**
   - **Given** a resolver returns `ResolveStep::Failed`
   - **When** the registry's resolution loop processes the failure
   - **Then** the next applicable resolver at the same or lower priority is tried
   - **And** all applicable resolvers are exhausted before giving up
   - **And** an `AllResolversFailed` error is returned only when no resolver succeeds

4. **AC4: Redirect Chaining**
   - **Given** a resolver returns `ResolveStep::Redirect(new_url)`
   - **When** the resolution loop processes the redirect
   - **Then** the registry finds a new handler for the redirect URL
   - **And** resolution continues with the new URL
   - **And** a maximum redirect limit (10) prevents infinite loops
   - **And** `TooManyRedirects` error is returned when the limit is exceeded

5. **AC5: ResolveStep Enum**
   - **Given** a resolver processes an input
   - **When** it returns a result
   - **Then** `ResolveStep::Url(ResolvedUrl)` indicates a final downloadable URL
   - **And** `ResolveStep::Redirect(String)` indicates an intermediate URL needing further resolution
   - **And** `ResolveStep::NeedsAuth(AuthRequirement)` signals authentication is required
   - **And** `ResolveStep::Failed(ResolveError)` indicates this resolver cannot handle the input

6. **AC6: Direct URL Resolver (Reference Implementation)**
   - **Given** a plain URL (InputType::Url)
   - **When** the DirectResolver processes it
   - **Then** it returns `ResolveStep::Url` with the URL passed through unchanged
   - **And** the DirectResolver has `Fallback` priority (lowest)
   - **And** it serves as a reference implementation proving the framework works

7. **AC7: Module Integration**
   - **Given** the resolver module is created
   - **When** it is registered in `lib.rs`
   - **Then** `Resolver`, `ResolverRegistry`, `ResolveStep`, and `ResolverPriority` are accessible from `downloader_core::resolver`
   - **And** the module follows all project conventions (imports, naming, errors, logging)

## Tasks / Subtasks

**Dependency chain:** Tasks 1-2 are independent. Task 3 depends on Task 2. Task 4 depends on Task 3. Task 5 depends on Tasks 3-4. Task 6 depends on Tasks 3-5. Tasks 7-8 depend on Tasks 3-6. Task 9 is final verification.

- [x] **Task 1: Add `async-trait` dependency to Cargo.toml** (AC: 1)
  - [x] Add `async-trait = "0.1"` with justification comment: `# Async trait objects - required for dyn Resolver dispatch (Rust 2024 native async traits are not object-safe)`
  - [x] Verify `cargo check` still passes

- [x] **Task 2: Create resolver error types** (AC: 5)
  - [x] Create `src/resolver/error.rs`
  - [x] Define `ResolveError` enum with `thiserror` following the What/Why/Fix pattern
  - [x] Add helper constructors: `ResolveError::no_resolver(input)`, `ResolveError::too_many_redirects(input, count)`, `ResolveError::auth_required(domain, message)`, `ResolveError::resolution_failed(input, reason)`, `ResolveError::all_failed(input, tried_count)`

- [x] **Task 3: Define core types and Resolver trait in `src/resolver/mod.rs`** (AC: 1, 2, 5)
  - [x] Create `src/resolver/mod.rs` with module doc comment
  - [x] Add `mod` declarations: `mod direct;`, `mod error;`, `mod registry;`
  - [x] Add `pub use` re-exports for all public types
  - [x] Define `ResolverPriority` enum with derived `Ord`
  - [x] Define `ResolvedUrl` struct with `new()` and `with_metadata()` constructors
  - [x] Define `AuthRequirement` struct
  - [x] Define `ResolveStep` enum
  - [x] Define `ResolveContext` struct with `new()` and `Default` impl
  - [x] Define `Resolver` trait using `async_trait` with `InputType` import from parser

- [x] **Task 4: Implement ResolverRegistry** (AC: 2, 3, 4)
  - [x] Create `src/resolver/registry.rs`
  - [x] Define `ResolverRegistry` struct with `Vec<Box<dyn Resolver>>`
  - [x] Implement `new()`, `register()`, `find_handlers()`, `resolve_to_url()`
  - [x] Resolution loop handles: Url, Redirect, NeedsAuth, Failed, Err branches
  - [x] Add `#[tracing::instrument]` on `resolve_to_url` with field skips
  - [x] Add `resolver_count()` and `is_empty()` helpers
  - [x] Add `# Errors` doc section for clippy compliance

- [x] **Task 5: Implement DirectResolver** (AC: 6)
  - [x] Create `src/resolver/direct.rs`
  - [x] Implement `DirectResolver` unit struct with `new()` and `Default`
  - [x] Implement `Resolver` trait: name="direct", Fallback priority, URL-only
  - [x] Add `#[tracing::instrument]` on `resolve()`

- [x] **Task 6: Register module in lib.rs** (AC: 7)
  - [x] Add `pub mod resolver;` to `src/lib.rs`
  - [x] Add re-exports for all 8 public types
  - [x] Update module doc comment: moved resolver from "Future modules" to active modules

- [x] **Task 7: Write unit tests** (AC: 1-6)
  **In `src/resolver/error.rs`:** (6 tests)
  - [x] `test_resolve_error_no_resolver_message()`
  - [x] `test_resolve_error_too_many_redirects_message()`
  - [x] `test_resolve_error_all_failed_message()`
  - [x] `test_resolve_error_auth_required_message()` (bonus)
  - [x] `test_resolve_error_resolution_failed_message()` (bonus)
  - [x] `test_resolve_error_clone()`

  **In `src/resolver/mod.rs`:** (4 tests)
  - [x] `test_resolver_priority_ordering()`
  - [x] `test_resolved_url_new()`
  - [x] `test_resolved_url_with_metadata()`
  - [x] `test_resolve_context_default()`

  **In `src/resolver/direct.rs`:** (7 tests)
  - [x] `test_direct_resolver_name()`
  - [x] `test_direct_resolver_priority()`
  - [x] `test_direct_resolver_can_handle_url()`
  - [x] `test_direct_resolver_cannot_handle_doi()`
  - [x] `test_direct_resolver_cannot_handle_reference()`
  - [x] `test_direct_resolver_resolve_returns_url()`
  - [x] `test_direct_resolver_resolve_preserves_url()`

  **In `src/resolver/registry.rs`:** (12 tests)
  - [x] `test_registry_new_is_empty()`
  - [x] `test_registry_register_adds_resolver()`
  - [x] `test_registry_find_handlers_returns_matching()`
  - [x] `test_registry_find_handlers_priority_order()`
  - [x] `test_registry_find_handlers_empty_for_unknown()`
  - [x] `test_registry_resolve_to_url_direct()`
  - [x] `test_registry_resolve_to_url_no_resolver_error()`
  - [x] `test_registry_resolve_to_url_fallback_chain()`
  - [x] `test_registry_resolve_to_url_redirect()`
  - [x] `test_registry_resolve_to_url_too_many_redirects()`
  - [x] `test_registry_resolve_to_url_needs_auth()`
  - [x] `test_registry_resolve_to_url_all_fail()`

  **Mock resolver for testing:**
  - [x] Created `MockResolver` with configurable name, priority, handles, and step
  - [x] Helper constructors: `mock_url_resolver()`, `mock_failing_resolver()`, `mock_redirect_resolver()`

- [x] **Task 8: Write integration tests** (AC: 7)
  - [x] Create `tests/resolver_integration.rs`
  - [x] `test_resolver_registry_with_direct_resolver()` - full flow
  - [x] `test_resolver_registry_rejects_doi_with_only_direct()` - error path
  - [x] `test_resolver_registry_multiple_resolvers_priority()` - priority ordering
  - [x] `test_resolver_direct_preserves_url_exactly()` - URL preservation

- [x] **Task 9: Run pre-commit checks** (AC: all)
  - [x] `cargo fmt --check` - clean
  - [x] `cargo clippy -- -D warnings` - clean (6 issues found and fixed)
  - [x] `cargo test` - 348 tests pass (230 lib + 31 main + 2 bin + 85 integration + 19 doctests, 1 ignored)
  - [x] All existing 315 tests still pass (no regressions), 33 new resolver tests added

## Dev Notes

### Existing Code to Reuse - DO NOT Reinvent

**Parser module types (import, don't duplicate):**
- `crate::parser::InputType` - Used in `can_handle()` to determine if a resolver handles this input type
- `crate::parser::ParsedItem` - Not directly used by resolver trait but will be used by future caller code

**Error pattern (FOLLOW EXACTLY):**
- `src/parser/error.rs` - Follow the same `thiserror` pattern with What/Why/Fix structure
- `src/download/error.rs` - Another example of the error pattern in this project
- **IMPORTANT:** Resolver errors are a new module-level error enum, NOT added to `ParseError`

**Module pattern (FOLLOW EXACTLY):**
- `src/parser/mod.rs` - Module structure with `mod` declarations, `pub use` re-exports
- `src/download/mod.rs` - Another module pattern example

**Dependencies already in Cargo.toml:**
- `tracing` - For structured logging (`#[tracing::instrument]`)
- `thiserror` - For error types
- `tokio` - For async runtime (tests use `#[tokio::test]`)

**NEW dependency required:**
- `async-trait = "0.1"` - For async method support in trait objects (`Box<dyn Resolver>`)
- **Justification:** Rust 2024 native async traits are NOT object-safe. Since the registry stores `Box<dyn Resolver>`, we need `async_trait` for the `resolve()` method. This is the standard approach used by most async Rust crates.

### Architecture Compliance

**From architecture.md - Resolver Architecture:**
```rust
// Architecture specifies this contract (adapted for async):
trait Resolver: Send + Sync {
    fn name(&self) -> &str;
    fn priority(&self) -> ResolverPriority;
    fn can_handle(&self, input: &str) -> bool;
    async fn resolve(&self, input: &str, ctx: &ResolveContext) -> Result<ResolveStep>;
}
```
**Note:** Architecture says `can_handle(&self, input: &str)` but we add `input_type: InputType` parameter for type-based dispatch. This is a refinement that makes the trait more ergonomic (resolvers can match by type without re-parsing the input string).

**From architecture.md - Resolver Priority:**
```
enum ResolverPriority { Specialized, General, Fallback }
```

**From architecture.md - ResolveStep:**
```rust
enum ResolveStep {
    Url(ResolvedUrl),           // Final URL, hand to download engine
    Redirect(String),           // Intermediate, continue resolving
    NeedsAuth(AuthRequirement), // Signal auth needed
    Failed(ResolveError),       // Can't resolve
}
```

**From architecture.md - Module Ownership:**
- `src/resolver/` depends on: `parser` (for InputType), `auth` (future)
- `src/resolver/` is depended on by: `download` (for resolution)
- Parser has NO dependency on resolver (correct direction)

**From architecture.md - Resolution Loop:**
The registry's `resolve_to_url()` method implements the resolution loop that handles redirects and fallbacks. This is ENGINE-level logic, not resolver-level. Individual resolvers just return `ResolveStep`, the registry orchestrates the loop.

**From project-context.md:**
- `#[tracing::instrument]` on public functions
- `#[must_use]` on functions returning `Result` or important values
- `Send + Sync` on trait for async usage
- Import order: std → external → internal
- Never `.unwrap()` in library code
- Module-specific error enums with `thiserror`
- Unit tests inline with `#[cfg(test)]`

### Key Design Decisions

**Why `async_trait` and not native async traits:**
Rust 2024 stabilized `async fn` in traits for static dispatch (generics). However, the registry pattern requires DYNAMIC dispatch (`Box<dyn Resolver>`). Native async traits produce `impl Future` return types which are not object-safe. `async_trait` desugars `async fn` to `-> Pin<Box<dyn Future + Send + '_>>` which IS object-safe. This is the standard approach used by tower, axum, and other major async Rust crates.

**Why `InputType` parameter on `can_handle()`:**
The architecture shows `can_handle(&self, input: &str)` with only a string parameter. We add `input_type: InputType` because:
1. Resolvers often dispatch by type (DirectResolver handles URLs, CrossrefResolver handles DOIs)
2. Avoids re-parsing the input string in every resolver's `can_handle()` method
3. The parser already classified the input - leverage that work

**Why `ResolveStep::Failed` carries `ResolveError`:**
This allows the resolution loop to collect failure reasons from each resolver and report them in the final `AllResolversFailed` error. Individual resolver failures are informational, not terminal.

**Why DirectResolver is included in this story:**
A framework without any implementation is untestable. The DirectResolver is the simplest possible resolver (URL passthrough) and serves as:
1. A reference implementation for future resolver authors
2. A fallback that ensures plain URLs always work
3. A test vehicle for the registry and resolution loop

### Project Structure Notes

**New files:**
```
src/resolver/
├── mod.rs          # Module declarations, re-exports, Resolver trait, core types
├── error.rs        # ResolveError enum with thiserror
├── registry.rs     # ResolverRegistry struct with resolution loop
└── direct.rs       # DirectResolver (URL passthrough)
```

**Modified files:**
- `Cargo.toml` - Add `async-trait = "0.1"` dependency
- `src/lib.rs` - Add `pub mod resolver;` and re-exports

**New test files:**
- `tests/resolver_integration.rs` - Integration tests for resolver framework

**File structure after implementation:**
```
src/
├── lib.rs              # Add: pub mod resolver; and re-exports
├── resolver/
│   ├── mod.rs          # Resolver trait, ResolveStep, ResolverPriority, types
│   ├── error.rs        # ResolveError enum
│   ├── registry.rs     # ResolverRegistry with resolution loop
│   └── direct.rs       # DirectResolver (URL passthrough)
├── parser/             # UNCHANGED (resolver imports InputType from here)
├── download/           # UNCHANGED
├── queue/              # UNCHANGED
└── db.rs               # UNCHANGED
```

### Previous Story Intelligence

**From Story 2.1 (DOI Detection & Validation):**
- `LazyLock<Regex>` pattern works well for compile-once patterns
- `thiserror` error enums with What/Why/Fix structure are the established pattern
- Module structure: `mod.rs` + separate files for types, errors, implementations
- Integration tests in `tests/` directory (e.g., `tests/parser_integration.rs`)
- Pre-commit check: `cargo fmt && cargo clippy -- -D warnings && cargo test`
- **Code review found**: doc comments saying "future - Epic 2" needed updating. Update resolver docs proactively.
- **Code review found**: All regex character classes should be consistent. Ensure consistency in new code.
- **Regex lookbehind issue**: `regex` crate doesn't support lookbehinds. Not relevant here but good to know.
- Dead code gets flagged by clippy. Don't leave unused variables/functions.
- Tests should use `assert!(results.is_empty())` not `assert!(results.len() <= 1)` for "should not match" scenarios.

**From Story 2.1 - Code patterns established:**
- `#[allow(clippy::expect_used)]` with comment for static patterns
- `pub(crate)` for cross-module internal APIs
- `pub type ResultAlias = Result<T, ModuleError>` for module-specific result types
- Tests organized by category with `// ==================== Section ====================` headers

### Git Intelligence

Recent commits show only 2 commits (project is early):
- `84d9f0b Prepare first version`
- `7bec65d Initial commit: Downloader CLI with queue, retry, rate limiting`

All source in `src/`, tests in `tests/`. Single-crate with lib/bin split. No `src/resolver/` exists yet - this is a greenfield module.

### Testing Strategy

**Unit tests (inline `#[cfg(test)]`):**
- Each file gets its own test module
- `error.rs` - Test error message formatting
- `mod.rs` - Test type constructors and ordering
- `direct.rs` - Test DirectResolver behavior
- `registry.rs` - Test registration, lookup, resolution loop, fallback chain

**Integration tests (`tests/resolver_integration.rs`):**
- Test full resolution flow through public API
- Test registry with DirectResolver
- Test error cases (no resolver, unsupported type)

**Mock resolver for tests:**
Create a `MockResolver` in test modules that returns configurable `ResolveStep` values. This avoids needing real HTTP calls for registry behavior tests.

```rust
#[cfg(test)]
struct MockResolver {
    name: &'static str,
    priority: ResolverPriority,
    handles: Vec<InputType>,
    step: ResolveStep,
}

#[cfg(test)]
#[async_trait]
impl Resolver for MockResolver {
    fn name(&self) -> &str { self.name }
    fn priority(&self) -> ResolverPriority { self.priority }
    fn can_handle(&self, _input: &str, input_type: InputType) -> bool {
        self.handles.contains(&input_type)
    }
    async fn resolve(&self, _input: &str, _ctx: &ResolveContext) -> Result<ResolveStep, ResolveError> {
        Ok(self.step.clone())
    }
}
```

### Anti-Patterns to Avoid

| Anti-Pattern | Correct Approach |
|---|---|
| Using native async traits with `Box<dyn Resolver>` | Use `async_trait` crate for object-safe async traits |
| Creating a separate crate for the resolver | Add as module in existing lib (`src/resolver/`) |
| Making `ResolveContext` mutable across async boundaries | Use `&ResolveContext` (immutable), interior mutability if needed later |
| Putting resolution loop in individual resolvers | Resolution loop lives in `ResolverRegistry::resolve_to_url()` |
| Using `HashMap<String, Box<dyn Resolver>>` for registry | Use `Vec<Box<dyn Resolver>>` sorted by priority |
| Hardcoding max redirects | Use `ResolveContext::max_redirects` (configurable) |
| Making DirectResolver handle DOIs | DirectResolver handles ONLY `InputType::Url` |
| Logging with `println!` | Use `tracing::debug!`, `tracing::info!` |
| Using `lazy_static!` | Use `std::sync::LazyLock` if needed (Rust 2024) |
| `.unwrap()` in library code | Return `Result`, use `?` |
| Testing only happy paths | Include fallback chain, redirect loop, and error case tests |
| Implementing Crossref resolver here | That's Story 2.3 - this story is framework only |
| Adding HTTP client to ResolveContext | Keep context minimal for now, expand in Story 2.3 |
| Skipping `#[must_use]` on public functions | Add `#[must_use]` on functions returning Result or important values |

### References

- [Source: architecture.md#Resolver-Architecture] - Resolver trait, ResolveStep, ResolverPriority, resolution loop
- [Source: architecture.md#Implementation-Patterns-&-Consistency-Rules] - Naming, imports, errors, async patterns
- [Source: architecture.md#Module-Ownership-Mapping] - Resolver depends on parser, depended on by download
- [Source: architecture.md#Project-Structure-&-Boundaries] - `src/resolver/` directory structure
- [Source: project-context.md#Rust-Language-Rules] - Error handling, async, naming
- [Source: project-context.md#Testing-Rules] - Test organization, naming, coverage targets
- [Source: project-context.md#Framework-Specific-Rules] - Tokio async, tracing patterns
- [Source: epics.md#Story-2.2] - Original acceptance criteria
- [Source: 2-1-doi-detection-validation.md] - Previous story patterns and learnings

---

## Dev Agent Record

### Agent Model Used

Claude Opus 4.6 (claude-opus-4-6)

### Debug Log References

- Clippy found 6 issues during Task 9: `unnecessary_literal_bound` on DirectResolver::name(), `redundant_closure_for_method_calls` on find_handlers(), `missing_errors_doc` on resolve_to_url(), `needless_continue` x2 in resolution loop, `doc_markdown` on PubMed. All fixed.

### Completion Notes List

- Implemented complete resolver framework: trait, registry, error types, and DirectResolver reference implementation
- 29 unit tests across 4 files + 4 integration tests = 33 new tests total
- All 7 ACs satisfied: Resolver trait (AC1), priority registry (AC2), fallback chain (AC3), redirect chaining (AC4), ResolveStep enum (AC5), DirectResolver (AC6), module integration (AC7)
- MockResolver pattern established for testing registry behavior without HTTP calls
- Architecture compliance verified: resolver depends on parser (InputType), not vice versa
- `async_trait` v0.1.89 added for object-safe async trait dispatch

### File List

**New files:**
- `src/resolver/mod.rs` - Resolver trait, core types (ResolverPriority, ResolvedUrl, AuthRequirement, ResolveStep, ResolveContext)
- `src/resolver/error.rs` - ResolveError enum with 5 variants and helper constructors
- `src/resolver/registry.rs` - ResolverRegistry with resolution loop, fallback chain, redirect handling
- `src/resolver/direct.rs` - DirectResolver (URL passthrough, Fallback priority)
- `tests/resolver_integration.rs` - 4 integration tests

**Modified files:**
- `Cargo.toml` - Added `async-trait = "0.1"` dependency
- `Cargo.lock` - Auto-updated with async-trait dependency tree
- `src/lib.rs` - Added `pub mod resolver;` and 8 re-exports, updated module doc comment

### Change Log

- 2026-02-12: Implemented Story 2.2 - Resolver Trait & Registry. Created extensible resolver framework with async Resolver trait, priority-ordered ResolverRegistry with resolution loop (redirect chaining, fallback chain), ResolveError types, and DirectResolver reference implementation. Added 33 tests (29 unit + 4 integration). All 348 tests pass.
- 2026-02-12: Code review fixes - Added `#[tracing::instrument]` to `register()` and `find_handlers()` (M2), added manual `Debug` impl for `ResolverRegistry` (M3), added `AuthRequirement::new()` constructor (L1), added handler count summary log in resolution loop (L2), documented `Cargo.lock` in File List (M1). Added 2 new tests. All 350 tests pass.
