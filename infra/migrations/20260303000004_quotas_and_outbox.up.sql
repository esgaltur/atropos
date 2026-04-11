CREATE TABLE tenant_quotas (
    tenant_id VARCHAR(255) NOT NULL,
    pool_type VARCHAR(255) NOT NULL,
    max_active_leases INTEGER NOT NULL,
    PRIMARY KEY (tenant_id, pool_type)
);

CREATE TABLE outbox_events (
    id UUID PRIMARY KEY,
    event_type VARCHAR(255) NOT NULL,
    payload JSONB NOT NULL,
    status VARCHAR(50) DEFAULT 'PENDING' NOT NULL,
    created_at TIMESTAMPTZ DEFAULT NOW() NOT NULL,
    processed_at TIMESTAMPTZ
);

CREATE INDEX idx_outbox_pending ON outbox_events (status, created_at) WHERE status = 'PENDING';
