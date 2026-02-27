-- Migration: Add topics column for topic auto-detection (Story 8.1)
-- Topics are stored as JSON array of strings: ["machine learning", "climate change"]
-- This enables topic-based organization and index generation.

ALTER TABLE queue
ADD COLUMN topics TEXT; -- JSON array of topic strings

ALTER TABLE download_log
ADD COLUMN topics TEXT; -- JSON array of topic strings

-- Index for efficient topic-based queries
CREATE INDEX IF NOT EXISTS idx_download_log_topics ON download_log(topics);
