-- Migration: Add resumable download tracking fields to queue table
-- These fields allow interrupted downloads to record partial progress.

ALTER TABLE queue
ADD COLUMN bytes_downloaded INTEGER NOT NULL DEFAULT 0;

ALTER TABLE queue
ADD COLUMN content_length INTEGER;
