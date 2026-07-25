-- 1. Idempotency: guarantee at most one ACTIVE lease per idempotency key.
-- This makes allocation retries safe at the storage layer, complementing the
-- application-level pre-check.
CREATE UNIQUE INDEX IF NOT EXISTS idx_leases_idempotency_active
    ON leases (idempotency_key)
    WHERE status = 'ACTIVE' AND idempotency_key IS NOT NULL;

-- 2. Remove the unused legacy waitlist table (superseded by waitlist_entries).
DROP TABLE IF EXISTS waitlist;

-- 3. Outbox delivery resilience: track delivery attempts and the last error so
-- the worker can retry and eventually dead-letter (FAILED) undeliverable events.
ALTER TABLE outbox_events ADD COLUMN IF NOT EXISTS attempts INT NOT NULL DEFAULT 0;
ALTER TABLE outbox_events ADD COLUMN IF NOT EXISTS last_error TEXT;
