---
date: 2026-03-10
status: open
sprint_origin: UI polish sprint (design partner readiness)
---

# UI Deferred Backlog

Items identified during the UI polish sprint planning that are explicitly deferred.
Each item has a trigger condition for when to revisit.

## Item 1: Bibliography File Import (.bib / .ris) in GUI
Trigger: 2 of first 3 design partners confirm "I would import a .bib or .ris file"
vs. pasting DOIs/URLs.
Spec: See UI polish sprint plan — Work Item 3 (full implementation detail).
Risk if deferred too long: librarian workflow requires this; without it they paste DOIs
manually which is worse than their current Zotero workflow.

## Item 2: ToS Acknowledgment Flow in Desktop App
Current state: CLI has first-run ToS prompt + `tos_acknowledged` persisted to
`~/.config/downloader/config.toml`. The desktop app backend skips ToS entirely.
Trigger: Before any public release or design partner handoff where users are not
personally known to the author.
Risk: Legally and ethically the desktop app should not let users batch-download
without acknowledgment of publisher ToS responsibility.

## Item 3: Config Path Reconciliation
Current state: CLI reads/writes `~/.config/downloader/config.toml` (XDG standard).
Desktop app reads/writes `~/.downloader/config.toml` (non-standard).
Impact: A user who configures `output_dir` or `tos_acknowledged` via the CLI will
have those settings silently ignored by the desktop app, and vice versa.
Trigger: Before recommending both CLI and GUI to the same design partner.
Fix: Align both to the XDG path, or add a migration shim.

## Item 4: Output Directory Picker
Current state: Output directory is read from config; no in-app way to change it.
Trigger: Design partner asks "where do my files go and can I change it?"
UX strategy ref: Phase C "advanced options affordance" in ui-ux-strategy-2026-03-09.md.

## Item 5: App Icon / Visual Identity
Current state: Default Tauri icon set (generic).
Trigger: Before any public-facing announcement, Show HN, or screenshot share.
Note: Low priority until the product story is validated; a distinctive icon on an
unvalidated product is wasted effort.

## Item 6: Dark Mode
Trigger: Design partner feedback or after bibliography import validated.
Current hardcoded values to update: background `#fafafa`, form card `white`,
primary `#396cd8`, text `#1a1a2e`, `#333`, `#555`.
