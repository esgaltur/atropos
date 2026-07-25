-- 1. Pool capacity: optional hard cap on how many resources a pool may hold.
ALTER TABLE pools ADD COLUMN IF NOT EXISTS max_capacity INT;

-- 2. Fair-share / weighted quotas.
--    weight     : relative share used for future fair-share scheduling.
--    soft_limit : advisory threshold; allocations above it are logged but allowed.
ALTER TABLE tenant_quotas ADD COLUMN IF NOT EXISTS weight INT NOT NULL DEFAULT 1;
ALTER TABLE tenant_quotas ADD COLUMN IF NOT EXISTS soft_limit INT;

-- 3. Lease labels for tagging and cost reporting.
ALTER TABLE leases ADD COLUMN IF NOT EXISTS labels JSONB NOT NULL DEFAULT '{}';

-- 4. Reservations: capacity requested for a future point in time. A background
--    promoter allocates a real lease when start_at is reached.
CREATE TABLE IF NOT EXISTS reservations (
    id UUID PRIMARY KEY,
    pool_type VARCHAR(255) NOT NULL,
    owner_id VARCHAR(255) NOT NULL,
    tenant_id VARCHAR(255) NOT NULL,
    priority INT NOT NULL DEFAULT 0,
    ttl_seconds BIGINT NOT NULL,
    constraints JSONB,
    start_at TIMESTAMPTZ NOT NULL,
    status VARCHAR(50) NOT NULL DEFAULT 'PENDING',
    lease_id UUID,
    last_error TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
CREATE INDEX IF NOT EXISTS idx_reservations_due ON reservations (start_at) WHERE status = 'PENDING';
