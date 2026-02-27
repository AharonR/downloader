-- Migration: Add failure-detail tracking columns to download_log.
-- Story 6.2 requires actionable failure diagnostics for post-run analysis.

ALTER TABLE download_log
ADD COLUMN error_type TEXT CHECK (error_type IN ('network', 'auth', 'not_found', 'parse_error'));

ALTER TABLE download_log
ADD COLUMN retry_count INTEGER NOT NULL DEFAULT 0;

ALTER TABLE download_log
ADD COLUMN last_retry_at TEXT;

ALTER TABLE download_log
ADD COLUMN original_input TEXT;

CREATE INDEX IF NOT EXISTS idx_download_log_error_type_started_at
ON download_log(error_type, started_at DESC);

CREATE INDEX IF NOT EXISTS idx_download_log_status_retry
ON download_log(status, retry_count DESC, started_at DESC);
