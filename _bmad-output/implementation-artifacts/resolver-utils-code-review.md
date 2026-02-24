# Code review: Resolver utils extraction

**Scope:** `src/resolver/utils.rs` and its use in `arxiv`, `pubmed`, `ieee`, `springer`, `sciencedirect`.

**Review date:** 2026-02-19

---

## Summary

The refactor cleanly moves shared host/DOI helpers and the `CITATION_PDF_RE` regex into `resolver::utils`, and all resolvers now use these consistently. The design is sound and behavior is preserved. A few small improvements (constants, docs, tests) would make the module more robust and easier to maintain.

---

## What works well

- **Single source of truth:** `canonical_host`, `parse_host_or_fallback`, `hosts_match`, `looks_like_doi`, and `CITATION_PDF_RE` live in one place. No duplicated logic.
- **Consistent API:** `looks_like_doi(value, prefix)` is a clear, reusable helper; IEEE, Springer, and ScienceDirect use it with their DOI prefix constants.
- **Sensible module boundary:** `utils` is `mod utils` (private). Only the resolver submodules use it; the rest of the crate does not depend on it.
- **Documentation:** Module and function docs describe purpose and behavior. `#[must_use]` is used where appropriate.
- **ScienceDirect alignment:** ScienceDirect now uses the same `parse_host_or_fallback` as the others (including trailing-dot trim and lowercasing in the fallback path), so host handling is consistent.
- **Import style:** Resolvers import only what they need from `super::utils::{ ... }`; arxiv’s qualified `super::utils::compile_static_regex` is also fine.

---

## Suggestions

### 1. ScienceDirect: introduce a DOI prefix constant

ScienceDirect uses the literal `"10.1016/"` in four places. For consistency with IEEE and Springer (and to avoid typos), define a constant:

```rust
const SCIENCE_DIRECT_DOI_PREFIX: &str = "10.1016/";
```

Then use `looks_like_doi(..., SCIENCE_DIRECT_DOI_PREFIX)` everywhere instead of `"10.1016/"`.

### 2. Document `looks_like_doi` prefix contract

`looks_like_doi` lowercases `value` but not `prefix`. If `prefix` were mixed-case or missing the trailing slash, behavior could be surprising. Recommend adding to the doc comment:

- Prefix should be lowercase and include the trailing slash (e.g. `"10.1109/"`), since `value` is compared after trim and lowercasing.

Optional hardening: normalize `prefix` (e.g. `prefix.trim().to_ascii_lowercase()`) so callers can pass `"10.1109/"` or `"10.1109/"` and get the same result. Not strictly necessary while all call sites use lowercase constants.

### 3. Unit tests for `utils`

The helpers are pure and easy to test. Adding a `#[cfg(test)]` block in `utils.rs` would lock in behavior and protect against regressions:

- **canonical_host:** e.g. `"  www.Example.COM.  "` → `"example.com"`, `"doi.org"` unchanged, empty/whitespace.
- **parse_host_or_fallback:** valid URL → host string; bare host → same as `canonical_host`; invalid URL → fallback to `canonical_host`.
- **hosts_match:** `"https://www.IEEE.org"` vs `"ieee.org"` → true; different hosts → false.
- **looks_like_doi:** `"10.1109/foo"` with prefix `"10.1109/"` → true; `"10.1007/bar"` with `"10.1109/"` → false; leading/trailing spaces and case insensitivity.

No need to test `compile_static_regex` or `CITATION_PDF_RE` beyond what existing resolver tests already cover.

### 4. Optional: minor allocation in `hosts_match`

`hosts_match` does `canonical_host(lhs) == canonical_host(rhs)`, so it allocates two `String`s. For hostnames this is negligible. If you ever profile and see hot paths here, you could do:

```rust
pub fn hosts_match(lhs: &str, rhs: &str) -> bool {
    let a = canonical_host(lhs);
    let b = canonical_host(rhs);
    a == b
}
```

Same number of allocations; only a style/clarity consideration. Not a change to make unless you have a reason.

---

## Minor nits

- **utils.rs line 9:** `panic!` in `compile_static_regex` is appropriate for “invalid static regex” and is documented. No change needed.
- **arxiv.rs:** Uses `super::utils::canonical_host` and `super::utils::compile_static_regex`. You could switch to `use super::utils::{canonical_host, compile_static_regex}` for consistency with the other resolvers; either way is fine.
- **mod.rs:** `mod utils` is intentionally not re-exported (`pub use`). That’s correct; external crates should not depend on resolver internals.

---

## Verdict

**Approve with minor suggestions.** The refactor is correct, keeps behavior consistent, and improves maintainability. Implementing the ScienceDirect constant, clarifying the `looks_like_doi` contract (and optionally normalizing `prefix`), and adding unit tests in `utils` would make the change even stronger. The optional `hosts_match` tweak and arxiv import style are low priority.
