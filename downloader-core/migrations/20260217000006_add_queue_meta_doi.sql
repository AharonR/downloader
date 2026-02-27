-- Migration: Add DOI metadata field to queue rows.
-- Story 6.1 uses this field to preserve resolver DOI metadata for history logging.

ALTER TABLE queue
ADD COLUMN meta_doi TEXT;
