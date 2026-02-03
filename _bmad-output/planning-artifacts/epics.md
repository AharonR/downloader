---
stepsCompleted: [1, 2, 3, 4]
status: complete
inputDocuments:
  - "_bmad-output/planning-artifacts/prd.md"
  - "_bmad-output/planning-artifacts/architecture.md"
  - "_bmad-output/planning-artifacts/ux-design-specification.md"
  - "_bmad-output/project-context.md"
---

# Downloader - Epic Breakdown

## Overview

This document provides the complete epic and story breakdown for Downloader, decomposing the requirements from the PRD, UX Design, and Architecture into implementable stories.

## Requirements Inventory

### Functional Requirements

**FR-1: Input Parsing**
- FR-1.1: Accept direct URLs (http/https) [Must]
- FR-1.2: Resolve DOIs to downloadable URLs [Must]
- FR-1.3: Parse reference strings (Author, Year, Title format) [Must]
- FR-1.4: Extract references from pasted bibliographies [Must]
- FR-1.5: Accept BibTeX format [Should]
- FR-1.6: Handle mixed-format input (URLs + DOIs + references) [Must]

**FR-2: Download Engine**
- FR-2.1: Download files via HTTP/HTTPS [Must]
- FR-2.2: Support authenticated sites via cookie/session capture [Must]
- FR-2.3: Implement site-specific resolvers for common academic sites [Must]
- FR-2.4: Retry failed downloads with exponential backoff [Must]
- FR-2.5: Support concurrent downloads (configurable, default 10) [Must]
- FR-2.6: Rate limit requests per domain [Must]
- FR-2.7: Support resumable downloads (Range requests) [Should]
- FR-2.8: Accept cookies from file (Netscape format) or stdin [Must] *(added from PM review)*

**FR-3: Organization**
- FR-3.1: Create project folders from CLI flag [Must]
- FR-3.2: Support sub-project organization [Must]
- FR-3.3: Name files from metadata (Author_Year_Title.ext) [Must]
- FR-3.4: Generate index.md per project with file listing [Must]
- FR-3.5: Auto-detect topics via keyword extraction [Should]
- FR-3.6: Store metadata as JSON-LD sidecar files [Should]

**FR-4: Logging & Memory**
- FR-4.1: Log all download attempts with status [Must]
- FR-4.2: Log failures with actionable error info [Must]
- FR-4.3: Create per-project download.log [Must]
- FR-4.4: Track parsing confidence for ambiguous references [Should]
- FR-4.5: Enable querying past downloads [Should]

**FR-5: CLI Interface**
- FR-5.1: Accept input via stdin (piped bibliography) [Must]
- FR-5.2: Accept --project flag for organization [Must]
- FR-5.3: Display progress during download [Must]
- FR-5.4: Show summary on completion [Must]
- FR-5.5: Support --dry-run for preview [Must] *(promoted from Should)*
- FR-5.6: Support configuration file for defaults [Must] *(promoted from Should)*
- FR-5.7: Display helpful usage when invoked with no input [Must] *(added from PM review)*

### Non-Functional Requirements

**NFR-1: Performance**
- NFR-1.1: Parse 150 references < 5 seconds
- NFR-1.2: Concurrent downloads: 10 parallel (different domains)
- NFR-1.3: Memory usage < 200MB during operation
- NFR-1.4: Startup time < 1 second

**NFR-2: Reliability**
- NFR-2.1: Download success rate ≥ 90%
- NFR-2.2: Auth site success rate ≥ 70%
- NFR-2.3: Naming accuracy ≥ 95%
- NFR-2.4: Graceful failure handling - never crash, always log

**NFR-3: Usability**
- NFR-3.1: Zero-config start - works with defaults
- NFR-3.2: Clear error messages - actionable, not cryptic
- NFR-3.3: Progress visibility - user knows what's happening

**NFR-4: Maintainability**
- NFR-4.1: Site resolver modularity - easy to add new resolvers
- NFR-4.2: Configuration flexibility - overridable defaults
- NFR-4.3: Logging for debugging - sufficient for resolver improvement

### Additional Requirements

**From Architecture:**
- ARCH-1: Project scaffolding with lib/bin split (single crate, dual targets)
- ARCH-2: Rust 2024 edition with Tokio async runtime
- ARCH-3: SQLite database for queue persistence, metadata, and logs
- ARCH-4: Resolver trait + registry pattern with priority-based fallback
- ARCH-5: thiserror for library errors, anyhow for binary only
- ARCH-6: tracing for structured logging with #[instrument] on public functions
- ARCH-7: clap derive macros for CLI argument parsing
- ARCH-8: indicatif for progress bar display
- ARCH-9: WAL mode for SQLite concurrent reads
- ARCH-10: Cookie persistence opt-in with keychain for encryption key

**From UX Design:**
- UX-1: Input parsing feedback - show parsed item counts by type
- UX-2: Progress design - spinners, in-place updates, status line format
- UX-3: Completion summary - success/failure counts, organized output path
- UX-4: Error message pattern - What/Why/Fix structure
- UX-5: Verbosity levels - default, --verbose, --quiet, --debug
- UX-6: Exit codes - 0 (success), 1 (partial), 2 (failure)
- UX-7: Terminal compatibility - width detection, NO_COLOR support
- UX-8: Interrupt handling - Ctrl+C graceful stop with partial progress

**From Project Context:**
- CTX-1: RFC 430 naming conventions throughout
- CTX-2: Import organization: std → external → internal
- CTX-3: Unit tests inline with #[cfg(test)], integration tests in tests/
- CTX-4: Test naming: test_<unit>_<scenario>_<expected>
- CTX-5: 80%+ test coverage target for library code
- CTX-6: cargo fmt && cargo clippy -- -D warnings before every commit

### FR Coverage Map

| Requirement | Epic | Coverage |
|-------------|------|----------|
| **Input Parsing** | | |
| FR-1.1: Direct URLs | Epic 1 | ✓ Foundation |
| FR-1.2: DOI resolution | Epic 2 | ✓ Smart Resolution |
| FR-1.3: Reference parsing | Epic 2 | ✓ Smart Resolution |
| FR-1.4: Bibliography extraction | Epic 2 | ✓ Smart Resolution |
| FR-1.5: BibTeX format | Epic 2 | ✓ Smart Resolution |
| FR-1.6: Mixed-format input | Epic 2 | ✓ Smart Resolution |
| **Download Engine** | | |
| FR-2.1: HTTP/HTTPS download | Epic 1 | ✓ Foundation |
| FR-2.2: Authenticated sites | Epic 4 | ✓ Auth Downloads |
| FR-2.3: Site-specific resolvers | Epic 2 | ✓ Smart Resolution |
| FR-2.4: Retry with backoff | Epic 1 | ✓ Foundation |
| FR-2.5: Concurrent downloads | Epic 1 | ✓ Foundation |
| FR-2.6: Rate limiting | Epic 1 | ✓ Foundation |
| FR-2.7: Resumable downloads | Epic 3 | ✓ Batch Processing |
| FR-2.8: Cookie file input | Epic 4 | ✓ Auth Downloads |
| **Organization** | | |
| FR-3.1: Project folders | Epic 5 | ✓ Organized Output |
| FR-3.2: Sub-project organization | Epic 5 | ✓ Organized Output |
| FR-3.3: Metadata file naming | Epic 5 | ✓ Organized Output |
| FR-3.4: Index generation | Epic 5 | ✓ Organized Output |
| FR-3.5: Topic auto-detection | Epic 8 | Should → Polish |
| FR-3.6: JSON-LD sidecar files | Epic 8 | Should → Polish |
| **Logging & Memory** | | |
| FR-4.1: Download attempt logging | Epic 6 | ✓ Download History |
| FR-4.2: Failure logging | Epic 6 | ✓ Download History |
| FR-4.3: Per-project download.log | Epic 6 | ✓ Download History |
| FR-4.4: Parsing confidence tracking | Epic 8 | Should → Polish |
| FR-4.5: Query past downloads | Epic 8 | Should → Polish |
| **CLI Interface** | | |
| FR-5.1: stdin input | Epic 7 | ✓ Professional CLI |
| FR-5.2: --project flag | Epic 5 | ✓ Organized Output |
| FR-5.3: Progress display | Epic 3 | ✓ Batch Processing |
| FR-5.4: Completion summary | Epic 3 | ✓ Batch Processing |
| FR-5.5: --dry-run | Epic 7 | ✓ Professional CLI |
| FR-5.6: Config file | Epic 7 | ✓ Professional CLI |
| FR-5.7: No-input help | Epic 7 | ✓ Professional CLI |
| **UX Requirements** | | |
| UX-1: Input parsing feedback | Epic 3 | ✓ Batch Processing |
| UX-2: Progress design | Epic 3 | ✓ Batch Processing |
| UX-3: Completion summary | Epic 3 | ✓ Batch Processing |
| UX-4: Error message pattern | Epic 7 | ✓ Professional CLI |
| UX-5: Verbosity levels | Epic 7 | ✓ Professional CLI |
| UX-6: Exit codes | Epic 7 | ✓ Professional CLI |
| UX-7: Terminal compatibility | Epic 7 | ✓ Professional CLI |
| UX-8: Interrupt handling | Epic 3 | ✓ Batch Processing |

**Coverage Summary:**
- Epic 1: 6 FRs (Foundation) - 8 stories (including 1.0)
- Epic 2: 5 FRs (Smart Resolution) - 7 stories
- Epic 3: 5 FRs + 4 UX (Batch Processing) - 6 stories
- Epic 4: 2 FRs (Auth) - 5 stories
- Epic 5: 5 FRs (Organization) - 4 stories
- Epic 6: 3 FRs (History) - 4 stories
- Epic 7: 4 FRs + 4 UX (Professional CLI) - 8 stories
- Epic 8: 5 Should items (Polish) - 5 stories

**Total: 47 stories across 8 epics**

## Epic Structure

**User-Value Focused Organization (Party Mode refined):**

| Epic | Name | User Value | Primary FRs | Dependencies |
|------|------|------------|-------------|--------------|
| 1 | Download Any List | "I can paste URLs and get files" | FR-1.1, FR-2.1, FR-2.4-2.6, ARCH-1-10 | None |
| 2 | Smart Reference Resolution | "I can paste DOIs and references" | FR-1.2-1.6, FR-2.3 | Epic 1 |
| 3 | Reliable Batch Processing | "I can walk away and trust it works" | FR-5.3, FR-5.4, FR-2.7, UX-1-3, UX-8 | Epic 2 |
| 4 | Authenticated Downloads | "I can download from my subscriptions" | FR-2.2, FR-2.8, ARCH-10 | Epic 3 |
| 5 | Organized Output | "My downloads are organized automatically" | FR-3.*, FR-5.2 | Epic 3 |
| 6 | Download History | "I can see what I've downloaded" | FR-4.* | Epic 3 |
| 7 | Professional CLI | "The tool feels polished and helpful" | FR-5.1, FR-5.5-5.7, UX-4-7 | Epics 3, 5, 6 |
| 8 | Polish & Enhancements | "Everything works even better" | All Should items | All Must epics |

**MVP Cut Line:** Epics 1-5 = Minimum Viable Product

**Dependency Chain:**
```
Epic 1 (Download Any List) - Foundation + First Value
    │
    └── Epic 2 (Smart Reference Resolution)
            │
            └── Epic 3 (Reliable Batch Processing)
                    ├── Epic 4 (Authenticated Downloads)
                    ├── Epic 5 (Organized Output)
                    └── Epic 6 (Download History)
                            │
                            └── Epic 7 (Professional CLI) [requires 3, 5, 6]
                                    │
                                    └── Epic 8 (Polish) [after all Must]
```

**Key Insight:** Epic 1 combines infrastructure setup with immediate user value—after Epic 1, users can already download URL lists. No separate "infrastructure-only" epic.

## Epic List

### Epic 1: Download Any List
**User Value:** "I can paste URLs and get files"

**Scope:**
- Project scaffolding (lib/bin split, Cargo.toml)
- Core async runtime setup (Tokio)
- HTTP client with connection pooling (reqwest)
- SQLite database schema and WAL mode
- URL input detection and validation
- Basic download engine (single and concurrent)
- Retry logic with exponential backoff
- Per-domain rate limiting
- Minimal CLI to accept URLs and download

**Exit Criteria:** User can run `echo "https://example.com/paper.pdf" | downloader` and get a file.

**FRs:** FR-1.1, FR-2.1, FR-2.4, FR-2.5, FR-2.6
**ARCH:** ARCH-1 through ARCH-9

---

### Epic 2: Smart Reference Resolution
**User Value:** "I can paste DOIs and references"

**Scope:**
- DOI detection and validation
- Crossref API integration for DOI resolution
- Reference string parsing (Author, Year, Title)
- Bibliography extraction from pasted text
- BibTeX format parsing
- Mixed-format input handling
- Resolver trait and registry pattern
- Site-specific resolver framework (Crossref first)

**Exit Criteria:** User can paste `10.1234/example` or `Smith (2024). Paper Title. Journal.` and get downloads.

**FRs:** FR-1.2, FR-1.3, FR-1.4, FR-1.5, FR-1.6, FR-2.3

---

### Epic 3: Reliable Batch Processing
**User Value:** "I can walk away and trust it works"

**Scope:**
- Progress display with indicatif (spinners, bars)
- In-place terminal updates (no scroll spam)
- Parsing feedback ("Found 47 items...")
- Completion summary with success/failure counts
- Graceful Ctrl+C handling
- Partial progress preservation
- Resumable downloads (Range requests)
- Queue persistence in SQLite

**Exit Criteria:** User can paste 50 references, walk away, and return to a clear summary.

**FRs:** FR-5.3, FR-5.4, FR-2.7
**UX:** UX-1, UX-2, UX-3, UX-8

---

### Epic 4: Authenticated Downloads
**User Value:** "I can download from my subscriptions"

**Scope:**
- Cookie capture from browser session
- Cookie storage (encrypted, opt-in keychain)
- Cookie file input (Netscape format, stdin)
- Auth-required site detection
- ScienceDirect resolver (first auth site)
- Session refresh workflow

**Exit Criteria:** User can download from sciencedirect.com after running `downloader auth capture`.

**FRs:** FR-2.2, FR-2.8
**ARCH:** ARCH-10

---

### Epic 5: Organized Output
**User Value:** "My downloads are organized automatically"

**Scope:**
- `--project` flag for folder creation
- Sub-project organization
- Metadata-based file naming (Author_Year_Title.ext)
- index.md generation per project
- File listing with metadata

**Exit Criteria:** Downloads land in organized project folders with meaningful filenames and index.

**FRs:** FR-3.1, FR-3.2, FR-3.3, FR-3.4, FR-5.2

---

### Epic 6: Download History
**User Value:** "I can see what I've downloaded"

**Scope:**
- Download attempt logging to SQLite
- Failure logging with actionable error info
- Per-project download.log file
- `downloader log` command for history queries

**Exit Criteria:** User can run `downloader log` and see past downloads with status.

**FRs:** FR-4.1, FR-4.2, FR-4.3

**Dependencies:** Epic 3 (for completion events), Epic 1 Story 1.4 (for SQLite schema)

---

### Epic 7: Professional CLI
**User Value:** "The tool feels polished and helpful"

**Scope:**
- stdin piped input support
- `--dry-run` for preview
- Configuration file (~/.config/downloader/config.toml)
- Helpful usage display when no input
- What/Why/Fix error message pattern
- Verbosity levels (--verbose, --quiet, --debug)
- Exit codes (0/1/2)
- Terminal width detection
- NO_COLOR support

**Exit Criteria:** CLI feels professional with helpful errors and appropriate output levels.

**FRs:** FR-5.1, FR-5.5, FR-5.6, FR-5.7
**UX:** UX-4, UX-5, UX-6, UX-7

---

### Epic 8: Polish & Enhancements
**User Value:** "Everything works even better"

**Scope:**
- Topic auto-detection via keyword extraction
- JSON-LD sidecar metadata files
- Parsing confidence tracking
- Query past downloads functionality
- Additional site-specific resolvers
- Performance optimizations

**Exit Criteria:** All Should-level requirements implemented.

**FRs:** FR-3.5, FR-3.6, FR-4.4, FR-4.5

---

## Epic 1: Download Any List

**Goal:** "I can paste URLs and get files"

**FRs Covered:** FR-1.1, FR-2.1, FR-2.4, FR-2.5, FR-2.6
**ARCH:** ARCH-1 through ARCH-9

### Story 1.0: Basic CLI Entry Point

As a **user**,
I want **to invoke the downloader command**,
So that **I can start using the tool**.

**Acceptance Criteria:**

**Given** the downloader binary is installed
**When** I run `downloader` or `echo "url" | downloader`
**Then** the CLI accepts input from arguments or stdin
**And** clap parses command-line arguments
**And** the main entry point initializes tracing
**And** the async Tokio runtime is started
**And** errors are reported via anyhow in the binary

---

### Story 1.1: Project Scaffolding

As a **contributor**,
I want **a properly structured Rust project with all dependencies configured**,
So that **I can implement features on a solid foundation**.

**Acceptance Criteria:**

**Given** a new Rust project
**When** I run `cargo build`
**Then** the project compiles without errors
**And** the lib/bin split is configured (src/lib.rs + src/main.rs)
**And** Cargo.toml includes: tokio, reqwest, sqlx, clap, tracing, thiserror, anyhow
**And** rustfmt.toml and clippy configuration are present
**And** `cargo clippy -- -D warnings` passes

---

### Story 1.2: HTTP Download Core

As a **user**,
I want **to download a file from a URL**,
So that **I can retrieve documents from the web**.

**Acceptance Criteria:**

**Given** a valid HTTP/HTTPS URL pointing to a file
**When** I pass the URL to the download function
**Then** the file is downloaded to the specified output directory
**And** the original filename is preserved (or derived from URL)
**And** the download streams to disk (not buffered in memory)
**And** errors are returned as structured Error types (not panics)

---

### Story 1.3: URL Input Detection

As a **user**,
I want **to paste URLs and have them automatically recognized**,
So that **I don't need to specify the input type**.

**Acceptance Criteria:**

**Given** text input containing URLs
**When** the parser processes the input
**Then** valid http:// and https:// URLs are extracted
**And** invalid URLs are reported with clear error messages
**And** non-URL text is ignored (for now)
**And** the parser returns a list of validated URL items

---

### Story 1.4: SQLite Queue Persistence

As a **user**,
I want **my download queue to persist across sessions**,
So that **interrupted downloads can resume**.

**Acceptance Criteria:**

**Given** a list of URLs to download
**When** the download process starts
**Then** items are stored in SQLite with status (pending/in_progress/complete/failed)
**And** WAL mode is enabled for concurrent reads
**And** the schema supports: id, url, status, created_at, updated_at, error_message
**And** migration files exist in migrations/ directory
**And** migrations run automatically on first use
**And** `cargo sqlx prepare` is run to enable compile-time query checking

---

### Story 1.5: Concurrent Downloads

As a **user**,
I want **multiple files to download simultaneously**,
So that **batch downloads complete faster**.

**Acceptance Criteria:**

**Given** items stored in SQLite queue (from Story 1.4)
**When** the download engine processes them
**Then** up to 10 downloads run concurrently (configurable default)
**And** a semaphore limits concurrent connections
**And** each download updates its status in the queue independently
**And** completion is tracked correctly for all items

---

### Story 1.6: Retry with Exponential Backoff

As a **user**,
I want **failed downloads to retry automatically**,
So that **transient errors don't require manual intervention**.

**Acceptance Criteria:**

**Given** a download that fails due to network error
**When** the retry logic activates
**Then** the download retries up to 3 times (configurable)
**And** delays increase exponentially (1s, 2s, 4s)
**And** permanent failures (404, 403) do not retry
**And** retry attempts are logged with reason

---

### Story 1.7: Per-Domain Rate Limiting

As a **user**,
I want **requests to respect site rate limits**,
So that **I don't get blocked by servers**.

**Acceptance Criteria:**

**Given** multiple URLs from the same domain
**When** downloads are processed
**Then** requests to the same domain are spaced (default 1 req/sec)
**And** different domains are processed in parallel without waiting
**And** rate limit settings can be overridden per domain
**And** rate limiting is logged at debug level

---

## Epic 2: Smart Reference Resolution

**Goal:** "I can paste DOIs and references"

**FRs Covered:** FR-1.2, FR-1.3, FR-1.4, FR-1.5, FR-1.6, FR-2.3

### Story 2.1: DOI Detection & Validation

As a **user**,
I want **DOIs to be automatically recognized in my input**,
So that **I can paste DOIs without special formatting**.

**Acceptance Criteria:**

**Given** text input containing DOIs
**When** the parser processes the input
**Then** DOI patterns (10.xxxx/...) are detected
**And** both bare DOIs and doi.org URLs are recognized
**And** invalid DOI formats are reported clearly
**And** DOIs are normalized to standard format (without URL prefix)

---

### Story 2.2: Resolver Trait & Registry

As a **developer**,
I want **an extensible resolver system**,
So that **new resolvers can be added without modifying core code**.

**Acceptance Criteria:**

**Given** the resolver module
**When** a new resolver is implemented
**Then** it implements the async `Resolver` trait with `can_resolve()` and `async fn resolve()` methods
**And** resolvers register in a priority-ordered registry
**And** the registry tries resolvers in order until one succeeds
**And** resolver failures fall through to next resolver gracefully
**And** trait uses `async_trait` macro for async method support

---

### Story 2.3: Crossref DOI Resolution

As a **user**,
I want **DOIs to resolve to downloadable URLs**,
So that **I can download papers by pasting their DOI**.

**Acceptance Criteria:**

**Given** a valid DOI
**When** the Crossref resolver processes it
**Then** the Crossref API is called to get metadata
**And** the PDF URL is extracted from the response (if available)
**And** metadata (title, authors, year) is captured for later use
**And** API failures return structured errors with retry hints
**And** rate limiting respects Crossref's polite pool guidelines

---

### Story 2.4: Reference String Parsing

As a **user**,
I want **to paste reference strings and have them recognized**,
So that **I can copy references directly from papers**.

**Acceptance Criteria:**

**Given** a reference string like "Smith, J. (2024). Paper Title. Journal Name, 1(2), 3-4."
**When** the parser processes it
**Then** author, year, and title are extracted
**And** partial matches are attempted (author + year, or title alone)
**And** parsing confidence is tracked (high/medium/low)
**And** unparseable references are flagged for manual review

---

### Story 2.5: Bibliography Extraction

As a **user**,
I want **to paste an entire bibliography and have all references extracted**,
So that **I can process reference lists in bulk**.

**Acceptance Criteria:**

**Given** multi-line text containing multiple references
**When** the parser processes it
**Then** individual references are separated correctly
**And** numbered lists (1. 2. 3.) are handled
**And** blank-line-separated references are handled
**And** each reference is parsed individually
**And** a summary shows: "Found X references (Y parsed, Z uncertain)"

---

### Story 2.6: BibTeX Format Support

As a **user**,
I want **to paste BibTeX entries**,
So that **I can use exports from reference managers**.

**Acceptance Criteria:**

**Given** BibTeX formatted input
**When** the parser processes it
**Then** @article, @book, @inproceedings entries are recognized
**And** DOI field is extracted if present
**And** Title/Author/Year fields are extracted for resolution
**And** Multiple BibTeX entries in one paste are handled
**And** Malformed BibTeX reports clear parsing errors

---

### Story 2.7: Mixed-Format Input Handling

As a **user**,
I want **to paste URLs, DOIs, and references together**,
So that **I don't need to separate my input by type**.

**Acceptance Criteria:**

**Given** input containing mixed formats (URLs + DOIs + references)
**When** the parser processes it
**Then** each item is classified by type (url/doi/reference/bibtex)
**And** items are processed by appropriate handlers
**And** parsing summary shows counts by type
**And** all items enter the same download queue regardless of source type

---

## Epic 3: Reliable Batch Processing

**Goal:** "I can walk away and trust it works"

**FRs Covered:** FR-5.3, FR-5.4, FR-2.7
**UX:** UX-1, UX-2, UX-3, UX-8

### Story 3.1: Input Parsing Feedback

As a **user**,
I want **immediate feedback on what was parsed from my input**,
So that **I trust the tool understood me correctly**.

**Acceptance Criteria:**

**Given** input containing various item types
**When** parsing completes
**Then** a summary is displayed: "Parsed X items: Y URLs, Z DOIs, W references"
**And** uncertain parses are flagged: "(3 references need verification)"
**And** the summary appears before downloading begins
**And** output respects terminal width constraints

---

### Story 3.2: Progress Spinner Display

As a **user**,
I want **to see active progress during downloads**,
So that **I know the tool is working**.

**Acceptance Criteria:**

**Given** downloads in progress
**When** the terminal is displayed
**Then** an animated spinner shows activity
**And** the current item count is shown: "[12/47]"
**And** the current domain is shown: "Downloading from sciencedirect.com..."
**And** indicatif library is used for spinner rendering

---

### Story 3.3: In-Place Status Updates

As a **user**,
I want **progress updates without terminal scroll spam**,
So that **I can see status clearly**.

**Acceptance Criteria:**

**Given** multiple downloads completing
**When** status changes
**Then** the status line updates in-place (carriage return)
**And** completed items don't produce individual lines (in default mode)
**And** only errors produce persistent output lines
**And** terminal supports is detected; falls back to line-by-line if needed

---

### Story 3.4: Completion Summary

As a **user**,
I want **a clear summary when all downloads finish**,
So that **I know what succeeded and what needs attention**.

**Acceptance Criteria:**

**Given** a batch download completes
**When** the summary is displayed
**Then** success count is shown: "✓ 44/47 downloaded successfully"
**And** output location is shown: "Organized to /Projects/Research/"
**And** failures are grouped by type (auth required, not found, etc.)
**And** each failure type shows actionable next step
**And** the summary is visually distinct (box/separator)

---

### Story 3.5: Graceful Interrupt Handling

As a **user**,
I want **Ctrl+C to stop cleanly without losing progress**,
So that **I can safely interrupt long batches**.

**Acceptance Criteria:**

**Given** a batch download in progress
**When** user presses Ctrl+C
**Then** current downloads complete (or timeout after 5s)
**And** queue state is preserved in SQLite
**And** partial progress summary is displayed
**And** exit code reflects partial completion (1)
**And** "Interrupted. X/Y completed. Run again to resume." message shown

---

### Story 3.6: Resumable Downloads

As a **user**,
I want **large file downloads to resume if interrupted**,
So that **I don't re-download gigabytes on failure**.

**Acceptance Criteria:**

**Given** a partially downloaded file exists
**When** the download resumes
**Then** HTTP Range header requests remaining bytes
**And** server support is detected (Accept-Ranges response)
**And** non-supporting servers restart from beginning
**And** partial download state (bytes_downloaded, content_length) stored in queue schema
**And** file integrity is verified after resume (Content-Length match)
**And** resume attempts are logged

---

## Epic 4: Authenticated Downloads

**Goal:** "I can download from my subscriptions"

**FRs Covered:** FR-2.2, FR-2.8
**ARCH:** ARCH-10

### Story 4.1: Auth-Required Detection

As a **user**,
I want **to know when a download failed due to authentication**,
So that **I can take action to provide credentials**.

**Acceptance Criteria:**

**Given** a download attempt to an authenticated site
**When** the server returns 401/403 or login redirect
**Then** the failure is classified as "auth_required"
**And** the domain requiring auth is captured
**And** error message suggests: "Run `downloader auth capture` to authenticate"
**And** auth failures are grouped separately in completion summary

---

### Story 4.2: Cookie File Input

As a **user**,
I want **to provide cookies from a file**,
So that **I can use cookies exported from browser extensions**.

**Acceptance Criteria:**

**Given** a Netscape-format cookie file
**When** I run `downloader --cookies cookies.txt`
**Then** cookies are loaded and applied to requests
**And** `--cookies -` reads cookies from stdin (distinct from URL stdin input)
**And** invalid cookie format produces clear error
**And** cookies are matched to domains correctly
**And** sensitive cookie values are never logged

---

### Story 4.3: Browser Cookie Capture

As a **user**,
I want **to capture cookies from my browser session**,
So that **I can use my existing institutional login**.

**Acceptance Criteria:**

**Given** I run `downloader auth capture`
**When** the capture process runs
**Then** step-by-step instructions are displayed:
  1. "Install a cookie export extension (e.g., 'Get cookies.txt LOCALLY')"
  2. "Log into the site you want to download from"
  3. "Export cookies to Netscape format"
  4. "Paste the cookie file path or pipe contents"
**And** user is prompted to paste/provide cookie data
**And** common browser extension formats are supported (Netscape, JSON)
**And** captured cookies are validated (not expired, has required fields)
**And** success message confirms: "Cookies captured for X domains"

---

### Story 4.4: Secure Cookie Storage

As a **user**,
I want **my cookies stored securely (opt-in)**,
So that **I don't need to re-capture for every session**.

**Acceptance Criteria:**

**Given** captured cookies and `--save-cookies` flag
**When** cookies are persisted
**Then** cookies are encrypted at rest
**And** encryption key uses system keychain (macOS Keychain, etc.)
**And** storage location is ~/.config/downloader/cookies.enc
**And** `downloader auth clear` removes stored cookies
**And** cookie persistence is opt-in, not default

---

### Story 4.5: ScienceDirect Resolver

As a **user**,
I want **to download papers from ScienceDirect**,
So that **I can access my institutional subscriptions**.

**Acceptance Criteria:**

**Given** a ScienceDirect URL or DOI
**When** the resolver processes it with valid cookies
**Then** the PDF download URL is resolved
**And** authentication cookies are applied to the request
**And** article metadata is extracted from the page
**And** common ScienceDirect URL patterns are recognized
**And** failures suggest cookie refresh if auth appears expired

---

## Epic 5: Organized Output

**Goal:** "My downloads are organized automatically"

**FRs Covered:** FR-3.1, FR-3.2, FR-3.3, FR-3.4, FR-5.2

### Story 5.1: Project Folder Creation

As a **user**,
I want **to organize downloads into a project folder**,
So that **my files are grouped by research topic**.

**Acceptance Criteria:**

**Given** the `--project "Climate Research"` flag
**When** downloads complete
**Then** a folder "Climate-Research" is created (sanitized name)
**And** all downloaded files are placed in the project folder
**And** the folder location is shown in completion summary
**And** existing folders are reused (not duplicated)
**And** default output location is configurable

---

### Story 5.2: Sub-Project Organization

As a **user**,
I want **to create nested project structures**,
So that **I can organize large research efforts**.

**Acceptance Criteria:**

**Given** the `--project "Climate/Emissions/2024"` flag
**When** downloads complete
**Then** nested folders are created: Climate/Emissions/2024/
**And** parent folders are created if they don't exist
**And** path separators work cross-platform (/ on all systems)
**And** deeply nested paths are supported (up to reasonable limit)

---

### Story 5.3: Metadata-Based File Naming

As a **user**,
I want **downloaded files named with author, year, and title**,
So that **I can identify files without opening them**.

**Acceptance Criteria:**

**Given** a downloaded paper with metadata
**When** the file is saved
**Then** filename follows pattern: Author_Year_Title.ext
**And** long titles are truncated (max 60 chars)
**And** special characters are sanitized for filesystem safety
**And** duplicate names get numeric suffix (Author_Year_Title_2.pdf)
**And** missing metadata falls back to: domain_timestamp.ext

---

### Story 5.4: Project Index Generation

As a **user**,
I want **an index.md file listing all downloads**,
So that **I can see what's in each project at a glance**.

**Acceptance Criteria:**

**Given** a project folder with downloads
**When** the batch completes
**Then** index.md is created/updated in the project folder
**And** each file is listed with: filename, title, authors, source URL
**And** files are grouped by download session (with timestamp)
**And** markdown formatting renders nicely in editors/GitHub
**And** existing index.md entries are preserved (append mode)

---

## Epic 6: Download History

**Goal:** "I can see what I've downloaded"

**FRs Covered:** FR-4.1, FR-4.2, FR-4.3

### Story 6.1: Download Attempt Logging

As a **user**,
I want **all download attempts logged**,
So that **I have a record of what was downloaded and when**.

**Acceptance Criteria:**

**Given** any download attempt (success or failure)
**When** the attempt completes
**Then** a record is stored in SQLite with: url, status, timestamp, file_path
**And** metadata is stored: title, authors, doi (if available)
**And** records are queryable by date range, status, project
**And** logging does not slow down downloads noticeably

---

### Story 6.2: Failure Logging with Details

As a **user**,
I want **failures logged with actionable information**,
So that **I can diagnose and retry failed downloads**.

**Acceptance Criteria:**

**Given** a failed download
**When** the failure is logged
**Then** error type is categorized (network, auth, not_found, parse_error)
**And** HTTP status code is captured (if applicable)
**And** error message includes suggestion for resolution
**And** retry count and last retry timestamp are tracked
**And** original input (URL/DOI/reference) is preserved

---

### Story 6.3: Per-Project Download Log

As a **user**,
I want **a download.log file in each project folder**,
So that **I can see the history for that specific project**.

**Acceptance Criteria:**

**Given** a project folder
**When** downloads complete
**Then** download.log is created/appended in the project folder
**And** log format is human-readable (not JSON)
**And** each entry shows: timestamp, status, filename, source
**And** failures are clearly marked with error reason
**And** log file doesn't duplicate SQLite data (references it)

---

### Story 6.4: History Query Command

As a **user**,
I want **to query my download history**,
So that **I can find past downloads and check their status**.

**Acceptance Criteria:**

**Given** the `downloader log` command
**When** I run it with optional filters
**Then** recent downloads are listed (default: last 50)
**And** filters supported: --project, --status, --since, --domain
**And** output shows: date, status, title/filename, source
**And** `downloader log --failed` shows only failures with fix suggestions
**And** output is formatted for terminal width

---

## Epic 7: Professional CLI

**Goal:** "The tool feels polished and helpful"

**FRs Covered:** FR-5.1, FR-5.5, FR-5.6, FR-5.7
**UX:** UX-4, UX-5, UX-6, UX-7

### Story 7.1: Stdin Piped Input

As a **user**,
I want **to pipe input from other commands**,
So that **I can integrate with shell workflows**.

**Acceptance Criteria:**

**Given** piped input like `cat refs.txt | downloader`
**When** the tool reads stdin
**Then** input is read completely before processing begins
**And** stdin detection works (isatty check)
**And** combining stdin with file arguments works
**And** empty stdin produces helpful message, not error

---

### Story 7.2: Dry Run Mode

As a **user**,
I want **to preview what would be downloaded**,
So that **I can verify parsing before committing**.

**Acceptance Criteria:**

**Given** the `--dry-run` or `-n` flag
**When** I run the command
**Then** input is parsed and displayed
**And** resolved URLs are shown (DOIs resolved, references matched)
**And** no files are downloaded
**And** no database records are created
**And** output clearly states: "Dry run - no files downloaded"

---

### Story 7.3: Configuration File

As a **user**,
I want **to set default options in a config file**,
So that **I don't repeat common flags**.

**Acceptance Criteria:**

**Given** a config file at ~/.config/downloader/config.toml
**When** the tool runs
**Then** config values are loaded as defaults
**And** XDG_CONFIG_HOME is respected on Linux ($XDG_CONFIG_HOME/downloader/config.toml)
**And** CLI flags override config values
**And** supported settings: output_dir, concurrency, rate_limit, verbosity
**And** `downloader config show` displays current effective config
**And** missing config file is not an error (use built-in defaults)

---

### Story 7.4: No-Input Help Display

As a **user**,
I want **helpful guidance when I run the tool with no input**,
So that **I can learn how to use it**.

**Acceptance Criteria:**

**Given** running `downloader` with no arguments and no stdin
**When** the tool starts
**Then** a friendly usage message is displayed (not error)
**And** common examples are shown
**And** the message fits in 80-char terminal width
**And** `--help` shows full help (this shows quick start)

---

### Story 7.5: What/Why/Fix Error Pattern

As a **user**,
I want **error messages that tell me what to do**,
So that **I can resolve problems without searching**.

**Acceptance Criteria:**

**Given** any error condition
**When** the error is displayed
**Then** format follows: What happened → Why it might have happened → How to fix
**And** errors are categorized with appropriate icons (🔐, ❌, 🌐, ⚠️)
**And** fix suggestions are specific and actionable
**And** error grouping in summary shows count per category

---

### Story 7.6: Verbosity Levels

As a **user**,
I want **control over output verbosity**,
So that **I can get more or less detail as needed**.

**Acceptance Criteria:**

**Given** verbosity flags
**When** running the tool
**Then** default shows: status line + completion summary
**And** `--verbose` or `-v` shows: per-item progress
**And** `--quiet` or `-q` shows: summary only (for scripts)
**And** `--debug` shows: full tracing output
**And** verbosity levels are mutually exclusive

---

### Story 7.7: Exit Codes

As a **user**,
I want **standard exit codes**,
So that **I can use the tool in scripts**.

**Acceptance Criteria:**

**Given** the tool completes
**When** the process exits
**Then** exit code 0 means: all items succeeded
**And** exit code 1 means: partial success (some failed)
**And** exit code 2 means: complete failure (none succeeded or fatal error)
**And** exit codes are documented in --help

---

### Story 7.8: Terminal Compatibility

As a **user**,
I want **the tool to work in any terminal**,
So that **output is readable everywhere**.

**Acceptance Criteria:**

**Given** various terminal environments
**When** the tool runs
**Then** terminal width is detected and output truncated appropriately
**And** `NO_COLOR` env var disables color output
**And** `--no-color` flag disables color output
**And** dumb terminals get plain text (no ANSI escape codes)
**And** spinners fall back to text in non-interactive terminals

---

## Epic 8: Polish & Enhancements

**Goal:** "Everything works even better"

**FRs Covered:** FR-3.5, FR-3.6, FR-4.4, FR-4.5

### Story 8.1: Topic Auto-Detection

As a **user**,
I want **downloads automatically tagged with topics**,
So that **I can see themes in my collection**.

**Acceptance Criteria:**

**Given** downloaded papers with metadata
**When** topic detection runs
**Then** keywords are extracted from titles and abstracts
**And** common academic topics are recognized
**And** topics are added to index.md: "(12 topics detected)"
**And** topic detection is optional (--detect-topics flag)
**And** custom topic lists can be provided

---

### Story 8.2: JSON-LD Sidecar Files

As a **user**,
I want **machine-readable metadata alongside downloads**,
So that **other tools can process my collection**.

**Acceptance Criteria:**

**Given** a downloaded file with metadata
**When** sidecar generation is enabled
**Then** a .json file is created alongside: paper.pdf → paper.json
**And** JSON-LD format follows Schema.org/ScholarlyArticle
**And** fields include: title, authors, datePublished, doi, sourceUrl
**And** sidecar generation is optional (--sidecar flag)
**And** existing sidecars are not overwritten

---

### Story 8.3: Parsing Confidence Tracking

As a **user**,
I want **to know which references were uncertain**,
So that **I can verify them manually**.

**Acceptance Criteria:**

**Given** references parsed with varying confidence
**When** the summary is displayed
**Then** confidence levels are tracked: high/medium/low
**And** uncertain items are flagged in completion summary
**And** `downloader log --uncertain` lists items needing review
**And** confidence is stored in database for later query
**And** confidence factors are logged at debug level

---

### Story 8.4: Query Past Downloads

As a **user**,
I want **to search my download history**,
So that **I can find papers I've downloaded before**.

**Acceptance Criteria:**

**Given** `downloader search <query>`
**When** I search with keywords
**Then** titles, authors, and DOIs are searched
**And** results show: title, date downloaded, file path
**And** results can be filtered by project, date range
**And** `--open` flag opens the file directly
**And** fuzzy matching helps with typos

---

### Story 8.5: Additional Site Resolvers

As a **user**,
I want **more academic sites supported**,
So that **I can download from various sources**.

**Acceptance Criteria:**

**Given** URLs from common academic sites
**When** the resolver processes them
**Then** additional resolvers are available: arXiv, PubMed, IEEE, Springer
**And** each resolver follows the Resolver trait pattern
**And** resolvers are prioritized by specificity
**And** resolver documentation lists supported sites
**And** community resolver contributions are easy to add

