-- Reverse of 20260303000007_platform_features.up.sql

DROP TABLE IF EXISTS reservations;

ALTER TABLE leases DROP COLUMN IF EXISTS labels;

ALTER TABLE tenant_quotas DROP COLUMN IF EXISTS soft_limit;
ALTER TABLE tenant_quotas DROP COLUMN IF EXISTS weight;

ALTER TABLE pools DROP COLUMN IF EXISTS max_capacity;
