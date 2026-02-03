-- Migration: Create queue table for download orchestration
-- This table holds pending, in-progress, and failed download items.
--
-- Status values:
--   - 'pending': Waiting to be processed
--   - 'in_progress': Currently being downloaded
--   - 'completed': Successfully downloaded
--   - 'failed': Failed after all retries exhausted
--
-- Source types:
--   - 'direct_url': A direct HTTP/HTTPS URL
--   - 'doi': A DOI that was resolved to a URL
--   - 'reference': A bibliographic reference that was parsed

CREATE TABLE IF NOT EXISTS queue (
    id INTEGER PRIMARY KEY AUTOINCREMENT,

    -- The resolved URL to download
    url TEXT NOT NULL,

    -- How this item entered the queue
    source_type TEXT NOT NULL CHECK (source_type IN ('direct_url', 'doi', 'reference')),

    -- Original user input (for reference/debugging)
    original_input TEXT,

    -- Current processing status
    status TEXT NOT NULL DEFAULT 'pending' CHECK (status IN ('pending', 'in_progress', 'completed', 'failed')),

    -- Higher priority items processed first (default 0)
    priority INTEGER NOT NULL DEFAULT 0,

    -- Number of retry attempts made
    retry_count INTEGER NOT NULL DEFAULT 0,

    -- Last error message if failed
    last_error TEXT,

    -- Timestamps
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

-- Index for efficient status-based queries (get pending items)
CREATE INDEX IF NOT EXISTS idx_queue_status ON queue(status);

-- Index for priority ordering
CREATE INDEX IF NOT EXISTS idx_queue_priority ON queue(priority DESC, created_at ASC);
