-- Migration: Add parser confidence metadata to queue and download history.
-- Story 8.3 persists deterministic confidence level/factors for reference-derived inputs.

ALTER TABLE queue
ADD COLUMN parse_confidence TEXT;

ALTER TABLE queue
ADD COLUMN parse_confidence_factors TEXT;

ALTER TABLE download_log
ADD COLUMN parse_confidence TEXT;

ALTER TABLE download_log
ADD COLUMN parse_confidence_factors TEXT;

-- Supports `downloader log --uncertain` (low-confidence only) with stable sort.
CREATE INDEX IF NOT EXISTS idx_download_log_parse_confidence_started_at
ON download_log(parse_confidence, started_at DESC);

-- Narrow index for uncertain-only scans.
CREATE INDEX IF NOT EXISTS idx_download_log_uncertain_started_at
ON download_log(started_at DESC)
WHERE parse_confidence = 'low';
