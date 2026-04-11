-- 1. Performance: Partial Index for high-speed SKIP LOCKED
CREATE INDEX idx_resources_healthy ON resources (pool_id) WHERE status = 'Healthy';

-- 2. Waitlisting
CREATE TABLE waitlist_entries (
    id UUID PRIMARY KEY,
    pool_type VARCHAR(255) NOT NULL,
    owner_id VARCHAR(255) NOT NULL,
    tenant_id VARCHAR(255) NOT NULL,
    priority INT NOT NULL DEFAULT 0,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- 3. Webhooks
CREATE TABLE webhooks (
    id UUID PRIMARY KEY,
    tenant_id VARCHAR(255) NOT NULL,
    url VARCHAR(255) NOT NULL,
    event_type VARCHAR(50) NOT NULL
);
