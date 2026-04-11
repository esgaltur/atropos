-- Add heartbeating to leases
ALTER TABLE leases ADD COLUMN last_heartbeat_at TIMESTAMPTZ DEFAULT NOW() NOT NULL;
CREATE INDEX idx_leases_liveness ON leases (status, last_heartbeat_at) WHERE status = 'ACTIVE';

-- Add rack_id for affinity/anti-affinity
ALTER TABLE resources ADD COLUMN rack_id VARCHAR(255);
CREATE INDEX idx_resources_rack ON resources (rack_id);
