# Downloader Desktop App — Manual Smoke Test Checklist

Run this checklist before each release to catch regressions that automated Vitest tests
do not cover (window lifecycle, native file pickers, OS clipboard, system font rendering).

Vitest covers: component rendering, input validation, progress event handling, error display,
completion summary, URL/DOI parsing utilities. See `src/lib/*.test.ts` for scope.

This checklist covers: the full Tauri IPC round-trip, native OS integration, and
multi-window / multi-session edge cases.

---

## Prerequisites

- `cargo tauri dev` running (or a release build)
- A test output directory created: `~/Downloads/downloader-smoke-test/`
- Network access available

---

## 1. Project Management

- [ ] App launches without error dialogs
- [ ] Project input field is empty by default
- [ ] Typing a project name highlights the field (blue border + light background)
- [ ] Clearing the project field removes the active styling
- [ ] If prior projects exist, the datalist dropdown shows them when you click or type
- [ ] Keyboard navigation (arrow keys + Enter) in the project datalist selects a suggestion
      **Note:** This works natively in Chrome-based WebView (Tauri) and Safari without
      custom event handlers — the `<input list="...">` + `<datalist>` pattern delegates
      keyboard handling to the browser engine.

---

## 2. URL Input

- [ ] Paste a single `https://` URL → no validation error
- [ ] Paste a `10.xxxx/yyyy` DOI → no validation error
- [ ] Paste a `https://www.youtube.com/watch?v=ID` YouTube URL → no error
- [ ] Paste a `https://www.youtube.com/shorts/ID` YouTube Shorts URL → no error
- [ ] Paste an empty string + click Download → shows "Please enter at least one URL or DOI"
- [ ] Paste only whitespace → same empty-input error
- [ ] Paste something that cannot be parsed (e.g. `hello world`) → shows parse error

---

## 3. Download Flow

- [ ] Paste a valid URL, click Download → button disables and shows spinner
- [ ] Cancel button appears while downloading
- [ ] Clicking Cancel stops in-flight downloads (progress bars freeze then disappear)
- [ ] After cancellation, the form resets and Download button re-enables
- [ ] Download completes: progress bar reaches 100%, then completion summary appears
- [ ] File appears at the expected output path

---

## 4. Progress Display

- [ ] Each download shows a separate progress bar with a URL label
- [ ] Progress bars update in real time (not just at 0% and 100%)
- [ ] Long URLs are truncated gracefully in the progress bar label
- [ ] Batch of 3+ URLs shows 3 concurrent progress bars

---

## 5. Error Handling

- [ ] Invalid DOI (e.g. `10.9999/fake`) → shows "failed" in completion summary
- [ ] Failed item entry shows: input, error description
- [ ] What/Why/Fix pattern visible in error text (description + likely cause + suggestion)
- [ ] **Expand/Collapse toggle** (>5 failures): paste 6+ invalid URLs, download, verify
  - "Show failed items (N)" button appears
  - "Expand all" / "Collapse all" button appears when N > 5
  - Clicking "Expand all" shows all failure details at once
  - Clicking "Collapse all" hides all details

---

## 6. Completion Summary

- [ ] 0 failures: green border, "Downloaded N file(s) to <path>"
- [ ] Mixed success/failure: amber border, "Completed: X downloaded, Y failed"
- [ ] All failed: red border, "Completed: 0 downloaded, N failed"
- [ ] Cancelled: grey border, "Cancelled — X completed, Y failed"
- [ ] "Download more" button resets to the empty form

---

## 7. Settings / Configuration

- [ ] Output directory setting is respected — files appear under the configured path
- [ ] Concurrency setting changes take effect (test with 1 vs default)
- [ ] Config survives app restart

---

## Result

| Date | Tester | Build | Platform | Pass/Fail | Notes |
|------|--------|-------|----------|-----------|-------|
| | | | | | |
