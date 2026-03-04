# Atropos Runbook

## Disaster Recovery (DR)
This document outlines the standard operating procedures for restoring the Atropos platform in the event of a catastrophic failure.

### 1. Database Backup (RPO < 5 minutes)
The PostgreSQL database should be backed up using Continuous Archiving and Point-in-Time Recovery (PITR) via `pgBackRest` or `WAL-G`.

**Manual Logical Backup (For minor migrations):**
```bash
pg_dump -U postgres -h localhost -d resource_allocator -F c -f /backups/resource_allocator_$(date +%F).dump
```

### 2. Database Restore (RTO < 1 hour)
To restore a logical backup into a fresh database instance:
```bash
pg_restore -U postgres -h localhost -d resource_allocator -1 /backups/resource_allocator_XXXX-XX-XX.dump
```

### 3. Graceful Degradation
If the database connection is saturated or dead:
- The API will immediately return `500 Internal Server Error` and fail closed.
- The `ReaperService` will log failures but will not crash the application. It will retry on the next tick.

## Scaling Operations

### Adding API Nodes
The API is completely stateless. To increase throughput (NFR-4), you can spin up additional container instances behind a Load Balancer. The PostgreSQL `SKIP LOCKED` concurrency model ensures no double-booking regardless of how many API instances are running.

### Monitoring
- Health Check: `GET /health`
- Metrics: Prometheus metrics are exposed via the `/metrics` endpoint (port 9000 by default or as configured).
