-- Migration: Add metadata and naming fields to queue table
-- These fields support metadata-driven file naming and project index generation.

ALTER TABLE queue
ADD COLUMN suggested_filename TEXT;

ALTER TABLE queue
ADD COLUMN meta_title TEXT;

ALTER TABLE queue
ADD COLUMN meta_authors TEXT;

ALTER TABLE queue
ADD COLUMN meta_year TEXT;

ALTER TABLE queue
ADD COLUMN saved_path TEXT;
