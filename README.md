# Downloader

Batch download and organize reference documents.

Downloader transforms curated lists of sources (URLs, DOIs, bibliographies)
into organized, searchable, LLM-ready knowledge.

## Quick Start

```bash
# Download a single URL
echo "https://example.com/paper.pdf" | downloader

# Download multiple URLs
echo "https://a.com/1.pdf
https://b.com/2.pdf
https://c.com/3.pdf" | downloader

# Pass URLs as arguments
downloader https://example.com/paper.pdf https://other.com/doc.pdf

# Flags can appear after URLs
downloader https://example.com/paper.pdf -q

# Pipe from a file
cat urls.txt | downloader

# Download to a specific directory
downloader -o ./downloads https://example.com/file.pdf

# Organize test outputs
mkdir -p test_outputs
downloader -o test_outputs urls.txt
```

## Options

| Flag | Short | Description | Default |
|------|-------|-------------|---------|
| `--output-dir` | `-o` | Output directory for downloaded files | current directory |
| `--concurrency` | `-c` | Max concurrent downloads (1-100) | 10 |
| `--max-retries` | `-r` | Max retry attempts for transient failures (0-10) | 3 |
| `--rate-limit` | `-l` | Min delay between requests to same domain in ms (0 to disable) | 1000 |
| `--verbose` | `-v` | Increase verbosity (`-v` debug, `-vv` trace) | info |
| `--quiet` | `-q` | Suppress non-error output | off |

Flags may appear before or after positional URLs. Use `--` to pass a URL literal that starts with `-`.

## Supported Resolvers

Resolver dispatch is priority-ordered (`Specialized` before `General` before `Fallback`) and deterministic.

| Resolver | Accepted inputs | Resolution behavior | Auth behavior |
|---|---|---|---|
| `arxiv` | `https://arxiv.org/abs/<id>`, `https://arxiv.org/pdf/<id>.pdf`, `10.48550/arXiv.*` | Normalizes to canonical `https://arxiv.org/pdf/<id>.pdf` | Open-access; no auth flow expected |
| `pubmed` | `https://pubmed.ncbi.nlm.nih.gov/<pmid>/`, `https://pmc.ncbi.nlm.nih.gov/articles/PMC*` | Resolves PubMed records through PMC full-text links to a PDF target | Returns structured failure when no PMC full text is available |
| `ieee` | `https://ieeexplore.ieee.org/document/<id>/`, `10.1109/*`, DOI URLs for `10.1109/*` | Extracts/normalizes IEEE stamp PDF URL from document metadata | Returns `NeedsAuth` for likely paywall/sign-in responses |
| `springer` | `https://link.springer.com/article/10.1007/*`, `https://link.springer.com/chapter/10.1007/*`, `10.1007/*` | Extracts canonical `/content/pdf/<doi>.pdf` URL from metadata with deterministic fallback | Returns `NeedsAuth` for paywall/subscription signals |
| `sciencedirect` | `https://www.sciencedirect.com/science/article/*`, `10.1016/*`, DOI URLs for `10.1016/*` | Extracts ScienceDirect PDF endpoint and metadata from article page | Returns `NeedsAuth` when auth/session is required |
| `crossref` | DOI input (`InputType::Doi`) | Resolves DOI metadata via Crossref; may redirect to `doi.org` fallback | N/A |
| `direct` | Direct URL input (`InputType::Url`) | Pass-through fallback resolver | N/A |

### Resolver Metadata Contract

Site resolvers should populate normalized metadata keys when available:

- `title`
- `authors`
- `doi`
- `year`
- `source_url`

## New Resolver Checklist

When adding a new site resolver:

1. Add a dedicated module under `src/resolver/` implementing `Resolver` (`name`, `priority`, `can_handle`, `resolve`).
2. Use shared resolver HTTP client policy (`src/resolver/http_client.rs`) and keep requests panic-free.
3. Register the resolver in `build_default_resolver_registry` (`src/resolver/mod.rs`) so CLI and dry-run stay in sync.
4. Add focused unit tests for matching/normalization and integration tests in `tests/resolver_integration.rs` for success + negative/auth paths.
5. Verify deterministic priority behavior against overlapping DOI/URL patterns.
6. Update this README section with supported inputs and auth expectations.

## Known Limitations

**Bot Detection & WAF Blocking**

Some URLs may fail with HTTP 403 despite the downloader's bot-detection handling:

- The tool sends a default User-Agent identifying the tool (e.g. `downloader/0.1.0 (academic-research-tool; +https://github.com/...)`) so requests look legitimate and respectful
- On 403 errors, it retries once with a browser-like User-Agent as a last resort before giving up
- This recovers most bot-detection blocks (tested with 38 URLs, 4 of 5 failures recovered)

However, sites using enterprise WAF/CDN protection (Akamai, Cloudflare Enterprise, etc.) may block based on:
- IP reputation, geolocation, or rate-limiting policies
- TLS fingerprinting or behavioral analysis
- Site-wide access restrictions

These blocks occur at the edge layer before the request reaches the origin server, so User-Agent spoofing alone cannot bypass them. This is expected behavior for sites with strict programmatic access controls.

## Building

```bash
cargo build --release
```

The binary will be at `target/release/downloader`.

## Testing

```bash
cargo test
cargo clippy -- -D warnings
```

### Flaky Test Stress Lane

Run the known sidecar flaky test repeatedly with a lean stress runner:

```bash
# Default: 12 iterations
cargo run --bin stress-sidecar-flaky

# Override iterations via env var
STRESS_ITERATIONS=25 cargo run --bin stress-sidecar-flaky

# Override iterations via CLI arg
cargo run --bin stress-sidecar-flaky -- --iterations 25
```

Logs are written to `target/stress-logs/sidecar-flaky/`:
- `iteration_XXX.log` per attempt
- `summary.txt` run summary
- `failed_iteration.txt` when a failure occurs

CI workflow: `.github/workflows/stress-sidecar-flaky.yml`
- non-PR signal lane (`workflow_dispatch` + nightly schedule)
- default `12` iterations, `15` minute timeout
- uploads stress logs as artifacts on failure

## Extract URLs From Markdown

Create a `urls.txt`-style file (one URL per line) from markdown files:

```bash
# Single markdown file
cargo run --bin extract-md-links -- README.md -o urls.txt

# Directory (recursive)
cargo run --bin extract-md-links -- ./external_reports -o urls.txt

# Multiple paths
cargo run --bin extract-md-links -- notes.md docs/ -o urls.txt
```

By default URLs are deduplicated. Use `--keep-duplicates` to preserve duplicates.

## License

MIT
