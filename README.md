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
```

## Options

| Flag | Short | Description | Default |
|------|-------|-------------|---------|
| `--concurrency` | `-c` | Max concurrent downloads (1-100) | 10 |
| `--max-retries` | `-r` | Max retry attempts for transient failures (0-10) | 3 |
| `--rate-limit` | `-l` | Min delay between requests to same domain in ms (0 to disable) | 1000 |
| `--verbose` | `-v` | Increase verbosity (`-v` debug, `-vv` trace) | info |
| `--quiet` | `-q` | Suppress non-error output | off |

Flags may appear before or after positional URLs. Use `--` to pass a URL literal that starts with `-`.

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

## License

MIT
