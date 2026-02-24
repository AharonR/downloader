-- Migration: Add composite queue index for dequeue/status-ordered queries
--
-- Optimizes:
--   SELECT ... FROM queue
--   WHERE status = ?
--   ORDER BY priority DESC, created_at ASC
--
-- Used by queue.dequeue() and queue.list_by_status().
CREATE INDEX IF NOT EXISTS idx_queue_status_priority_created
ON queue(status, priority DESC, created_at ASC);
