-- Reverse of 20260303000006_hardening.up.sql

ALTER TABLE outbox_events DROP COLUMN IF EXISTS last_error;
ALTER TABLE outbox_events DROP COLUMN IF EXISTS attempts;

-- Recreate the legacy waitlist table.
CREATE TABLE IF NOT EXISTS waitlist (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    pool_id UUID REFERENCES pools(id),
    owner_id TEXT NOT NULL,
    priority INT DEFAULT 0,
    constraints JSONB,
    created_at TIMESTAMPTZ DEFAULT NOW()
);

DROP INDEX IF EXISTS idx_leases_idempotency_active;
