-- Migration: Add project scoping to queue rows.
--
-- The downloader-app uses a global queue database. This column allows
-- queue operations to be isolated to the current output project key.

ALTER TABLE queue
ADD COLUMN project TEXT;

CREATE INDEX IF NOT EXISTS idx_queue_project_status_priority_created
ON queue(project, status, priority DESC, created_at ASC);

CREATE INDEX IF NOT EXISTS idx_queue_project_url_status
ON queue(project, url, status);
