DROP INDEX IF EXISTS idx_leases_resource_active_unique;
ALTER TABLE leases ADD CONSTRAINT leases_resource_id_status_key UNIQUE (resource_id, status);
