-- Migration: Add metadata fields and query indexes for download history.
-- Story 6.1 requires title/authors/doi persistence and fast filtering by status/project/time.

ALTER TABLE download_log
ADD COLUMN title TEXT;

ALTER TABLE download_log
ADD COLUMN authors TEXT;

ALTER TABLE download_log
ADD COLUMN doi TEXT;

CREATE INDEX IF NOT EXISTS idx_download_log_status ON download_log(status);

CREATE INDEX IF NOT EXISTS idx_download_log_project_status_started_at
ON download_log(project, status, started_at DESC);

CREATE INDEX IF NOT EXISTS idx_download_log_doi ON download_log(doi);
