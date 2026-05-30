# Development Notes

Quick reference for maintainers (and AI agents) working on this repo.

## Quality gates (what CI enforces)

CI is `.github/workflows/phase-rollout-gates.yml`. Run these before pushing — they
are the exact gates that block a merge:

```bash
cargo fmt --all --check                     # formatting
cargo test                                  # Rust tests (workspace)
cargo clippy --workspace -- -D warnings     # lints — see trap below
npm run test --prefix downloader-app        # frontend (vitest run; no watch hang)
```

### ⚠️ Clippy trap: do NOT add `--all-targets`

The CI clippy gate is `cargo clippy --workspace -- -D warnings` — **`--all-targets`
is intentionally omitted.** The crates enable restriction lints (`expect_used`,
`unwrap_used`) that are fine in libs/bins but fire all over **test code**. Running
`cargo clippy --workspace --all-targets -- -D warnings` produces 100+ errors
(e.g. ~163 in `downloader-core` lib tests) that CI never gates. A perfectly green
branch will look broken if you add `--all-targets`. Use the command above verbatim.

Related: `downloader-core/src/lib.rs` carries `#[deny(clippy::expect_used)]` for
lib code — use `ok_or` / `let Some(...) else { ... }` instead of `.expect()` there.

## Disk hygiene

The Rust `target/` dir grows large for this workspace (heavy deps: Tauri, tokio,
rustls; plus debug + release + llvm-cov profiles and per-test binaries). It has
reached **75 GB+**, and multiple git worktrees each carry their own `target/`.

- Reclaim space: `cargo clean` (full) or delete `target/llvm-cov-target` and
  `target/release` for a lighter ~few-GB recovery while keeping the debug cache.
- If you use `git worktree`, remember each worktree builds its own `target/`.
  Tear down stale ones: `git worktree remove <dir>` + `git branch -D <branch>`.

## Conventions

- Always `cargo fmt` before committing — CI fails on format drift.
- See `_bmad-output/project-context.md` for the full project coding rules.
- `CODEBASE_MAP.md` is generated routing only (gitignored) — verify against source.
</content>
</invoke>
