---
name: Fix download module build errors
overview: Fix full crate build by replacing unstable Option::is_none_or in src/download/robots.rs with stable map_or(true, ...). This unblocks the download module so RobotsCache, RobotsDecision, and origin_for_robots resolve in task.rs; no change to task.rs.
todos:
  - id: fix-robots
    content: In src/download/robots.rs replace is_none_or with map_or(true, ...) and add brief comment
    status: pending
  - id: verify-build
    content: Run cargo build -p downloader and cargo test -p downloader
    status: pending
  - id: verify-resolver
    content: Run cargo test -p downloader --lib resolver
    status: pending
  - id: verify-robots-tests
    content: Run cargo test -p downloader --lib download::robots
    status: pending
  - id: clippy
    content: Run cargo clippy -p downloader --lib and fix any new lints
    status: pending
isProject: false
---

# Fix download-module build errors (RobotsCache, RobotsDecision, origin_for_robots)

## Context

Full `cargo build` / `cargo test` fails with unresolved names in the download module: `RobotsCache`, `RobotsDecision`, and `origin_for_robots` (e.g. in `src/download/engine/task.rs`). The resolver-dedup work does not touch those files. This plan fixes the underlying cause so the build and tests (including `cargo test -p downloader --lib resolver`) pass.

---

## Root cause

**Single crate.** The library is one crate, `downloader_core` ([src/lib.rs](src/lib.rs)); there is no separate `download` crate.

**Symbol flow.** [src/download/robots.rs](src/download/robots.rs) defines `RobotsCache`, `RobotsDecision`, and `origin_for_robots`. [src/download/mod.rs](src/download/mod.rs) re-exports them; [src/lib.rs](src/lib.rs) re-exports from `download`. [src/download/engine/task.rs](src/download/engine/task.rs) imports from the crate root: `use crate::{RobotsCache, RobotsDecision, origin_for_robots};` — which is correct.

**Failure chain.**

1. **robots.rs** uses `Option::is_none_or()` at line 61. `is_none_or` is **not stabilized** in Rust ([tracking issue](https://github.com/rust-lang/rust/issues/126383)); on stable toolchains this yields a compile error in `robots.rs`.
2. Because `robots.rs` fails, the `download` module never finishes compiling, so the re-exports in `download/mod.rs` and `lib.rs` are never available.
3. The compiler then reports **unresolved** `RobotsCache`, `RobotsDecision`, and `origin_for_robots` in **task.rs** — a cascade from step 1, not a bug in task.rs.

**Conclusion.** The only code change required is in **robots.rs**. **task.rs** needs no edits. This fix keeps the crate building on **stable Rust** without the unstable `is_none_or` feature.

---

## Fix in `src/download/robots.rs`

**Location:** Inside `RobotsCache::check_allowed`, around lines 58–63.

Replace the unstable `is_none_or` with the stable `map_or(true, ...)`. The behavior is identical: `is_none_or(f)` is `true` when the option is `None`, or when it is `Some(x)` and `f(x)` is true; `map_or(true, f)` does the same (default `true` for `None`, otherwise the result of the closure).

Add a one-line comment so future readers don’t revert to `is_none_or` when it stabilizes without checking.

**Current code:**

```rust
let need_fetch = self
    .cache
    .get(origin)
    .is_none_or(|c| {
        now.duration_since(c.fetched_at).unwrap_or(Duration::MAX) > ROBOTS_TTL
    });
```

**Replace with:**

```rust
// Stable replacement for Option::is_none_or (not yet stabilized as of 2025)
let need_fetch = self
    .cache
    .get(origin)
    .map_or(true, |c| {
        now.duration_since(c.fetched_at).unwrap_or(Duration::MAX) > ROBOTS_TTL
    });
```

**Semantics (unchanged):** *Need to fetch if there is no cache entry, or if the cached entry is older than `ROBOTS_TTL`.*

---

## Verification

Run all of the following from the **repository root**. Each command must succeed (exit 0).

1. **Build**
  - `cargo build -p downloader`
2. **Full library tests**
  - `cargo test -p downloader`
3. **Resolver tests** (as called out in the original issue)
  The word `resolver` here is a **test-name substring filter**: Cargo runs any test whose name contains `resolver`. It is not a module path.
  - `cargo test -p downloader --lib resolver`
4. **Robots module tests** (regression check for the edited file)
  This runs tests in the `download::robots` module (path-based filter).
  - `cargo test -p downloader --lib download::robots`
5. **Lints**
  - `cargo clippy -p downloader --lib`  
   Fix any new lints in the changed code.

---

## Success criteria

- `src/download/robots.rs` is the only file modified (one expression replaced + one comment added).
- `cargo build -p downloader` succeeds.
- `cargo test -p downloader` succeeds.
- `cargo test -p downloader --lib resolver` succeeds.
- `cargo test -p downloader --lib download::robots` succeeds.
- `cargo clippy -p downloader --lib` reports no new issues in the changed code.

---

## Summary


| File                                                       | Action                                                                                                                     |
| ---------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------- |
| [src/download/robots.rs](src/download/robots.rs)           | In `check_allowed`: replace `.is_none_or(|c| ...)` with `.map_or(true, |c| ...)` and add the comment above the assignment. |
| [src/download/engine/task.rs](src/download/engine/task.rs) | No change; imports are correct.                                                                                            |
| Other modules                                              | No change; resolver-dedup work does not touch these files.                                                                 |


When the above is done, the download module compiles, the re-exports are available, and the build and test commands in this plan pass.