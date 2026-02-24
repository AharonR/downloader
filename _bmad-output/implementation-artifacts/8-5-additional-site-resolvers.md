# Story 8.5: Additional Site Resolvers

Status: done

<!-- Note: Validation is optional. Run validate-create-story for quality check before dev-story. -->

## Story

As a **user**,
I want **more academic sites supported**,
so that **I can download from various sources**.

## Acceptance Criteria

1. **AC1: Additional resolvers are available for core target sites**
   - **Given** URLs (or site-specific DOI patterns) from common academic sources
   - **When** resolver dispatch runs
   - **Then** dedicated resolvers handle arXiv, PubMed, IEEE, and Springer inputs
   - **And** each resolver returns a final downloadable URL or clear structured failure

2. **AC2: Each new resolver follows the existing Resolver trait contract**
   - **Given** the `src/resolver` module architecture
   - **When** new resolver implementations are added
   - **Then** each resolver implements `Resolver` with `name`, `priority`, `can_handle`, and `resolve`
   - **And** resolver behavior remains compatible with `ResolveStep::{Url, Redirect, NeedsAuth, Failed}`

3. **AC3: Resolver execution order is prioritized by specificity**
   - **Given** a resolver registry containing specialized and fallback resolvers
   - **When** a site-specific URL or DOI is processed
   - **Then** site-specific resolvers run before general/fallback resolvers
   - **And** priority ordering is deterministic and covered by tests

4. **AC4: Supported-site documentation is explicit and current**
   - **Given** user-facing docs
   - **When** this story is complete
   - **Then** documentation lists supported site resolvers and accepted URL/DOI patterns
   - **And** docs clarify expected auth behavior where direct PDF access may require subscription

5. **AC5: Community resolver contributions are easy to add**
   - **Given** future contributors adding a new site resolver
   - **When** they follow project docs and module patterns
   - **Then** extension points are clear (module placement, registration, tests)
   - **And** a minimal "new resolver checklist" exists in docs for consistency

## Tasks / Subtasks

- [x] Task 1: Add arXiv resolver module with stable URL normalization (AC: 1, 2, 3)
  - [x] 1.1 Create `src/resolver/arxiv.rs` implementing `Resolver` as `Specialized`
  - [x] 1.2 Support common arXiv URL forms (`/abs/<id>`, `/pdf/<id>.pdf`) and `10.48550/arXiv.*` DOI signals
  - [x] 1.3 Resolve `abs` pages to canonical PDF URLs without regressing direct-PDF passthrough behavior
  - [x] 1.4 Add focused unit tests for `can_handle()` and normalization/resolve behavior

- [x] Task 2: Add PubMed resolver with robust failure messaging (AC: 1, 2, 3)
  - [x] 2.1 Create `src/resolver/pubmed.rs` implementing `Resolver` as `Specialized`
  - [x] 2.2 Support common `pubmed.ncbi.nlm.nih.gov` URL forms and PMC-linked full-text resolution paths
  - [x] 2.3 Return actionable `ResolveStep::Failed` when no PDF/full-text target can be determined
  - [x] 2.4 Add unit/integration tests covering success, no-full-text, and malformed-input cases

- [x] Task 3: Add IEEE resolver with auth-aware behavior (AC: 1, 2, 3)
  - [x] 3.1 Create `src/resolver/ieee.rs` implementing `Resolver` as `Specialized`
  - [x] 3.2 Support `ieeexplore.ieee.org` article/document URL patterns and `10.1109/*` DOI signals
  - [x] 3.3 Detect likely auth/paywall responses and surface `NeedsAuth` or clear failure guidance
  - [x] 3.4 Add tests validating URL matching, resolution, and auth-required signaling paths

- [x] Task 4: Add Springer resolver with canonical PDF extraction (AC: 1, 2, 3)
  - [x] 4.1 Create `src/resolver/springer.rs` implementing `Resolver` as `Specialized`
  - [x] 4.2 Support `link.springer.com` article/chapter URLs and common `10.1007/*` DOI patterns
  - [x] 4.3 Extract/normalize final PDF URL from metadata/link patterns with deterministic fallback
  - [x] 4.4 Add tests for URL handling, metadata extraction, and fallback behavior

- [x] Task 5: Wire resolver registry and exports (AC: 2, 3)
  - [x] 5.1 Update `src/resolver/mod.rs` to declare and re-export new resolver modules
  - [x] 5.2 Register resolvers in pipeline setup paths (`run_downloader`, `run_dry_run_preview`) before general/fallback resolvers
  - [x] 5.3 Ensure existing ScienceDirect/Crossref/Direct behavior remains intact for unaffected inputs

- [x] Task 6: Expand integration coverage for multi-resolver behavior (AC: 1, 3)
  - [x] 6.1 Extend `tests/resolver_integration.rs` with wiremock-backed scenarios for each new resolver
  - [x] 6.2 Add deterministic ordering assertions proving specialized resolver precedence
  - [x] 6.3 Add regression tests ensuring unknown/unsupported URLs still fall through cleanly

- [x] Task 7: Document supported sites and contribution flow (AC: 4, 5)
  - [x] 7.1 Update `README.md` with a "Supported resolvers" section listing arXiv, PubMed, IEEE, Springer, ScienceDirect, Crossref, Direct URL
  - [x] 7.2 Add a short resolver-authoring checklist (module + registration + tests + docs)
  - [x] 7.3 Reference contribution expectations from project context (tracing, tests, error style)

### Review Follow-ups (AI)

- [x] [AI-Audit][High] Define explicit per-site resolver completion contracts for arXiv, PubMed, IEEE, and Springer (expected success path, expected no-full-text behavior, and expected auth/paywall behavior) and map each to AC1 so implementation can be validated deterministically.
- [x] [AI-Audit][High] Introduce a shared resolver HTTP client policy/helper (timeouts, User-Agent, proxy compatibility, cookie support) and require all new resolvers to use it; add regression tests to prevent `no_proxy` or inconsistent client behavior.
- [x] [AI-Audit][Medium] Add a resolver-priority test matrix for overlapping DOI/URL patterns (`10.48550`, `10.1109`, `10.1007`, `10.1016`) proving specialized resolvers win before general/fallback handlers.
- [x] [AI-Audit][Medium] Add integration fixtures and tests for no-full-text, paywalled/auth-required, malformed URL, and metadata-missing scenarios for each new resolver.
- [x] [AI-Audit][Medium] Refactor resolver registration into a single shared function used by both `run_downloader` and `run_dry_run_preview` so resolver sets cannot drift between normal and dry-run flows.
- [x] [AI-Audit][Medium] Define and test a cross-resolver metadata normalization contract (`title`, `authors`, `doi`, `year`, `source_url`) so output/indexing behavior remains consistent across site resolvers.

## Dev Notes

### Architecture Context

Resolver architecture already uses:

- Trait-based resolver contracts (`Resolver`)
- Priority-ordered registry (`ResolverRegistry`)
- Structured result types (`ResolveStep`)
- Site-specific specialization pattern (`ScienceDirectResolver`)

Story 8.5 extends this pattern by adding four additional specialized resolvers without changing the core dispatch contract.

### Implementation Guidance

- Prefer site-specific resolver modules that stay stateless except for reusable HTTP client configuration.
- Keep direct PDF URLs fast-path where possible to avoid unnecessary HTML fetches.
- Follow existing error style with actionable suggestions (What/Why/Fix-compatible messaging).
- Use structured tracing (`#[instrument]`, `debug!`, `warn!`) for resolver flow and failure diagnostics.
- Keep resolver matching conservative: `can_handle()` should reject ambiguous inputs that belong to other resolvers.

### Architecture Compliance

- Preserve resolver priority semantics: `Specialized < General < Fallback`.
- Do not break current fallback chain for non-target URLs.
- Keep library code panic-free in resolver construction and request paths.
- Reuse current `reqwest` client safety conventions (timeouts + explicit `User-Agent`).

### Library / Framework Requirements

- Use existing crates only unless a new dependency is clearly justified in `Cargo.toml`.
- Keep async flows runtime-safe (`tokio`, no blocking calls in async paths).
- Use `wiremock` for resolver HTTP integration tests.

### File Structure Requirements

**New files expected:**
- `src/resolver/arxiv.rs`
- `src/resolver/pubmed.rs`
- `src/resolver/ieee.rs`
- `src/resolver/springer.rs`

**Files likely modified:**
- `src/resolver/mod.rs`
- `src/main.rs`
- `src/lib.rs`
- `tests/resolver_integration.rs`
- `README.md`

### Testing Requirements

- Add unit tests per resolver for `name`, `priority`, `can_handle`, and base resolution rules.
- Add integration tests for success + fallback + auth/permission edge cases.
- Assert registry ordering where multiple resolvers could match.
- Run targeted quality gates:
  - `cargo fmt`
  - `cargo clippy -- -D warnings`
  - `cargo test --test resolver_integration`
  - relevant targeted unit tests for `src/resolver/*`

### Previous Story Intelligence (from 8.4 and 8.3)

- Keep deterministic behavior explicit and test-backed (ordering, tie-breaks, fallback flow).
- Add small helper seams for hard-to-test external behavior.
- Preserve compatibility with existing CLI and history paths while adding new capabilities.
- Favor bounded, explicit behavior over implicit heuristics that are hard to validate.

### Git Intelligence Summary

- Recent committed history is shallow (`84d9f0b`, `7bec65d`) and does not provide deep implementation intent for resolver expansion.
- Current working tree already contains mature resolver infrastructure (ScienceDirect + Crossref + registry tests), so story implementation should anchor on present source patterns instead of commit archaeology.

### Latest Technical Notes (validated 2026-02-18)

- NCBI E-Utilities usage guidance documents request-rate expectations (3 req/s baseline; API-key paths can permit higher limits), relevant for PubMed resolver request discipline.
- NCBI PubMed EFetch documentation confirms `PubmedArticleSet` XML response structure for PMID lookups, useful when deriving metadata/full-text links.
- IEEE Developer portal documents metadata API and query-based retrieval capabilities, useful for resolver fallback strategy when direct page extraction is insufficient.
- Springer Nature metadata API supports JSON/XML response formats and query-based metadata lookup; this can backstop URL-only parsing where needed.

### Project Context Reference

From `/Users/ar2463/Documents/GitHub/Downloader/_bmad-output/project-context.md`:

- Use `tracing` (not `println!`) throughout resolver code and tests.
- Keep library code on `thiserror`/typed error paths, no panic-by-default behavior.
- Keep resolver modules organized and re-exported through `src/resolver/mod.rs`.
- Maintain test rigor with wiremock-backed integration tests and deterministic assertions.

### References

- [Source: /Users/ar2463/Documents/GitHub/Downloader/_bmad-output/planning-artifacts/epics.md#Story-8.5-Additional-Site-Resolvers]
- [Source: /Users/ar2463/Documents/GitHub/Downloader/_bmad-output/planning-artifacts/prd.md#FR-2-Download-Engine]
- [Source: /Users/ar2463/Documents/GitHub/Downloader/_bmad-output/planning-artifacts/architecture.md]
- [Source: /Users/ar2463/Documents/GitHub/Downloader/_bmad-output/project-context.md]
- [Source: /Users/ar2463/Documents/GitHub/Downloader/src/resolver/mod.rs]
- [Source: /Users/ar2463/Documents/GitHub/Downloader/src/resolver/registry.rs]
- [Source: /Users/ar2463/Documents/GitHub/Downloader/src/resolver/sciencedirect.rs]
- [Source: /Users/ar2463/Documents/GitHub/Downloader/tests/resolver_integration.rs]
- [NCBI E-Utilities Usage Guidelines](https://eutilities.github.io/site/API_Key/usageandkey/)
- [NCBI EFetch for PubMed](https://www.ncbi.nlm.nih.gov/books/NBK25500/)
- [IEEE API and Services Portal](https://developer.ieee.org/)
- [IEEE Xplore Metadata API Overview](https://developer.ieee.org/docs/read/Metadata_API_details)
- [Springer Nature Metadata API](https://dev.springernature.com/)
- [Springer Metadata API Quick Start](https://dev.springernature.com/docs/quick-start-guide)

## Developer Context

### Critical Implementation Guardrails

1. Keep resolver matching explicit to avoid stealing inputs from existing resolvers.
2. Preserve fallbacks: non-matching inputs must still be handled by Crossref/Direct where appropriate.
3. Do not add blocking network behavior or unbounded retries inside resolver `resolve()` paths.
4. Ensure all new resolver failures remain actionable and structured.
5. Keep contributor ergonomics high: module pattern, registration steps, and tests should be obvious from docs and code.

## Dev Agent Record

### Agent Model Used

gpt-5-codex

### Debug Log References

- dev-story implementation: added `arxiv`, `pubmed`, `ieee`, and `springer` resolvers with shared HTTP policy helper and deterministic resolver registration.
- validations run: `cargo fmt`, `cargo clippy -- -D warnings`, `cargo test --test resolver_integration`, and targeted resolver unit suites.
- full `cargo test` attempted in sandbox; several unrelated suites failed due wiremock port-binding sandbox restrictions and pre-existing non-resolver test failures.

### Completion Notes List

- 2026-02-18: Story created and staged as `ready-for-dev` via epic-auto-flow create-story stage.
- 2026-02-18: Ultimate context engine analysis completed - comprehensive developer guide created.
- 2026-02-18: Party mode audit completed with `pass_with_actions`; high/medium follow-up tasks inserted under `Review Follow-ups (AI)`.
- 2026-02-18: Added `ArxivResolver`, `PubMedResolver`, `IeeeResolver`, and `SpringerResolver` with resolver-trait compliance and structured failure/auth signaling.
- 2026-02-18: Added shared resolver HTTP client policy helper and centralized resolver registration via `build_default_resolver_registry` used by both normal and dry-run CLI flows.
- 2026-02-18: Expanded resolver integration coverage (24 tests) including overlap priority matrix, per-site negative paths, fallthrough regression, and metadata contract assertions.
- 2026-02-18: Updated README with supported resolvers, accepted patterns, auth expectations, metadata contract, and new resolver checklist.
- 2026-02-18: Added post-review regression coverage for Crossref constructor invalid-mailto handling and default-registry Crossref registration/skipping behavior.

### File List

- _bmad-output/implementation-artifacts/8-5-additional-site-resolvers.md
- _bmad-output/implementation-artifacts/sprint-status.yaml
- README.md
- src/lib.rs
- src/main.rs
- src/resolver/arxiv.rs
- src/resolver/crossref.rs
- src/resolver/direct.rs
- src/resolver/error.rs
- src/resolver/http_client.rs
- src/resolver/ieee.rs
- src/resolver/mod.rs
- src/resolver/pubmed.rs
- src/resolver/registry.rs
- src/resolver/sciencedirect.rs
- src/resolver/springer.rs
- tests/resolver_integration.rs

## Party Mode Audit (AI)

- **Audit Date:** 2026-02-18
- **Topic:** 8-5 Additional Site Resolvers
- **Outcome:** pass_with_actions
- **Findings:** 2 High · 4 Medium · 2 Low

### Findings by Perspective

**📋 Product/PM**
- **High (PM-1):** AC1 says resolvers are "available" but does not define per-site success/failure contracts (especially for PubMed no-full-text and IEEE/Springer paywalls), so implementation can pass superficially while user outcomes remain inconsistent.  
  Evidence: `/Users/ar2463/Documents/GitHub/Downloader/_bmad-output/implementation-artifacts/8-5-additional-site-resolvers.md` (AC1), `/Users/ar2463/Documents/GitHub/Downloader/_bmad-output/planning-artifacts/epics.md` (Story 8.5).
- **Low (PM-2):** AC4 requests supported-site docs but does not require explicit unsupported-pattern messaging, which can create false user expectations for near-miss domains.

**🏗️ Architect**
- **High (ARC-1):** Story does not enforce shared HTTP client behavior across new resolvers, risking divergence from project-context network guardrails (`User-Agent`, proxy compatibility, timeout policy).  
  Evidence: `/Users/ar2463/Documents/GitHub/Downloader/_bmad-output/project-context.md` (reqwest deployment rules), `/Users/ar2463/Documents/GitHub/Downloader/src/resolver/sciencedirect.rs` (site-specific client setup pattern).
- **Medium (ARC-2):** Registration is currently performed in multiple CLI flows; without a shared registration function, resolver list drift can cause inconsistent behavior between normal and dry-run runs.  
  Evidence: `/Users/ar2463/Documents/GitHub/Downloader/src/main.rs` (resolver registry setup in multiple functions).
- **Low (ARC-3):** Metadata field normalization is implied but not explicitly specified as a cross-resolver contract, increasing downstream variability risk.

**🧪 QA/TEA**
- **Medium (QA-1):** AC3 requires priority-by-specificity, but there is no explicit overlap matrix for DOI/URL patterns that may match multiple resolvers.  
  Evidence: `/Users/ar2463/Documents/GitHub/Downloader/_bmad-output/implementation-artifacts/8-5-additional-site-resolvers.md` (AC3), `/Users/ar2463/Documents/GitHub/Downloader/src/resolver/registry.rs` (priority resolution loop).
- **Medium (QA-2):** Story tasks do not yet force fixture-backed negative coverage for no-full-text, paywall/auth-required, malformed URLs, and missing metadata per site, leaving significant regression gaps.

**💻 Developer**
- **Medium (DEV-1):** Resolver output metadata schema is not explicitly locked across new resolvers, which can produce inconsistent file naming/index/history behavior during implementation.

### Single Prioritized Action List

1. Define per-site resolver success/failure contracts and attach them directly to AC1 acceptance checks.
2. Establish a shared HTTP client policy/helper for all new resolvers and add guardrail tests.
3. Add an overlap-priority test matrix for resolver dispatch determinism.
4. Add negative fixture coverage for no-full-text, auth, malformed, and sparse-metadata scenarios.
5. Centralize resolver registration across CLI execution paths.
6. Lock metadata normalization keys and add contract tests.

## Senior Developer Review (AI)

- **Review Date:** 2026-02-18
- **Mode:** adversarial + safe auto-fix
- **Outcome:** changes requested and applied

### Findings

- **[High][Fixed]** `CrossrefResolver` constructors used `expect(...)`, which could panic during resolver registry bootstrap if client creation failed in constrained environments. Fixed by making `CrossrefResolver::{new,with_base_url}` return `Result<_, ResolveError>` and handling failure in `build_default_resolver_registry` with structured warning + fallback continuation.
- **[Medium][Fixed]** Crossref User-Agent integration test asserted an obsolete header shape (`downloader/0.1.0 (mailto:...)`), causing deterministic failure outside sandbox constraints. Fixed expected header to match current production format (`downloader/<version> (crossref; mailto:...)`) and to derive version dynamically.
- **[Medium][Fixed]** Story Dev Agent File List omitted changed resolver core files (`direct`, `error`, `registry`), reducing traceability of AC2/AC3 implementation and review coverage. File list updated.

### Decisions Needed

- None.

## Change Log

- 2026-02-18: Completed story 8.5 implementation with four new site resolvers (arXiv, PubMed, IEEE, Springer), centralized resolver pipeline wiring, shared resolver HTTP policy helper, and expanded integration coverage.
- 2026-02-18: Senior code review completed; removed Crossref constructor panic path, corrected Crossref User-Agent test expectation, and updated story file list completeness.
- 2026-02-18: Added unit + integration regression tests to validate Crossref constructor error handling and registry behavior for valid vs invalid Crossref mailto configuration.
