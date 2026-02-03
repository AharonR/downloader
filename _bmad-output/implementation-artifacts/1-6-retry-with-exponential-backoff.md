# Story 1.6: Retry with Exponential Backoff

Status: done

## Story

As a **user**,
I want **failed downloads to retry automatically**,
So that **transient errors don't require manual intervention**.

## Acceptance Criteria

1. **AC1: Automatic Retry on Transient Errors**
   - **Given** a download that fails due to network error (timeout, connection refused, 5xx)
   - **When** the retry logic activates
   - **Then** the download retries up to 3 times (configurable)
   - **And** the retry is triggered automatically without user intervention

2. **AC2: Exponential Backoff Delays**
   - **Given** a download that needs retry
   - **When** retry attempts are made
   - **Then** delays increase exponentially (1s, 2s, 4s base delays)
   - **And** the pattern follows 2^attempt seconds
   - **And** backoff respects configured base and max delays

3. **AC3: Permanent Failure Classification**
   - **Given** a download that fails with HTTP 400, 404, 410, 451
   - **When** the error is evaluated
   - **Then** permanent failures do NOT retry
   - **And** the item is immediately marked as failed with appropriate error message
   - **And** the failure type is categorized (Permanent vs Transient vs RateLimited)
   - **Note:** 401/403 classified as Permanent for now; will become NeedsAuth in Epic 4 (Authenticated Downloads)

4. **AC4: Retry Logging**
   - **Given** a download retry attempt
   - **When** the retry is executed
   - **Then** retry attempts are logged with reason
   - **And** log includes: attempt number, delay, original error
   - **And** logging uses tracing at info/debug levels

5. **AC5: Retry Count Tracking**
   - **Given** multiple retry attempts
   - **When** the retry policy is applied
   - **Then** retry count is tracked per download item (in-memory during retry loop)
   - **And** max retries is respected (default 3)
   - **And** after max retries, item is marked as permanently failed
   - **And** final retry_count is persisted to database on failure

6. **AC6: Rate Limit Handling (Deferred)**
   - **Note:** HTTP 429 with Retry-After header handling is deferred to Story 1.7 (Per-Domain Rate Limiting)
   - **For this story:** 429 is classified as `RateLimited` and retries with exponential backoff
   - **Future:** Story 1.7 will add Retry-After header parsing and respect server-specified delays

## Tasks / Subtasks

- [x] **Task 1: Create retry module** (AC: 1, 2, 3)
  - [x] Create `src/download/retry.rs` for retry logic
  - [x] Define `RetryPolicy` struct with max_attempts, base_delay, max_delay, backoff_multiplier
  - [x] Define `RetryDecision` enum: `Retry { delay: Duration, attempt: u32 }`, `DoNotRetry { reason: String }`
  - [x] Implement `Default` for RetryPolicy (3 attempts, 1s base, 32s max, 2.0 multiplier)
  - [x] Add `#[tracing::instrument]` to public methods

- [x] **Task 2: Implement error classification** (AC: 3)
  - [x] Define `FailureType` enum: `Transient`, `Permanent`, `RateLimited`, `NeedsAuth`
  - [x] Implement `classify_error(error: &DownloadError) -> FailureType`
  - [x] Map HTTP status codes (429 is RateLimited ONLY, not Transient):
    - `Transient`: 408, 500, 502, 503, 504, timeouts, connection errors
    - `Permanent`: 400, 404, 410, 451
    - `NeedsAuth`: 401, 403 (treated as Permanent until Epic 4)
    - `RateLimited`: 429 (retries with backoff; Retry-After support in Story 1.7)
  - [x] Write unit tests for each status code classification

- [x] **Task 3: Implement retry decision logic** (AC: 1, 2, 5)
  - [x] Implement `RetryPolicy::should_retry(failure_type, attempt_count) -> RetryDecision`
  - [x] Calculate exponential delay: `min(base_delay * 2^attempt, max_delay)`
  - [x] Add jitter (0-500ms random) to prevent thundering herd
  - [x] Return `DoNotRetry` if attempt_count >= max_attempts
  - [x] Return `DoNotRetry` if FailureType::Permanent or FailureType::NeedsAuth
  - [x] Return `Retry` for FailureType::Transient and FailureType::RateLimited
  - [x] Write unit tests for delay calculation and decision logic

- [x] **Task 4: Integrate retry into download engine** (AC: 1, 2, 4, 5)
  - [x] Modify `DownloadEngine` to accept `RetryPolicy` in constructor
  - [x] Update download task to wrap HTTP call in retry loop
  - [x] Use `tokio::time::sleep` for backoff delays
  - [x] Add `retried: AtomicUsize` to DownloadStats for retry attempt tracking
  - [x] Log retry attempts at info level: "Retrying download (attempt {}/{}): {url}"
  - [x] Log backoff at debug level: "Waiting {}s before retry"

- [x] **Task 5: Utilize existing queue schema for retry tracking** (AC: 5)
  - [x] Schema ALREADY EXISTS: `retry_count` and `last_error` columns in `queue` table (see migrations/20260128000001)
  - [x] Verified `Queue::mark_failed()` already increments retry_count
  - [x] Final retry_count persisted when marking failed after retries exhausted
  - [x] NO migration needed - schema is ready

- [x] **Task 6: Add CLI retry configuration** (AC: 1, 2)
  - [x] Add `--max-retries` / `-r` flag (default 3, min 0, max 10)
  - [x] Pass RetryPolicy to DownloadEngine

- [x] **Task 7: Write unit tests** (AC: 1-5)
  - [x] Test RetryPolicy::default() values
  - [x] Test exponential backoff delay calculation at each attempt level
  - [x] Test classify_error for all HTTP status codes (verify 429 → RateLimited, not Transient)
  - [x] Test should_retry returns DoNotRetry for Permanent errors
  - [x] Test should_retry returns DoNotRetry for NeedsAuth errors
  - [x] Test should_retry returns Retry for Transient errors
  - [x] Test should_retry returns Retry for RateLimited errors
  - [x] Test should_retry respects max_attempts
  - [x] Test jitter: N=100 samples, verify all in [0, 500ms], mean roughly 250ms

- [x] **Task 8: Write integration tests** (AC: 1-5)
  - [x] Test 503→200 sequence: first request fails, retry succeeds
  - [x] Test transient error (503) triggers retry and eventually succeeds
  - [x] Test permanent error (404) does not retry (expect exactly 1 request)
  - [x] Test 401/403 does not retry (NeedsAuth, expect exactly 1 request)
  - [x] Test 429 triggers retry with backoff
  - [x] Test max retries exhausted marks item failed
  - [ ] Test retry logging output (capture tracing subscriber) *(Deferred: requires tracing-test setup)*
  - [x] Test final retry_count persisted in database after failure

## Dev Notes

### Context from Previous Stories

**Story 1.5 (Concurrent Downloads) established:**
- `src/download/engine.rs` with DownloadEngine struct
- `process_queue()` method spawning tasks with semaphore control
- `DownloadStats` with AtomicUsize counters (completed, failed)
- Pattern: `client.download_file(&item.url, &output_dir).await`
- Errors call `queue.mark_failed(item.id, &e.to_string())`

**Story 1.4 (SQLite Queue Persistence) established:**
- `src/queue/mod.rs` with Queue struct
- `QueueStatus` enum: Pending, InProgress, Completed, Failed
- `queue` table with `retry_count INTEGER DEFAULT 0` and `last_error TEXT` columns
- `mark_failed(id, error_message)` updates status

**Story 1.2 (HTTP Download Core) established:**
- `src/download/client.rs` with HttpClient
- `DownloadError` enum for error handling
- Uses reqwest under the hood

### Architecture Compliance

**Error Classification (from architecture.md):**
```rust
enum FailureType {
    Transient,   // Network timeout, 5xx -> auto-retry
    Permanent,   // 400, 404, 410, 451 -> mark failed immediately
    NeedsAuth,   // 401, 403 -> treated as Permanent until Epic 4
    RateLimited, // 429 -> retry with backoff (Retry-After in Story 1.7)
}
```

**Retry Policy (from architecture.md):**
```rust
struct RetryPolicy {
    max_attempts: u32,        // 3
    backoff_base: Duration,   // 1s (user-friendly for CLI tool)
    backoff_max: Duration,    // 32s (allows 5 doublings)
    backoff_multiplier: f32,  // 2.0
}
```

**Design Decision - 1s Base Delay:** Architecture originally specified 5s, but 1s chosen for better CLI UX. A download tool that makes users wait 5s before first retry feels sluggish. 1s→2s→4s provides reasonable delays while keeping the tool responsive.

**Module Structure:**
```
src/download/
├── mod.rs          # Re-exports
├── client.rs       # HttpClient (from 1.2)
├── engine.rs       # DownloadEngine (from 1.5)
├── retry.rs        # NEW: RetryPolicy, FailureType, retry logic
├── progress.rs     # Future: progress tracking
└── stream.rs       # Future: streaming handler
```

### Design Decisions

**Retry Location:** Retry logic lives in the download task, not in HttpClient. This keeps HttpClient simple (single request) and allows engine-level control of retry policy.

**In-Memory Retry Tracking:** Retry attempts are tracked in-memory during the retry loop. Only the final count is persisted to the database when marking an item as failed. This is acceptable because:
- If process crashes mid-retry, the item remains "in_progress" and can be reclaimed
- Story 3.6 (Resumable Downloads) will handle crash recovery properly
- Simplifies implementation without sacrificing reliability

**Exponential Backoff Formula:**
```
delay = min(base_delay * multiplier^attempt, max_delay) + jitter
```
Where:
- base_delay = 1 second
- multiplier = 2.0
- max_delay = 32 seconds
- jitter = random(0, 500ms)

**Delays for default policy:** 1s, 2s, 4s (then fail after 3 attempts)

**Jitter:** Random 0-500ms added to prevent thundering herd when multiple downloads fail simultaneously. Use `rand` crate with `thread_rng()`.

**Error Classification Strategy:**
| HTTP Status | FailureType | Rationale |
|-------------|-------------|-----------|
| 400 | Permanent | Bad request - won't succeed on retry |
| 401 | NeedsAuth | Unauthorized - needs auth (Epic 4) |
| 403 | NeedsAuth | Forbidden - needs auth (Epic 4) |
| 404 | Permanent | Not found - resource doesn't exist |
| 408 | Transient | Request timeout - retry may succeed |
| 410 | Permanent | Gone - permanently removed |
| 429 | RateLimited | Rate limited - retry with backoff |
| 451 | Permanent | Legal block - won't succeed |
| 500 | Transient | Server error - may be temporary |
| 502 | Transient | Bad gateway - proxy issue |
| 503 | Transient | Service unavailable - temporary |
| 504 | Transient | Gateway timeout - temporary |

**Non-HTTP Errors:**
| Error Type | FailureType | Rationale |
|------------|-------------|-----------|
| Connection refused | Transient | Server may come back |
| Timeout | Transient | Network may recover |
| DNS resolution | Transient | DNS may recover |
| TLS error | Permanent | Certificate/config issue |
| Invalid URL | Permanent | Won't succeed |

### Implementation Pattern

**Retry wrapper in engine:**
```rust
async fn download_with_retry(
    client: &HttpClient,
    item: &QueueItem,
    output_dir: &Path,
    policy: &RetryPolicy,
) -> Result<PathBuf, (DownloadError, u32)> {  // Returns error + attempt count
    let mut attempts = 0;

    loop {
        attempts += 1;

        match client.download_file(&item.url, output_dir).await {
            Ok(path) => return Ok(path),
            Err(e) => {
                let failure_type = classify_error(&e);
                match policy.should_retry(failure_type, attempts) {
                    RetryDecision::Retry { delay, attempt } => {
                        info!(url = %item.url, attempt, delay_ms = delay.as_millis(), "Retrying download");
                        tokio::time::sleep(delay).await;
                    }
                    RetryDecision::DoNotRetry { reason } => {
                        debug!(url = %item.url, %reason, "Not retrying download");
                        return Err((e, attempts));
                    }
                }
            }
        }
    }
}
```

### Testing Strategy

**Unit Tests (in retry.rs):**
- Test error classification for each status code
- Test exponential delay calculation at each attempt
- Test jitter bounds (N=100 samples, verify range and distribution)
- Test policy respects max_attempts

**Integration Tests:**
- Use wiremock with `up_to_n_times(1)` for sequential response testing
- Use wiremock to return 404 (verify no retry, exactly 1 request)
- Use wiremock to return 503 indefinitely (verify max retries)
- Verify retry_count in database after failures

**Wiremock Pattern for Retry Testing (CORRECTED):**
```rust
use wiremock::{Mock, MockServer, ResponseTemplate};
use wiremock::matchers::{method, path};

#[tokio::test]
async fn test_retry_succeeds_after_transient_failure() -> Result<(), Box<dyn std::error::Error>> {
    let mock = MockServer::start().await;

    // IMPORTANT: Use up_to_n_times(1) for sequential responses
    // First call returns 503
    Mock::given(method("GET"))
        .and(path("/paper.pdf"))
        .respond_with(ResponseTemplate::new(503))
        .up_to_n_times(1)  // Matches exactly once, then falls through
        .mount(&mock)
        .await;

    // Second call (after retry) returns 200
    Mock::given(method("GET"))
        .and(path("/paper.pdf"))
        .respond_with(ResponseTemplate::new(200).set_body_bytes(b"PDF content"))
        .mount(&mock)
        .await;

    // Run download with retry policy...
    // Assert download succeeded
    // Assert exactly 2 requests were made
    Ok(())
}
```

**Jitter Statistical Test Pattern:**
```rust
#[test]
fn test_jitter_distribution() {
    let policy = RetryPolicy::default();
    let samples: Vec<Duration> = (0..100)
        .map(|_| policy.calculate_jitter())
        .collect();

    // All samples in valid range
    assert!(samples.iter().all(|d| d.as_millis() <= 500));

    // Mean should be roughly 250ms (within 100ms tolerance)
    let mean_ms = samples.iter().map(|d| d.as_millis()).sum::<u128>() / 100;
    assert!((200..300).contains(&mean_ms), "Jitter mean {} not near 250ms", mean_ms);
}
```

### Dependencies

**New crate needed:** `rand` for jitter
```toml
# Cargo.toml addition
rand = "0.8"
```

### Project Structure Notes

- New file: `src/download/retry.rs`
- Modified: `src/download/mod.rs` (add retry export)
- Modified: `src/download/engine.rs` (integrate retry, add retried stat)
- Modified: `src/cli.rs` (add --max-retries)
- Modified: `src/queue/mod.rs` (update mark_failed signature)
- NO migration needed - schema already has retry_count and last_error

### Pre-Commit Checklist

```bash
cargo fmt --check           # Formatting
cargo clippy -- -D warnings # Lints as errors
cargo test                  # All tests pass
cargo build --release       # Release build works
```

### References

- [Source: architecture.md#Resilience-&-Crash-Safety]
- [Source: architecture.md#Logging-&-Observability]
- [Source: epics.md#Story-1.6]
- [Source: project-context.md#Async-Patterns]
- [Source: 1-5-concurrent-downloads.md] (for DownloadEngine patterns)
- [Source: prd.md#FR-2.4] - Retry with exponential backoff requirement
- [Source: migrations/20260128000001] - Queue schema with retry_count

### Critical Anti-Patterns to Avoid

| Anti-Pattern | Correct Approach |
|--------------|------------------|
| `std::thread::sleep` for delays | Use `tokio::time::sleep` |
| Retrying permanent errors | Classify errors before retry decision |
| No max retry limit | Always enforce max_attempts |
| Fixed delays (no backoff) | Use exponential backoff with jitter |
| Retry in HttpClient | Keep retry in engine layer |
| Blocking random generation | Use `thread_rng()` which is non-blocking |
| 429 as Transient | 429 is RateLimited (distinct category) |
| wiremock `expect(1)` for sequences | Use `up_to_n_times(1)` for ordered responses |

### Key Learnings from Story 1.5

- AtomicUsize works well for concurrent stats tracking
- Integration tests should return `Result<(), Box<dyn Error>>` not use .expect()
- wiremock async patterns are the standard for HTTP mocking
- `#[tracing::instrument]` on all public methods
- Deferred subtasks should be documented clearly

---

## Party Mode Review (2026-02-02)

**Reviewers:** Winston (Architect), Amelia (Dev), Murat (Test Architect), Mary (Analyst), Bob (SM)

### Issues Identified and Fixed

| Issue | Severity | Resolution |
|-------|----------|------------|
| 429 classified as both Transient and RateLimited | High | Fixed: 429 is RateLimited ONLY |
| Retry count lost on crash | Medium | Documented: in-memory during loop, persisted on final failure |
| Wiremock test example incorrect | High | Fixed: use `up_to_n_times(1)` pattern |
| No Retry-After header handling | Medium | Added AC6 deferring to Story 1.7 |
| Base delay 1s vs 5s unclear | Low | Documented rationale for 1s choice |
| Task 5 "already exists" contradiction | Medium | Clarified: schema EXISTS, no migration needed |
| Missing NeedsAuth failure type | Low | Added to FailureType enum |
| Jitter test statistically weak | Medium | Added N=100 statistical test pattern |
| Missing 503→200 sequence test | Medium | Added explicit test case |
| No retry stats in DownloadStats | Low | Added `retried: AtomicUsize` to Task 4 |

**Verdict:** Story approved for implementation with all fixes applied.

---

## Dev Agent Record

### Agent Model Used

Claude Opus 4.5 (claude-opus-4-5-20251101)

### Debug Log References

N/A

### Completion Notes List

- All 8 tasks completed successfully
- 33 unit tests in `retry.rs` pass
- 18 CLI tests pass (including 6 new for max-retries flag)
- 15 integration tests in `download_engine_integration.rs` pass (including 8 new retry tests)
- Pre-existing test failure `test_extract_urls_preserves_wikipedia_style_parens` is unrelated to this story
- Pre-existing clippy warnings in db.rs, client.rs, parser/*.rs are unrelated to this story
- New code in retry.rs and engine.rs has no clippy warnings (match_same_arms allowed intentionally for documentation)

### Change Log

- Created `src/download/retry.rs` with RetryPolicy, FailureType, RetryDecision, classify_error
- Modified `src/download/mod.rs` to export retry module
- Modified `src/lib.rs` to re-export retry types
- Modified `src/download/engine.rs` to integrate retry logic:
  - Added RetryPolicy field to DownloadEngine
  - Added `retried` counter to DownloadStats
  - Added `download_with_retry()` helper function
  - Updated process_queue to use retry logic
- Modified `src/cli.rs` to add `--max-retries` / `-r` flag (0-10, default 3)
- Modified `src/main.rs` to use RetryPolicy from CLI args
- Modified `tests/download_engine_integration.rs` with 8 new retry tests
- Added `rand = "0.8"` to Cargo.toml for jitter

### File List

| File | Action | Description |
|------|--------|-------------|
| `src/download/retry.rs` | Created | RetryPolicy, FailureType, RetryDecision, classify_error, DEFAULT_MAX_RETRIES, 33 unit tests |
| `src/download/mod.rs` | Modified | Added `mod retry` and re-exports |
| `src/lib.rs` | Modified | Added re-exports for retry types |
| `src/download/engine.rs` | Modified | Integrated retry with exponential backoff, added retried stat, updated DownloadEngine constructor |
| `src/cli.rs` | Modified | Added `--max-retries` / `-r` flag (0-10, default 3), 6 new tests |
| `src/main.rs` | Modified | Uses RetryPolicy from CLI args |
| `tests/download_engine_integration.rs` | Modified | 8 new retry integration tests |
| `Cargo.toml` | Modified | Added `rand = "0.8"` dependency |
