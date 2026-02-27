# Epic 10: Tauri Desktop App

**User Value:** "I can use a native desktop GUI to download papers"

**Decision source:** `_bmad-output/planning-artifacts/ui-scope-decision.md` (ratified by fierce ✅ 2026-02-25)

## Scope

Convert Downloader from a single-crate CLI binary to a Cargo workspace that ships both the existing CLI (`downloader-cli`) and a new Tauri 2.x native desktop application (`downloader-app`), powered by the extracted `downloader-core` library crate.

- Extract `src/` (minus `main.rs` + `cli.rs`) into a `downloader-core/` workspace crate
- Move CLI binary into `downloader-cli/` workspace crate
- Scaffold `downloader-app/` with Tauri 2.x (`cargo create-tauri-app`)
- Wire `downloader-core` as the Tauri backend dependency
- Implement minimal URL/DOI input → download trigger UI
- Show real-time progress and completion summary in the GUI

**Exit Criteria:** User can open the Downloader desktop app, paste a URL or DOI, click download, and watch real-time progress — equivalent to the CLI's Epic 1–3 flow.

**Dependencies:** All 8 CLI epics (complete). Tauri 2.x current stable: 2.10.2.

**Pre-conditions:**
- Apple Developer account for macOS code signing (distribution only — dev builds work without it)
- Frontend framework chosen in Story 10-1 (Svelte recommended; Vue 3 or React also compatible)
- `DOWNLOADER_REQUIRE_SOCKET_TESTS=1` confirmed in dev workflow

---

## Architecture Constraints

All constraints from existing epics carry forward:

| Constraint | Rule |
|------------|------|
| Local-first | No cloud sync, no remote server dependency |
| `#[deny(clippy::expect_used)]` | Enforced in `downloader-core`; new GUI crate follows same rule |
| Error boundary | `thiserror` in `downloader-core`, `anyhow` in binary crates (`downloader-cli`, `downloader-app/src-tauri`) |
| `cargo fmt && cargo clippy -- -D warnings` | Before every commit, all workspace crates |
| `DOWNLOADER_REQUIRE_SOCKET_TESTS=1` | Standard in all CI jobs that include network tests |

### Workspace Layout (Target)

```
downloader/                        (workspace root, current repo)
├── Cargo.toml                     (workspace manifest)
├── downloader-core/               (extracted library — current src/ minus main.rs + cli.rs)
│   ├── Cargo.toml
│   └── src/                       (auth, db, download, parser, queue, resolver, sidecar, topics)
├── downloader-cli/                (current CLI binary)
│   ├── Cargo.toml                 (depends on downloader-core)
│   └── src/main.rs + cli.rs
└── downloader-app/                (new Tauri desktop app)
    ├── Cargo.toml
    ├── src-tauri/
    │   ├── Cargo.toml             (depends on downloader-core)
    │   └── src/                   (Tauri commands, app state)
    └── src/                       (frontend: Svelte/Vue/React)
```

### Migration Steps (from ui-scope-decision.md)

1. Create workspace `Cargo.toml` wrapping current crate
2. Extract library to `downloader-core/` (move `src/` minus `main.rs`/`cli.rs`)
3. Move CLI to `downloader-cli/` (depends on `downloader-core`)
4. Run `cargo create-tauri-app downloader-app` in workspace root
5. Add `downloader-core = { path = "../downloader-core" }` to `downloader-app/src-tauri/Cargo.toml`
6. Confirm `cargo build --workspace` and `cargo test --workspace` pass

---

## Testing Strategy (defined up front — retro requirement)

| Layer | Framework | Scope |
|-------|-----------|-------|
| Rust backend logic | `cargo test` (existing, 566 tests) | All business logic — unchanged |
| Tauri commands (IPC) | `#[tauri::test]` mock runtime | Command handlers, app state |
| Frontend unit/integration | Vitest + `@tauri-apps/api/mocks` | UI components, state, event handling |
| E2E Linux/Windows CI | `tauri-driver` + WebdriverIO | Critical user flows |
| E2E macOS | Manual smoke test | Until `tauri-driver` macOS support lands (accepted gap) |

---

## Stories

### Story 10-1: Workspace Extraction + Tauri Project Init

**As a developer,**
I want the codebase restructured as a Cargo workspace with `downloader-core`, `downloader-cli`, and `downloader-app` crates,
**so that** the Tauri desktop app can import the core library without duplicating logic.

**Scope:**
- Create workspace `Cargo.toml`
- Extract library to `downloader-core/` (all `src/` modules except `main.rs` and `cli.rs`)
- Move CLI to `downloader-cli/` depending on `downloader-core`
- Scaffold `downloader-app/` with `cargo create-tauri-app` (Tauri 2.x)
- Choose and configure frontend framework (Svelte recommended)
- Confirm `cargo build --workspace`, `cargo test --workspace`, and `cargo clippy --workspace -- -D warnings` all pass
- No UI functionality yet — scaffold only

**Exit Criteria:**
- `cargo build --workspace` exits 0
- `cargo test --workspace` passes all 566+ existing tests
- `cargo clippy --workspace -- -D warnings` exits 0
- Tauri dev window opens (empty/placeholder UI is fine)
- No regression in CLI behavior (`downloader --help` works from `downloader-cli`)

**Source Hints:**
- Migration steps: `_bmad-output/planning-artifacts/ui-scope-decision.md` §Migration Steps
- Architecture constraints: this file §Architecture Constraints
- Current `Cargo.toml`: `[lib] name = "downloader_core"` already set (minimal extraction effort)
- Tauri 2.x docs: https://v2.tauri.app/

---

### Story 10-2: Basic Download Trigger UI

**As a user,**
I want a minimal desktop window where I can paste a URL or DOI and trigger a download,
**so that** I can start a download without using the terminal.

**Scope:**
- Minimal Tauri window: URL/DOI text input + download button
- Tauri command `start_download(inputs: Vec<String>, options: DownloadOptions)` wrapping `downloader_core::DownloadEngine`
- Basic status feedback (downloading / done / error) in the UI
- Respect existing `AppConfig` defaults (output dir, concurrency, etc.)
- Error display using the What/Why/Fix pattern (matching CLI Epic 7)
- Vitest unit tests for input component + Tauri IPC mock test for `start_download` command

**Exit Criteria:**
- User pastes a URL, clicks Download, file is saved to the configured output directory
- User pastes a DOI (e.g. `10.1000/xyz123`), clicks Download, DOI resolves and file downloads
- Invalid input shows a clear error message
- `cargo test --workspace` passes
- `tauri-driver` E2E test passes on Linux CI (or documented manual smoke on macOS)

**Source Hints:**
- Core integration: `downloader-core::DownloadEngine`, `downloader-core::AppConfig`
- Tauri command pattern: `ui-scope-decision.md` §Migration Steps / Step 4
- Error pattern: What/Why/Fix from Epic 7 (story `7-5-what-why-fix-error-pattern.md`)
- Testing layer: `#[tauri::test]` mock runtime for IPC, Vitest for frontend

---

### Story 10-3: Progress Display + Completion Summary

**As a user,**
I want real-time per-download progress bars and a completion summary in the desktop app,
**so that** I can see what's happening and know when my batch is done — matching the CLI's Epic 3 experience.

**Scope:**
- Real-time progress bar per download (mirrors `indicatif` CLI behavior)
- Tauri event emissions from `downloader-core` download callbacks → frontend listener
- Completion summary: total downloaded, failures, output path
- Failed items listed with actionable error info (What/Why/Fix)
- Cancel / interrupt support (Ctrl+C equivalent in GUI)
- Vitest tests for progress component; `#[tauri::test]` for event emission; E2E smoke on Linux CI

**Exit Criteria:**
- Batch of 3+ URLs shows individual progress bars updating in real time
- Completion summary appears when all downloads finish
- Failed downloads listed with error details
- Cancel button stops in-flight downloads gracefully (no panics, partial files cleaned up or preserved per existing retry logic)
- `cargo test --workspace` passes

**Source Hints:**
- Progress events: hook into `DownloadEngine` progress callbacks (see `src/download/mod.rs`)
- Completion summary design: Epic 3 story `3-4-completion-summary.md`
- Interrupt handling: Epic 3 story `3-5-graceful-interrupt-handling.md`
- Tauri events API: https://v2.tauri.app/develop/inter-process-communication/

---

## Epic 10 Pre-conditions Checklist

- [ ] Apple Developer account obtained (macOS distribution only; dev builds work without it)
- [ ] Frontend framework confirmed in Story 10-1 (Svelte recommended)
- [ ] `DOWNLOADER_REQUIRE_SOCKET_TESTS=1` confirmed as standard in local dev workflow

---

## Sources

- `_bmad-output/planning-artifacts/ui-scope-decision.md` — evaluation matrix, rationale, ratification
- `_bmad-output/planning-artifacts/architecture.md` — §Migration Path to Tauri (v2), §Library Boundary
- `_bmad-output/implementation-artifacts/project-retro-2026-02-23.md` — §Takeaway 6, §Next Sprint Priorities
- Tauri 2.x: https://v2.tauri.app/
