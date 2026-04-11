-- Add priority column to leases table
ALTER TABLE leases ADD COLUMN priority INTEGER DEFAULT 0 NOT NULL;
-- Index for preemption searches
CREATE INDEX idx_leases_preemption ON leases (status, priority) WHERE status = 'ACTIVE';
