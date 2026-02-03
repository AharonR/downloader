# Story 1.5: Concurrent Downloads

Status: done

## Story

As a **user**,
I want **multiple files to download simultaneously**,
So that **batch downloads complete faster**.

## Acceptance Criteria

1. **AC1: Concurrent Download Execution**
   - **Given** items stored in SQLite queue (from Story 1.4)
   - **When** the download engine processes them
   - **Then** up to 10 downloads run concurrently (configurable default)
   - **And** downloads execute in parallel using Tokio tasks

2. **AC2: Semaphore-Based Concurrency Control**
   - **Given** the concurrency limit is configured
   - **When** downloads are spawned
   - **Then** a `tokio::sync::Semaphore` limits concurrent connections
   - **And** downloads wait for permit before starting HTTP request
   - **And** permits are released when download completes (success or failure)

3. **AC3: Independent Status Updates**
   - **Given** multiple concurrent downloads
   - **When** each download completes or fails
   - **Then** its status is updated independently in the queue
   - **And** status updates don't block other downloads
   - **And** database operations use the existing Queue API

4. **AC4: Completion Tracking**
   - **Given** a batch of downloads
   - **When** all downloads finish
   - **Then** completion is tracked correctly for all items
   - **And** final count matches: completed + failed = items_processed (items dequeued this run)
   - **And** no items are lost or double-counted

5. **AC5: Configurable Concurrency**
   - **Given** the CLI or configuration
   - **When** the user specifies `--concurrency N` or config value
   - **Then** the semaphore limit is set to N
   - **And** default value is 10 if not specified
   - **And** value must be between 1 and 100

## Tasks / Subtasks

- [x] **Task 1: Create download engine module** (AC: 1, 2)
  - [x] Create `src/download/engine.rs` for DownloadEngine struct
  - [x] Add `concurrency` field with default of 10
  - [x] Create Semaphore for concurrency control
  - [x] Implement `new(concurrency: usize)` constructor with validation (1-100)
  - [x] Add `#[tracing::instrument]` to public methods

- [x] **Task 2: Implement concurrent download processing** (AC: 1, 2, 3)
  - [x] Implement `process_queue(&self, queue: &Queue, client: &HttpClient, output_dir: &Path) -> Result<DownloadStats>`
  - [x] Use `tokio::spawn` to run downloads concurrently
  - [x] Acquire semaphore permit before each download
  - [x] Release permit on completion (RAII via `OwnedSemaphorePermit`)
  - [x] Call `queue.dequeue()` to claim items atomically
  - [x] Call `client.download_file(&item.url, output_dir)` for actual HTTP work
  - [x] Call `queue.mark_completed()` or `queue.mark_failed()` based on result

- [x] **Task 3: Implement download statistics tracking** (AC: 4)
  - [x] Create `DownloadStats` struct: completed, failed fields
  - [x] Use `AtomicUsize` counters for thread-safe concurrent updates (avoid Mutex lock contention)
  - [x] Verify count invariant: completed + failed = items_processed
  - [x] Return stats from `process_queue()`

- [x] **Task 4: Add CLI concurrency flag** (AC: 5)
  - [x] Add `--concurrency` / `-c` flag to clap Args struct in `src/cli.rs`
  - [x] Set default to 10, min 1, max 100
  - [x] Pass value to DownloadEngine constructor
  - [x] Update help text with flag description

- [x] **Task 5: Integrate engine into main workflow** (AC: 1-5)
  - [x] Update `src/main.rs` to create DownloadEngine with concurrency setting
  - [x] Replace any sequential download logic with engine.process_queue()
  - [x] Ensure database, queue, and client are properly wired together
  - [ ] Test end-to-end: `echo "url1\nurl2\nurl3" | downloader` *(Deferred: requires stdin input handling from future story)*

- [x] **Task 6: Write unit tests** (AC: 1-5)
  - [x] Test DownloadEngine::new() with valid concurrency values
  - [x] Test DownloadEngine::new() rejects invalid concurrency (0, 101)
  - [x] Test DownloadStats tracking invariant
  - [x] Test DownloadStats thread-safety with concurrent increments
  - Note: Semaphore concurrency limit test is in integration tests (Task 7) as it requires real Queue/HttpClient
  - Note: Empty queue test is in integration tests (Task 7) as it requires real Queue

- [x] **Task 7: Write integration tests** (AC: 1-5)
  - [x] Create `tests/download_engine_integration.rs`
  - [x] Test concurrent downloads with mock HTTP server (wiremock)
  - [x] Test that semaphore limits are respected (use atomic peak-concurrent counter, NOT timing)
  - [x] Test that all items reach terminal state (completed or failed)
  - [x] Test status updates don't interfere with each other
  - [x] Test mixed success/failure batch (e.g., 3 succeed, 2 fail)
  - [x] Test process_queue with empty queue returns zero stats
  - Note: DB error isolation is implemented (uses `if let Err` + `warn!` logging) but testing requires mock Queue which is out of scope

## Dev Notes

### Context from Previous Stories

**Story 1.4 (SQLite Queue Persistence) established:**
- `src/queue/mod.rs` with Queue struct and all CRUD operations
- `queue.dequeue()` - atomic claim of next pending item (UPDATE...RETURNING)
- `queue.mark_completed(id)` and `queue.mark_failed(id, error)` for status updates
- Queue owns cloned Database (cheap Arc clone)
- QueueStatus enum: Pending, InProgress, Completed, Failed
- 24 integration tests including concurrent dequeue safety test

**Story 1.2 (HTTP Download Core) established:**
- `src/download/mod.rs` with HttpClient and download functionality
- `download_file(url, output_dir)` for streaming downloads
- DownloadError enum for error handling
- Filename extraction and sanitization

**Story 1.1 (Project Scaffolding) established:**
- Tokio async runtime configured
- clap derive macros for CLI
- tracing for structured logging

### Design Decisions

**Semaphore vs Channel Pattern:** Using `tokio::sync::Semaphore` with spawned tasks rather than a bounded channel worker pool. Rationale:
- Simpler to implement and reason about
- Natural RAII cleanup with `OwnedSemaphorePermit`
- Each download is independent, no worker state needed
- Easy to integrate with existing Queue::dequeue() atomic claim pattern

**Task Spawning Pattern:** Each download runs in its own spawned task. The main loop:
1. Calls `queue.dequeue()` which atomically claims an item
2. If item returned, acquires semaphore permit
3. Spawns task to download (holding permit)
4. Continues looping for more items
5. Waits for all tasks to complete

**Statistics Collection:** Use `AtomicUsize` counters for thread-safe stats tracking. Mutex adds unnecessary lock contention for simple counter operations. Wrap counters in Arc for sharing across spawned tasks.

**Error Isolation:** Each download task handles its own errors. A failed download marks itself failed via `queue.mark_failed()` but doesn't affect other downloads.

### Architecture Compliance

**Module Structure (from architecture.md):**
```
src/download/
├── mod.rs          # Re-exports
├── client.rs       # HttpClient (exists from 1.2)
├── engine.rs       # NEW: DownloadEngine
├── progress.rs     # Future: progress tracking
└── stream.rs       # Future: streaming handler
```

**Concurrency Model (ARCH-decision):**
- Global semaphore for total concurrent downloads
- Per-domain rate limiting is a separate concern (Story 1.7)
- Semaphore is fair (FIFO permit granting)

**Error Handling (ARCH-5):**
- Use thiserror for DownloadEngineError if needed
- Individual download errors → mark_failed, don't propagate
- Only propagate fatal errors (database unavailable, etc.)

**Logging (ARCH-6):**
- Add `#[tracing::instrument]` to all public methods
- Log download start/complete at info level
- Log errors at warn/error level
- Use `skip(self)` in instrument to avoid logging large structs

### Concurrency Patterns

**Semaphore Usage:**
```rust
use std::sync::atomic::{AtomicUsize, Ordering};
use tokio::sync::Semaphore;

pub struct DownloadEngine {
    semaphore: Arc<Semaphore>,
    concurrency: usize,
}

pub struct DownloadStats {
    pub completed: AtomicUsize,
    pub failed: AtomicUsize,
}

impl DownloadEngine {
    pub fn new(concurrency: usize) -> Result<Self, EngineError> {
        if !(1..=100).contains(&concurrency) {
            return Err(EngineError::InvalidConcurrency { value: concurrency });
        }
        Ok(Self {
            semaphore: Arc::new(Semaphore::new(concurrency)),
            concurrency,
        })
    }

    pub async fn process_queue(
        &self,
        queue: &Queue,
        client: &HttpClient,
        output_dir: &Path,
    ) -> Result<DownloadStats> {
        let mut handles = Vec::new();
        let stats = Arc::new(DownloadStats::default());

        // Keep dequeuing until no more pending items
        loop {
            let item = match queue.dequeue().await? {
                Some(item) => item,
                None => break, // No more items
            };

            // Acquire permit (blocks if at limit)
            let permit = self.semaphore.clone().acquire_owned().await?;
            let queue = queue.clone();
            let client = client.clone();
            let stats = Arc::clone(&stats);
            let output_dir = output_dir.to_path_buf();

            handles.push(tokio::spawn(async move {
                let _permit = permit; // Dropped when task completes

                let result = client.download_file(&item.url, &output_dir).await;

                match result {
                    Ok(_) => {
                        queue.mark_completed(item.id).await.ok();
                        stats.completed.fetch_add(1, Ordering::SeqCst);
                    }
                    Err(e) => {
                        queue.mark_failed(item.id, &e.to_string()).await.ok();
                        stats.failed.fetch_add(1, Ordering::SeqCst);
                    }
                }
            }));
        }

        // Wait for all downloads to complete
        for handle in handles {
            handle.await.ok();
        }

        // All tasks done, we have sole ownership of Arc
        Arc::try_unwrap(stats)
            .map_err(|_| EngineError::Internal("tasks still hold Arc reference".into()))
    }
}
```

### CLI Integration

**src/cli.rs addition:**
```rust
#[derive(Parser, Debug)]
pub struct Args {
    // ... existing args ...

    /// Maximum concurrent downloads (1-100)
    #[arg(short = 'c', long, default_value = "10", value_parser = clap::value_parser!(u8).range(1..=100))]
    pub concurrency: u8,
}
```

### Test Patterns

**Concurrency Test with Atomic Peak Counter (Deterministic - No Timing):**
```rust
#[tokio::test]
async fn test_semaphore_limits_concurrent_downloads() {
    // Use atomic counters to track peak concurrent operations
    // Mock handlers: increment on request start, decrement on completion
    // Track maximum value seen during test execution
    //
    // Example approach:
    // - Create shared AtomicUsize for current_concurrent and peak_concurrent
    // - In wiremock response handler, increment current, update peak via fetch_max
    // - After small delay, decrement current
    // - After test, assert peak_concurrent <= configured_limit (2)
    // - Also assert peak_concurrent >= 2 (actually hit the limit with 5 items)
    //
    // This approach is deterministic - no timing dependencies that flake in CI

    let mock_server = MockServer::start().await;
    let current = Arc::new(AtomicUsize::new(0));
    let peak = Arc::new(AtomicUsize::new(0));

    // Setup mocks that track concurrency...
    // (Implementation details left to dev - key is atomic tracking, not timing)

    let engine = DownloadEngine::new(2); // Only 2 concurrent
    engine.process_queue(&queue, &client, &output_dir).await.unwrap();

    assert!(peak.load(Ordering::SeqCst) <= 2, "Semaphore should limit to 2 concurrent");
    assert!(peak.load(Ordering::SeqCst) >= 2, "Should reach concurrency limit with 5 items");
}
```

### Pre-Commit Checklist

Before marking complete:
```bash
cargo fmt --check           # Formatting
cargo clippy -- -D warnings # Lints as errors
cargo test                  # All tests pass
cargo build --release       # Release build works
```

### Project Structure Notes

- New file: `src/download/engine.rs`
- Modified: `src/download/mod.rs` (add engine export)
- Modified: `src/cli.rs` (add --concurrency flag)
- Modified: `src/main.rs` (wire up engine)
- New file: `tests/download_engine_integration.rs`

### Critical Anti-Patterns to Avoid

| Anti-Pattern | Correct Approach |
|--------------|------------------|
| Unbounded task spawning | Use Semaphore to limit concurrency |
| Blocking in async tasks | Use async-aware operations only |
| Panic on download error | Return Result, handle gracefully |
| Shared mutable state without sync | Use Arc<Mutex<>> or atomics |
| Polling queue in tight loop | dequeue() returns None when empty |

### References

- [Source: architecture.md#Concurrency-Model]
- [Source: architecture.md#Project-Structure]
- [Source: epics.md#Story-1.5]
- [Source: 1-4-sqlite-queue-persistence.md#Queue-Operations]
- [Source: 1-2-http-download-core.md] (for HttpClient)

---

## Party Mode Review (2026-02-02)

**Reviewers:** Winston (Architect), Amelia (Dev), Murat (Test Architect), Mary (Analyst), Bob (SM)

**Final Approvers:** Amelia (Dev), Murat (Test Architect)

### Issues Identified and Fixed

| Issue | Severity | Resolution |
|-------|----------|------------|
| AC4 `total` undefined | Medium | Clarified: `completed + failed = items_processed` |
| `download_item()` vs `download_file()` naming | Low | Standardized to `client.download_file()` |
| Timing-based test is flaky | High | Replaced with atomic peak-concurrent counter |
| Triple-unwrap in code sample | Medium | Used proper error handling with map_err |
| Output directory not in signature | Medium | Added `output_dir: &Path` parameter |
| Mutex vs AtomicUsize indecision | Low | Prescribed AtomicUsize for counters |
| Missing test: empty queue | Low | Added to Task 6 |
| Missing test: mixed success/failure | Medium | Added to Task 7 |
| Missing test: error isolation | Medium | Added to Task 7 |

**Verdict:** Story approved for implementation after fixes applied.

---

## Dev Agent Record

### Agent Model Used

Claude Opus 4.5 (claude-opus-4-5-20251101)

### Debug Log References

N/A

### Completion Notes List

- All 7 tasks completed (1 subtask deferred - stdin input handling)
- 8 unit tests in `engine.rs` pass
- 12 CLI tests pass (7 new for concurrency flag)
- 8 integration tests in `download_engine_integration.rs` pass (refactored to use Result return)
- Pre-existing test failure `test_extract_urls_preserves_wikipedia_style_parens` is unrelated to this story
- Pre-existing clippy warnings in db.rs, client.rs, parser/*.rs are unrelated to this story
- New code in engine.rs has no clippy warnings
- **Integration Note:** main.rs uses in-memory DB and has no stdin input yet - full end-to-end testing requires future story for input handling (Story 7.1 stdin-piped-input)

### Change Log

- Created `src/download/engine.rs` with DownloadEngine, DownloadStats, EngineError
- Modified `src/download/mod.rs` to export engine module
- Modified `src/lib.rs` to re-export engine types
- Modified `src/cli.rs` to add `--concurrency` / `-c` flag with validation (1-100)
- Modified `src/main.rs` to integrate engine with queue processing
- Created `tests/download_engine_integration.rs` with 8 integration tests

**Code Review Fixes (2026-02-02):**
- Added `#[instrument(skip(self))]` to `concurrency()` method
- Changed module doc example from `ignore` to `no_run` with proper async wrapper
- Refactored all integration tests to return `Result<(), Box<dyn Error>>` (no more `.expect()`)
- Added detailed doc comment on `ConcurrencyTrackingResponder` explaining why `std::thread::sleep` is acceptable
- Updated story tasks to accurately reflect what's unit vs integration tested
- Documented deferred stdin input subtask (belongs to Story 7.1)

### File List

| File | Action | Description |
|------|--------|-------------|
| `src/download/engine.rs` | Created | DownloadEngine with semaphore-based concurrency, DownloadStats with AtomicUsize counters, EngineError enum, 8 unit tests |
| `src/download/mod.rs` | Modified | Added `mod engine` and re-exports |
| `src/lib.rs` | Modified | Added re-exports for DownloadEngine, DownloadStats, EngineError, DEFAULT_CONCURRENCY |
| `src/cli.rs` | Modified | Added `--concurrency` / `-c` flag (1-100, default 10), 7 new tests |
| `src/main.rs` | Modified | Integrated DownloadEngine with database, queue, and HttpClient |
| `tests/download_engine_integration.rs` | Created | 8 integration tests covering empty queue, success/failure, concurrency limits, error isolation |

