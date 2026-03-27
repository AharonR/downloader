# Publisher Authentication (Cookie Capture) — Desktop App

## Problem

Some publishers (e.g. Emerald, Elsevier, Wiley) require institutional login to download full-text PDFs. When the downloader encounters an HTTP 403, it shows an `[AUTH]` error suggesting `downloader auth capture`. The CLI flow works but requires command-line steps. The desktop app needed a GUI equivalent.

## What was built

A collapsible **"Publisher authentication"** panel inside the download form that lets users import browser cookies with a button click.

### Backend (Rust / Tauri IPC)

Four new Tauri commands in `downloader-app/src-tauri/src/commands.rs`:

| Command | Purpose |
|---------|---------|
| `import_cookies(input: String)` | Parse pasted Netscape or JSON cookie text, validate, and persist to encrypted storage |
| `import_cookies_from_file()` | Open an OS single-file picker for a `.txt`/`.json` file, read its contents, and delegate to `import_cookies` |
| `get_cookie_status()` | Check if persisted cookies exist, return domain count and domain list |
| `clear_cookies()` | Delete all persisted cookies |

All commands reuse the existing `downloader_core::auth` module:
- `parse_captured_cookies()` for format detection and validation
- `store_persisted_cookies()` for XChaCha20-Poly1305 encrypted storage with keychain-backed keys
- `load_persisted_cookies()` for status checks
- `clear_persisted_cookies()` for cleanup

Commands are registered in `downloader-app/src-tauri/src/lib.rs`.

### Frontend (Svelte 5)

New component: `downloader-app/src/lib/AuthPanel.svelte`

- **Collapsed by default** — shows "Publisher authentication" with a domain count badge if cookies are saved
- **Expanded view** includes:
  - Current status bar showing which domains have saved cookies, with a "Clear cookies" button
  - Step-by-step instructions for exporting cookies from a browser
  - A textarea for pasting cookie data + "Import from paste" button
  - An "Import cookies.txt file" button that opens the OS file picker
  - Success/error feedback messages
- Integrated into `DownloadForm.svelte` between the bibliography row and the Download button

### User flow

1. User hits an `[AUTH]` error for a publisher (e.g. emerald.com)
2. User expands the "Publisher authentication" panel
3. User logs into the publisher site in their browser
4. User exports cookies via browser extension (e.g. "Get cookies.txt LOCALLY")
5. User either pastes the cookie text or imports the file via the file picker
6. Cookies are validated, encrypted, and stored locally (`~/.config/downloader/cookies.enc`)
7. Subsequent downloads to that publisher automatically include the saved cookies

### Cookie loading during downloads

Both download commands (`start_download`, `start_download_with_progress`) and the
resolver registry automatically load persisted cookies at the start of each run via
`load_runtime_cookie_jar()`. Cookies saved from either the CLI (`downloader auth capture`)
or the desktop app's Auth Panel are used by both interfaces.

Cookies are loaded independently by the resolver (for DOI redirect auth) and the HTTP
download client. This means two keychain/decrypt cycles per download — acceptable given
the cost is milliseconds against seconds of network I/O.

### Design constraint: single file import

`import_cookies_from_file` uses a single-file picker (`pick_file`), not a multi-file
picker. This is intentional: cookie files in JSON format cannot be naively concatenated
(two JSON arrays joined with `\n` produce invalid JSON). Netscape format would survive
concatenation, but using a single-file picker avoids the format-dependent edge case entirely.

### Security

- Cookies encrypted at rest with XChaCha20-Poly1305
- Encryption key stored in system keychain (or `DOWNLOADER_MASTER_KEY` env var)
- File permissions 0o600 (owner-only) on Unix
- Domain-scoped cookie jar prevents cross-site leakage
- Cookie values never logged (redacted in debug output)

## Files changed

- `downloader-app/src-tauri/src/commands.rs` — 4 new commands, `build_http_client_with_cookies()` helper, cookie jar loading in `resolve_and_enqueue()`
- `downloader-app/src-tauri/src/lib.rs` — command registration
- `downloader-app/src/lib/AuthPanel.svelte` — new UI component
- `downloader-app/src/lib/AuthPanel.test.ts` — 22 component tests
- `downloader-app/src/lib/DownloadForm.svelte` — import + render AuthPanel
- `downloader-app/src/lib/DownloadForm.test.ts` — updated mock to handle `get_cookie_status` from AuthPanel mount
