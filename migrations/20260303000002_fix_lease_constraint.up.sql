-- 1. Remove the broken rigid constraint
ALTER TABLE leases DROP CONSTRAINT IF EXISTS leases_resource_id_status_key;

-- 2. Add a Partial Unique Index: 
-- This allows multiple 'RELEASED' records for history, 
-- but strictly prevents more than one 'ACTIVE' lease per resource.
CREATE UNIQUE INDEX idx_leases_resource_active_unique ON leases (resource_id) WHERE (status = 'ACTIVE');
