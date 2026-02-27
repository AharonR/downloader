# UI Scope Decision — Downloader

**Date:** 2026-02-25
**Owners:** fierce (product direction), Architect (technical feasibility)
**Decision:** Tauri desktop app — proceed to Epic 10
**Recommended by:** dev agent (claude-sonnet-4-6)
**Ratified by:** fierce ✅ (2026-02-25)  Override (if any): none — decision stands as recommended

---

## Evaluation Matrix

### Hard Gate (PASS/FAIL — failure eliminates option)

| Gate | Tauri 2.x | Web (WASM) | Web (local server) | Deferred |
|------|-----------|------------|-------------------|---------|
| Local-first compliance | ✅ PASS | ❌ FAIL — browser sandbox blocks filesystem access; per-session user-permission dialogs break the tool's core download flow | ✅ PASS (marginal) — local only, no cloud, but requires managing a background server process | ✅ PASS |

> **Web (WASM) is eliminated.** WASM cannot access the local filesystem without the browser's File System Access API, which requires a user-permission dialog on every session and cannot write to arbitrary directories. This directly conflicts with the tool's core flow (download to a configured output path).
>
> **Web (local server)** technically passes the hard gate but proceeds to scoring with severe penalties — a background HTTP server is required, introducing port management, CORS attack surface, and two-process lifecycle complexity for a single-user local tool. It is assessed below for completeness.

---

### Scored Criteria (1–5, higher = better)

| Criterion | Wt | Tauri | Tauri×Wt | Web (server) | Web×Wt | Deferred | Def×Wt |
|-----------|-----|-------|----------|-------------|--------|----------|--------|
| UX quality | 4 | 4 | 16 | 2 | 8 | 2 | 8 |
| Testing strategy maturity | 4 | 3 | 12 | 3 | 12 | 5 | 20 |
| Migration effort | 3 | 3 | 9 | 2 | 6 | 5 | 15 |
| Time to first story | 3 | 3 | 9 | 2 | 6 | 5 | 15 |
| Maintenance overhead | 3 | 3 | 9 | 2 | 6 | 5 | 15 |
| Platform coverage | 2 | 3 | 6 | 4 | 8 | 3 | 6 |
| **Total** | **19** | — | **61** | — | **46** | — | **79** |

**Max possible score: 95**

---

### Score Notes

**Tauri 2.x (61/95):**
- *UX quality 4/5*: Native webview, any web framework, native OS integration, performance far exceeds CLI. Not a native-widget app but significantly better UX than CLI.
- *Testing maturity 3/5*: macOS `tauri-driver` WebDriver E2E not supported (only Linux + Windows). Primary platform (macOS) requires alternative: Rust backend fully covered by existing `cargo test`; Tauri frontend unit/integration testable with Vitest; E2E on macOS requires manual smoke testing or accessibility APIs until `tauri-driver` macOS support lands. This is a real constraint but not a blocker.
- *Migration effort 3/5*: Workspace extraction is well-understood Rust. The architecture documented the exact steps. However, Tauri requires `tauri-cli` (Node.js dependency), a frontend framework choice, and macOS code signing (~$99/yr Apple Developer account) for distribution.
- *Time to first story 3/5*: ~3 stories before anything useful ships: workspace setup, first Tauri window with basic download trigger, progress display.
- *Maintenance overhead 3/5*: `tauri-cli` Node.js toolchain, WebView OS version compatibility, platform-specific code signing, frontend dependency chain. Manageable but non-trivial.
- *Platform coverage 3/5*: macOS (primary, code signing required), Windows (NSIS/MSI, Authenticode signing optional), Linux (deb/rpm/AppImage, no signing required).

**Web local server (46/95):**
- *UX quality 2/5*: Browser UI for a local desktop tool is dissonant. Download progress requires polling or WebSocket. Users must open a browser tab manually.
- *Testing maturity 3/5*: Web testing (Playwright, Cypress) is mature, but the server integration layer adds complexity.
- *Migration effort 2/5*: Requires adding an HTTP server (axum), serializing all core types as JSON API, CORS policy, process management for server lifecycle, a frontend framework, and a build pipeline.
- *Time to first story 2/5*: Significant scaffolding before anything runs.
- *Maintenance overhead 2/5*: Two-process architecture (server + browser), port conflicts, localhost security concerns (any tab on the same machine could call the API if CORS is misconfigured).

**Deferred (79/95):**
- *UX quality 2/5*: No improvement — CLI-only. Acceptable but leaves value on the table.
- *Testing maturity 5/5*: Zero new testing infrastructure needed.
- *Migration/time/maintenance 5/5*: Zero cost. Stay as-is.
- *Platform coverage 3/5*: CLI works everywhere Rust runs; no GUI accessibility or discoverability benefits.

---

## Recommendation: Tauri — Proceed to Epic 10

### Why Tauri over Deferred, despite raw score

The scoring model is cost-weighted (4 cost criteria, 2 value criteria), which structurally favours deferral regardless of user intent. That bias is appropriate for a neutral evaluation, but it must be balanced against explicit product intent:

1. **Fierce has already stated intent**: "I do want future GUI, it's next step." The evaluation exists to confirm path and scope — not to re-open the product decision.
2. **The architecture was designed for this transition from day 1**: The lib/bin split, `[lib] name = "downloader_core"`, and the Tauri migration section in `architecture.md` were written specifically to keep this transition low-cost. Deferring indefinitely wastes that investment.
3. **v1 is complete and stable**: The retro explicitly named this moment as the right time. The concern at Epic 1 ("premature at MVP") is resolved.
4. **Deferred without a concrete trigger is a permanent carry-forward**: This story exists because the retro flagged it as a persistent open item. Choosing "deferred" without a trigger doesn't close the item — it restarts the cycle.

### Why Tauri over Web

Web (WASM) is eliminated by the hard gate. Web (local server) passes the gate but scores 15 points lower than Tauri on a purely cost basis — and offers worse UX with more operational complexity. Tauri was explicitly the designed-for path. Web is not.

### Tauri testing strategy (resolves AC#2 and retro requirement)

The testing strategy for a Tauri implementation:

| Layer | Framework | Scope | Status |
|-------|-----------|-------|--------|
| Rust backend logic | `cargo test` (existing) | All business logic, download engine, resolvers | ✅ Already comprehensive — 566 tests, 89.55% coverage |
| Tauri commands (IPC) | Rust unit tests + `#[tauri::test]` mock runtime | Command handlers, state management | New tests per story |
| Frontend unit/integration | Vitest + `@tauri-apps/api/mocks` | UI components, state, event handling | New tests per story |
| E2E (Linux/Windows CI) | `tauri-driver` + WebdriverIO | Critical user flows on Linux CI | Added to CI per story |
| E2E (macOS) | Manual smoke test + accessibility APIs | Until `tauri-driver` macOS support lands | Documented manual checklist per story |

**macOS E2E gap is accepted** as a known constraint. The Rust backend (which handles all business logic) is fully tested on all platforms via `cargo test`. The macOS WebDriver limitation only affects end-to-end UI automation — which is a quality-of-life CI feature, not a correctness gate.

---

## Migration Steps (from current single-crate to Tauri workspace)

Based on `architecture.md` §Migration Path to Tauri (v2), current Tauri 2.10.2:

### Step 1 — Extract library to separate crate
```
downloader/                    (current)
├── Cargo.toml                 ([lib] name = "downloader_core" already set)
├── src/lib.rs
└── src/main.rs

→ becomes:

downloader-workspace/
├── Cargo.toml                 (workspace manifest)
├── downloader-core/           (extracted library)
│   ├── Cargo.toml
│   └── src/                   (current src/ minus main.rs and cli.rs)
├── downloader-cli/            (current CLI binary)
│   ├── Cargo.toml             (depends on downloader-core)
│   └── src/main.rs + cli.rs
└── downloader-app/            (new Tauri app)
    ├── Cargo.toml             (depends on downloader-core)
    ├── src-tauri/
    └── src/                   (frontend)
```

### Step 2 — Tauri project init
```bash
cd downloader-workspace
cargo create-tauri-app downloader-app --template [framework TBD]
```

### Step 3 — Wire `downloader-core` as Tauri backend dependency
```toml
# downloader-app/src-tauri/Cargo.toml
[dependencies]
downloader-core = { path = "../../downloader-core" }
tauri = { version = "2", features = [] }
```

### Step 4 — Define Tauri commands wrapping core operations
```rust
#[tauri::command]
async fn start_download(urls: Vec<String>, options: QueueProcessingOptions) -> Result<(), String> {
    // delegates to downloader-core::DownloadEngine
}
```

### Frontend framework choice (deferred to Epic 10 story 1)
Options: Svelte (lightweight, good Rust interop), Vue 3, React. Decision to be made in Epic 10 story 1 based on team familiarity. All are compatible with Tauri 2.x.

---

## Epic 10 Outline — First 3 Story Candidates

| Story | Title | Description |
|-------|-------|-------------|
| 10-1 | Workspace extraction + Tauri project init | Extract to workspace, init `downloader-app` with Tauri 2.x, confirm `cargo build` and `cargo test` pass for all crates. No UI yet — just the scaffold. |
| 10-2 | Basic download trigger UI | Minimal window: URL/DOI input field, download button, basic status feedback. Wires to `downloader-core::DownloadEngine` via Tauri command. |
| 10-3 | Progress display + completion summary | Real-time progress bar per download, completion summary. Matches the CLI's Epic 3 UX in GUI form. |

Epic 10 pre-conditions:
- Apple Developer account obtained (macOS code signing for distribution)
- Frontend framework chosen (story 10-1)
- `DOWNLOADER_REQUIRE_SOCKET_TESTS=1` confirmed as standard in dev workflow (retro item, still open)

---

## Re-evaluation Criteria (N/A — proceeding)

Not applicable — decision is to proceed with Tauri. If this decision is overridden to "deferred," use these triggers:

| Trigger | Action |
|---------|--------|
| 2026-08-25 (6 months) | Mandatory re-evaluation regardless of other factors |
| `tauri-driver` macOS E2E support ships | Re-evaluate testing strategy maturity score — likely shifts Tauri to 4/5, strengthening the case further |
| A GitHub issue requesting GUI reaches 10 upvotes | Re-evaluate as demand signal |

---

## Sources

- [Tauri 2.0 Stable Release](https://v2.tauri.app/blog/tauri-20/)
- [Tauri Core Ecosystem Releases (current: 2.10.2)](https://v2.tauri.app/release/)
- [WebDriver Testing — Tauri](https://v2.tauri.app/develop/tests/webdriver/)
- [Tauri Tests Overview](https://v2.tauri.app/develop/tests/)
- [Rust + WASM file access limitations](https://users.rust-lang.org/t/use-local-files-used-from-the-browser-from-rusts-webassembly-wasm/90565)
- Project retrospective §Takeaway 6: `_bmad-output/implementation-artifacts/project-retro-2026-02-23.md`
- Architecture §Migration Path to Tauri: `_bmad-output/planning-artifacts/architecture.md`
