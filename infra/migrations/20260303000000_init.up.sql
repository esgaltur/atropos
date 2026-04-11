-- Enable UUID extension
CREATE EXTENSION IF NOT EXISTS "uuid-ossp";

-- 1. Pools
CREATE TABLE pools (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    name TEXT NOT NULL UNIQUE,
    resource_type TEXT NOT NULL,
    policy TEXT NOT NULL DEFAULT 'FIFO',
    created_at TIMESTAMPTZ DEFAULT NOW()
);

-- 2. Resources
CREATE TABLE resources (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    pool_id UUID REFERENCES pools(id),
    external_id TEXT NOT NULL,
    status TEXT NOT NULL DEFAULT 'Healthy',
    attributes JSONB DEFAULT '{}',
    version BIGINT DEFAULT 0,
    updated_at TIMESTAMPTZ DEFAULT NOW()
);
CREATE INDEX idx_resources_status ON resources(status);
CREATE INDEX idx_resources_attrs ON resources USING GIN(attributes);

-- 3. Leases
CREATE TABLE leases (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    resource_id UUID REFERENCES resources(id),
    owner_id TEXT NOT NULL,
    tenant_id TEXT NOT NULL,
    status TEXT NOT NULL DEFAULT 'ACTIVE',
    created_at TIMESTAMPTZ DEFAULT NOW(),
    expires_at TIMESTAMPTZ NOT NULL,
    idempotency_key TEXT, 
    cost_center TEXT,
    UNIQUE(resource_id, status)
);
CREATE INDEX idx_leases_owner ON leases(owner_id);
CREATE INDEX idx_leases_expires ON leases(expires_at) WHERE status = 'ACTIVE';

-- 4. Waitlist
CREATE TABLE waitlist (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    pool_id UUID REFERENCES pools(id),
    owner_id TEXT NOT NULL,
    priority INT DEFAULT 0,
    constraints JSONB,
    created_at TIMESTAMPTZ DEFAULT NOW()
);

-- 5. Audit Log (Append Only)
CREATE TABLE audit_log (
    id BIGSERIAL PRIMARY KEY,
    actor_id TEXT,
    action TEXT,
    resource_id UUID,
    lease_id UUID,
    details JSONB,
    created_at TIMESTAMPTZ DEFAULT NOW()
);
