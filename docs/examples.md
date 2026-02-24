# Downloader — Practical Examples

End-to-end workflows for common use cases. Each section shows a realistic scenario and the exact commands to run it.

---

## 1. URL List File

The simplest workflow: a plain text file with one URL per line. The file `url_lists/qa-automation-links.txt` is a real example included in this repo — 38 links covering AI-augmented QA tools and trends.

**Step 1: Validate before you commit**

Run with `--dry-run` first. The tool parses and resolves every input without downloading anything or writing database records.

```bash
downloader --dry-run < url_lists/qa-automation-links.txt
```

**Step 2: Download for real**

Pipe the file in and organise everything under a named project folder inside a target directory.

```bash
downloader \
  --output-dir ~/research \
  --project "QA Automation Survey" \
  < url_lists/qa-automation-links.txt
```

Downloaded files land in `~/research/QA Automation Survey/`. A SQLite history database is created at `~/research/.downloader/queue.db`.

**Step 3: Review what happened**

```bash
# See the last 50 attempts for this project
downloader log --output-dir ~/research --project "QA Automation Survey"

# Show only failures
downloader log --output-dir ~/research --project "QA Automation Survey" --failed
```

---

## 2. Research Papers via DOI and Mixed References

You have a mixed file of DOIs, formatted citations, and URLs — the kind of list that accumulates while reading. Pass it straight in; the parser handles all three formats automatically.

**Example input file (`refs.txt`)**

```
10.1145/3597503.3623308
Smith J, Jones A. 2024. Attention is all you need. Nature 123:45–67.
https://arxiv.org/abs/2301.07041
10.1038/s41586-023-06597-1
https://pmc.ncbi.nlm.nih.gov/articles/PMC10789012/
```

**Download with full metadata enrichment**

```bash
downloader \
  --project "LLM Reading List" \
  --output-dir ~/papers \
  --detect-topics \
  --sidecar \
  < refs.txt
```

- `--detect-topics` auto-tags each downloaded file based on its title and abstract.
- `--sidecar` writes a `.json` file alongside each PDF with structured metadata (title, authors, DOI, year, topics).

**Use a custom topics taxonomy**

```bash
downloader \
  --project "LLM Reading List" \
  --output-dir ~/papers \
  --detect-topics \
  --topics-file my-topics.txt \
  --sidecar \
  < refs.txt
```

`my-topics.txt` contains one topic label per line; matched topics are ranked first in the sidecar output.

**BibTeX from Zotero / Mendeley**

Export your library as a `.bib` file and pipe it straight in — the BibTeX parser extracts DOIs and URLs automatically.

```bash
downloader \
  --project "Thesis Bibliography" \
  --output-dir ~/thesis/papers \
  --detect-topics --sidecar \
  < library.bib
```

**Check for low-confidence parses**

Reference strings the parser couldn't resolve with high confidence are flagged:

```bash
downloader log --output-dir ~/papers --uncertain
```

Review these rows and add the correct DOI or URL to re-download them.

---

## 3. Authenticated Downloads (Institutional / Subscription Sites)

IEEE, Springer, and ScienceDirect require a valid session. Capture your browser cookies once and the downloader reuses them for every subsequent run.

**Step 1: Export cookies from your browser**

Use a browser extension (e.g. "Get cookies.txt LOCALLY") to export a Netscape-format cookie file while logged in to the publisher site.

**Step 2: Capture and persist cookies**

```bash
downloader auth capture --save-cookies < exported-cookies.txt
```

Cookies are encrypted and stored on disk. All future runs use them automatically — no need to pass `--cookies` each time.

**Step 3: Download the paywalled papers**

```bash
downloader \
  --project "IEEE Signal Processing 2025" \
  --output-dir ~/papers \
  --respectful \
  < ieee-dois.txt
```

`--respectful` sets conservative defaults (concurrency 2, rate-limit 3 s, robots.txt checked) — a good idea on publisher sites that have strict crawl policies.

**Step 4: Find auth failures**

```bash
downloader log \
  --output-dir ~/papers \
  --failed \
  --domain ieeexplore.ieee.org
```

If cookies have expired, re-run `auth capture` and retry the failed items. To clear stored cookies:

```bash
downloader auth clear
```

**One-off cookie file (without persisting)**

```bash
downloader \
  --cookies session-cookies.txt \
  --project "Springer Batch" \
  < springer-urls.txt
```

---

## 4. Extract-then-Download Pipeline

You have a markdown notes file full of links — meeting notes, a literature survey, a curated reading list. Extract the URLs and download them in one pipeline.

```bash
cargo run --bin extract-md-links -- notes.md \
  | downloader \
      --project "Notes Links" \
      --output-dir ~/downloads \
      --dry-run
```

Remove `--dry-run` when you are happy with the resolved list.

**Process a whole directory of markdown files**

```bash
cargo run --bin extract-md-links -- ./research-notes/ \
  | downloader \
      --project "Research Notes" \
      --output-dir ~/downloads
```

By default URLs are deduplicated, so links that appear in multiple files are only downloaded once.

**Save the extracted URL list for later**

```bash
cargo run --bin extract-md-links -- ./research-notes/ -o urls.txt
downloader --project "Research Notes" --dry-run < urls.txt  # review first
downloader --project "Research Notes" --output-dir ~/downloads < urls.txt
```

---

## 5. Querying History

All download attempts are recorded in a SQLite database. Use `log` and `search` to find what you've collected.

**Show recent downloads**

```bash
# Last 50 attempts across all projects
downloader log

# Last 50 attempts for one project
downloader log --project "QA Automation Survey"

# All failures since February 2026
downloader log --failed --since "2026-02-01 00:00:00"

# Failures from a specific domain
downloader log --failed --domain link.springer.com

# Show more rows
downloader log --limit 200
```

**Filter by status**

```bash
downloader log --status success
downloader log --status failed
downloader log --status skipped
```

**Full-text search across metadata**

`search` matches against title, authors, and DOI fields stored in the history database.

```bash
# Find everything about transformers
downloader search "transformer attention"

# Scope to a project
downloader search "attention" --project "LLM Reading List"

# Date-bounded search
downloader search "CRISPR" \
  --since "2026-01-01 00:00:00" \
  --until "2026-02-01 00:00:00"

# Open the top result immediately
downloader search "PageRank algorithm" --open
```

**Inspect configuration**

```bash
downloader config show
```

---

## 6. Scripting and CI

**Exit codes for shell scripting**

```
0  all items succeeded
1  partial success (some items failed)
2  complete failure or fatal error
```

```bash
downloader --quiet < urls.txt
if [ $? -eq 1 ]; then
  echo "Some downloads failed — check the log"
  downloader log --failed --limit 100
fi
```

**Validate a URL list in CI (no downloads)**

```bash
downloader --dry-run --quiet < url_lists/qa-automation-links.txt
```

Exit code `2` means the input couldn't be parsed at all; exit code `0` means every URL resolved successfully. Suitable as a pre-merge check to catch dead links early.

**Nightly batch job**

```bash
#!/usr/bin/env bash
set -euo pipefail

downloader \
  --quiet \
  --output-dir /data/papers \
  --project "Nightly Fetch $(date +%Y-%m-%d)" \
  --detect-topics \
  --sidecar \
  --respectful \
  < /data/daily-refs.txt

exit_code=$?
if [ $exit_code -ne 0 ]; then
  downloader log --output-dir /data/papers --failed --limit 500 \
    > /data/logs/failed-$(date +%Y-%m-%d).txt
fi
exit $exit_code
```

**Polite crawling on shared infrastructure**

Use `--respectful` to avoid hammering publisher servers. It overrides concurrency, rate-limit, and retry settings with conservative values and enables robots.txt checking.

```bash
downloader \
  --respectful \
  --output-dir ~/papers \
  < large-list.txt
```

For fine-grained control without `--respectful`:

```bash
downloader \
  --concurrency 2 \
  --rate-limit 5000 \
  --rate-limit-jitter 2000 \
  --check-robots \
  --output-dir ~/papers \
  < large-list.txt
```

This waits 5–7 seconds between requests to each domain and respects each site's `robots.txt`.

**Machine-readable output (no ANSI colours)**

```bash
downloader --no-color --quiet < urls.txt 2>&1 | tee run.log
```
