# Effort Log — 2026-04-21 — Registry + Dedup Hardening

## Scope

This pass implemented and validated the expert-audited fixes around:

- registry persistence signaling
- Windows atomic replacement safety
- registry coordination under concurrent same-project runs
- project-scoped dedup correctness
- sidecar corruption recovery robustness

## Implemented Decisions

### 1) Registry coordination (fail-fast lock)

- Added advisory lock file per project:
  `<output_dir>/.downloader/downloaded-registry.v1.lock`.
- Lock is acquired in `DownloadedRegistry::load(...)` and held for the lifetime
  of the registry object.
- Lock acquisition is fail-fast (`WouldBlock`) when already held.
- `DownloadedRegistry` is no longer `Clone` to keep lock ownership unambiguous.

### 2) Windows atomic replacement hardening

- Replaced Windows delete+rename behavior with a `ReplaceFileW`-first flow:
  1. try `ReplaceFileW(dst, tmp, ...)`
  2. if destination missing, fallback to `rename(tmp -> dst)`
  3. if fallback races into `AlreadyExists`, retry replacement
  4. bounded retry on transient sharing/access/lock errors
- No regular destination deletion step is used.

### 3) Registry save failure policy

- In app and CLI completion paths, `registry.save_if_dirty()` failures are now
  surfaced as warnings instead of aborting successful downloads.
- App summary now carries structured warning fields:
  `code`, `path`, `error`, `impact`, `fix`.
- Warning code for this case: `registry_persist_failed`.

### 4) Project-scoped dedup correctness

- Removed unscoped active URL short-circuit in app resolve/enqueue flow.
- Project-scoped active checks and skip-history recording are now authoritative.

### 5) Sidecar corruption recovery hardening

- If a sidecar exists but contains invalid JSON, it is quarantined and regenerated.
- Quarantine filename now includes high-entropy suffixes (`nanos`, `pid`, `seq`)
  and retries on `AlreadyExists` collisions.

## Tests Added

- App:
  - cross-project active URL does not block current-project enqueue
  - `duplicate_active` skip persists as `status=skipped` history row
- Core sidecar:
  - invalid sidecar is quarantined and regenerated
  - quarantine path generation is unique across calls

## Remaining Nice-to-Haves

- Add command-level `start_download_with_progress` project-isolation test.
- Add `cfg(windows)` retry-behavior test for transient replacement contention.
