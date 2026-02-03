---
stepsCompleted: [1, 2, 3, 'cli-condensed']
scope: 'cli-focused-mvp'
inputDocuments:
  - "_bmad-output/planning-artifacts/product-brief-Downloader-2026-01-20.md"
  - "_bmad-output/planning-artifacts/prd.md"
  - "_bmad-output/planning-artifacts/architecture.md"
  - "_bmad-output/project-context.md"
workflowType: 'ux-design'
project_name: 'Downloader'
user_name: 'fierce'
date: '2026-01-27'
---

# UX Design Specification: Downloader

**Author:** fierce
**Date:** 2026-01-27

---

## Executive Summary

### Project Vision

Downloader is an information ingestion engine that transforms curated lists (URLs, DOIs, bibliographies) into organized, searchable, LLM-ready knowledge. The core promise is **"Trust that your knowledge is captured."**

**Brand Promise:** Capture. Organize. Trust.

**Interface Strategy:** CLI-first for MVP. GUI planned for v2 with "90s aesthetic."

### Target Users

| User Type | Profile | Current Pain |
|-----------|---------|--------------|
| Power Users / Data Hoarders | Download at scale, curate collections | Manual one-by-one downloads, lose track, organization never happens |
| Researchers | Build project-based knowledge bases | Fragmented workflow, LLM processing blocked by disorganized inputs |

**The "Aha" Moment:** User pastes 47 references, walks away, returns to find everything downloaded, organized, and indexed. "I'm never doing this manually again."

### Key Design Challenges

1. **Progress Communication** - How to show meaningful progress for batch operations (32/47 downloading, 3 queued, 2 failed) without overwhelming the terminal
2. **Error Presentation** - Failures shouldn't feel like failures; they're logged, actionable, non-blocking. UX must communicate "it's handled"
3. **Trust Building** - Zero-config start means users need immediate confidence the tool understood their input correctly
4. **Completion Summary** - The final output is the payoff moment. Must feel satisfying and complete.

### Design Opportunities

1. **Semantic Progress** - Not just counts, but meaningful status ("Resolving DOIs...", "Downloading from ScienceDirect...")
2. **Quiet Confidence** - Minimal output during success, detailed output only when needed
3. **"Walk Away" UX** - Design for users who won't watch the terminal. Completion summary is king.

## Core User Experience

### Defining Experience

**Core Loop:** Paste → Project → Walk Away → Return to Knowledge

The fundamental interaction is batch processing with minimal attention required. Users don't want to watch downloads—they want results.

### Platform Strategy

**Primary Platform:** CLI (Terminal)
- Keyboard-driven, no mouse required
- Pipe-friendly for scripting (`cat refs.txt | downloader --project "Research"`)
- Works over SSH for remote research workflows
- No GUI dependencies for MVP

**Input Methods:**
- stdin (piped input)
- Direct arguments
- File references (`--input refs.bib`)

### Effortless Interactions

| Interaction | Implementation |
|-------------|----------------|
| Input parsing | Auto-detect format (URL/DOI/reference/BibTeX) |
| Project setup | `--project "Name"` creates folder structure |
| Auth sites | Capture cookies from browser session |
| Failures | Log and continue, never block queue |

### Critical Success Moments

1. **Parsing Confirmation** - "Found 47 references (32 URLs, 12 DOIs, 3 references)"
2. **Progress Assurance** - Status line shows work is happening
3. **Completion Summary** - Clear success/failure counts with organized output

### Experience Principles

1. **Trust Over Transparency** - Communicate confidence, not implementation details
2. **Failures Are Data** - Logged for later action, never blocking
3. **Output Is The Product** - The organized folder matters, not the terminal output
4. **Quiet When Right, Clear When Wrong** - Minimal success noise, actionable failure info

### Input Feedback Pattern

Immediate parsing echo builds trust:
```
Parsed 47 items:
  32 URLs (direct)
  12 DOIs (will resolve)
   3 references (best-effort)
```

### Progress Design

- Spinners for active work (terminal should feel alive)
- In-place updates for counts (no scrolling spam)
- Status line: `[32/47] Downloading from sciencedirect.com...`

### Verbosity Levels

| Flag | Output |
|------|--------|
| (default) | Status line + completion summary |
| `--verbose` | Per-item progress |
| `--quiet` | Summary only (scriptable) |
| `--debug` | Full tracing |

### Completion Summary Design

```
✓ 44/47 downloaded successfully
✓ Organized to /Projects/Climate-Research/
✓ Index generated: index.md (12 topics)
⚠ 3 items need attention (see below)

  • sciencedirect.com/... → Run: downloader auth capture
  • example.com/paper.pdf → 404 Not Found
  • 10.1234/broken → DOI not found
```

### Interrupt & Recovery

- Ctrl+C gracefully stops, shows partial progress
- `downloader status` recalls last run summary
- Exit codes: 0 (success), 1 (partial), 2 (failure)

### Terminal Compatibility

- Detect terminal width, truncate gracefully
- Support `--no-color` and `NO_COLOR` env var
- Test in plain/dumb terminal mode

## CLI UX Patterns (Condensed)

### Error Message UX

**Structure:** Every error follows What → Why → Fix pattern.

```
Error: Authentication required for sciencedirect.com
       Your browser session may have expired.
       Fix: Run `downloader auth capture` to refresh credentials
```

**Error Categories & Tone:**

| Category | Icon | Tone | Example |
|----------|------|------|---------|
| Auth Required | 🔐 | Helpful | "Run `downloader auth capture`" |
| Not Found | ❌ | Factual | "404 - URL may have moved or been removed" |
| Network | 🌐 | Patient | "Connection timeout - will retry automatically" |
| Parse | ⚠️ | Informative | "Could not parse as DOI, treating as URL" |
| Rate Limited | ⏳ | Reassuring | "Rate limited by site - waiting 30s" |

**Error Grouping in Summary:**

```
⚠ 5 items need attention:

  Auth Required (2):
    • sciencedirect.com/article/123
    • nature.com/papers/456
    → Run: downloader auth capture

  Not Found (2):
    • example.com/removed.pdf
    • broken-link.org/paper
    → Verify URLs are still valid

  Parse Failed (1):
    • "Smith et al 2024" - insufficient metadata
    → Add DOI or direct URL
```

### CLI Command Structure UX

**Command Hierarchy:**

```
downloader                     # Default: read stdin, download to current dir
downloader download <input>    # Explicit download command
downloader project <cmd>       # Project management
downloader auth <cmd>          # Authentication management
downloader log <cmd>           # History and logging
downloader config <cmd>        # Configuration
```

**Flag Conventions:**

| Pattern | Example | Rationale |
|---------|---------|-----------|
| Short + Long | `-p, --project` | Muscle memory + discoverability |
| Verbs for actions | `--retry`, `--skip` | Clear intent |
| Nouns for targets | `--output`, `--input` | Clear destination |
| Booleans obvious | `--dry-run`, `--verbose` | No value needed |

**Help Text UX:**

```
downloader - Batch download and organize reference documents

USAGE:
    downloader [OPTIONS] [INPUT]
    cat refs.txt | downloader --project "Research"

EXAMPLES:
    downloader --project "Climate" urls.txt
    downloader --dry-run bibliography.bib
    echo "10.1234/paper" | downloader

OPTIONS:
    -p, --project <NAME>    Organize into project folder
    -o, --output <DIR>      Output directory [default: ./]
    -n, --dry-run           Preview without downloading
    -v, --verbose           Show per-item progress
    -q, --quiet             Summary only (for scripts)
    -h, --help              Show this help
```

### Output Formatting Guidelines

**Progress States:**

```
# Parsing (brief)
Parsing input... 47 items found

# Active download (single line, updates in place)
⠋ [12/47] Downloading from sciencedirect.com...

# Completion (expanded)
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
✓ Download Complete

  44 succeeded
   3 need attention (see above)

  Output: /Projects/Climate-Research/
  Index:  index.md (12 topics detected)
  Log:    download.log
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
```

**Color Usage:**

| Element | Color | Fallback |
|---------|-------|----------|
| Success | Green | ✓ prefix |
| Warning | Yellow | ⚠ prefix |
| Error | Red | ✗ prefix |
| Info | Cyan | • prefix |
| Muted | Gray | (parentheses) |

**Width Handling:**

```
# Wide terminal (>100 chars)
[12/47] Downloading: https://sciencedirect.com/science/article/pii/S0140...

# Narrow terminal (<80 chars)
[12/47] sciencedirect.com/.../S0140...

# Minimum (60 chars)
[12/47] Downloading...
```

### Emotional Response (CLI Context)

**Target Feelings by Phase:**

| Phase | User Should Feel | How We Achieve It |
|-------|------------------|-------------------|
| Input | "It understood me" | Immediate parsing feedback with counts |
| Progress | "It's working" | Alive spinner, site names, count updates |
| Waiting | "I can walk away" | No required interaction mid-process |
| Completion | "That was easy" | Clean summary, organized output |
| Errors | "I know what to do" | Actionable suggestions, grouped issues |

**Anti-Patterns to Avoid:**

- Wall of text during progress
- Cryptic error codes without explanation
- Requiring user input mid-batch
- Silent failures (always log, always report)
- Ambiguous success ("Done" without details)

## Implementation Notes

### indicatif Integration

```rust
// Progress bar style
let style = ProgressStyle::default_bar()
    .template("{spinner:.cyan} [{pos}/{len}] {msg}")
    .progress_chars("━━─");

// Multi-progress for verbose mode
let multi = MultiProgress::new();
```

### Color Support Check

```rust
// Respect NO_COLOR and --no-color
let use_color = !env::var("NO_COLOR").is_ok()
    && !args.no_color
    && atty::is(atty::Stream::Stdout);
```

### Terminal Width

```rust
// Graceful width detection
let width = terminal_size::terminal_size()
    .map(|(w, _)| w.0 as usize)
    .unwrap_or(80);
```

---

## UX Design Status

**Scope:** CLI-focused MVP (GUI deferred to v2)

**Completed:**
- Executive Summary & Target Users
- Core User Experience & Principles
- Progress & Completion Design
- Error Message Patterns
- CLI Command Structure
- Output Formatting Guidelines
- Implementation Notes

**Deferred to v2 (GUI):**
- Visual Design System
- Component Strategy
- Responsive/Accessibility (beyond CLI)
- Design Directions & Mood Boards

**Document Status:** CLI UX Complete - Ready for Implementation

