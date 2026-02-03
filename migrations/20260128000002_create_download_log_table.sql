-- Migration: Create download_log table for history tracking
-- This table records all download attempts for history, debugging, and analytics.
--
-- Status values:
--   - 'success': Download completed successfully
--   - 'failed': Download failed (with error_message)
--   - 'skipped': Skipped (e.g., already exists, auth required)

CREATE TABLE IF NOT EXISTS download_log (
    id INTEGER PRIMARY KEY AUTOINCREMENT,

    -- Original URL requested
    url TEXT NOT NULL,

    -- Final URL after redirects (may differ from url)
    final_url TEXT,

    -- Outcome of the download attempt
    status TEXT NOT NULL CHECK (status IN ('success', 'failed', 'skipped')),

    -- Where the file was saved (NULL if failed)
    file_path TEXT,

    -- File size in bytes (NULL if failed)
    file_size INTEGER,

    -- HTTP Content-Type header value
    content_type TEXT,

    -- Timing information
    started_at TEXT NOT NULL,
    completed_at TEXT,

    -- Error details if failed
    error_message TEXT,

    -- Project name for organization (optional)
    project TEXT,

    -- HTTP status code from response
    http_status INTEGER,

    -- Duration in milliseconds
    duration_ms INTEGER
);

-- Index for project-based queries
CREATE INDEX IF NOT EXISTS idx_download_log_project ON download_log(project);

-- Index for time-based queries (recent downloads)
CREATE INDEX IF NOT EXISTS idx_download_log_started_at ON download_log(started_at DESC);

-- Index for URL lookups (check if already downloaded)
CREATE INDEX IF NOT EXISTS idx_download_log_url ON download_log(url);
