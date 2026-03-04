```markdown
# Resource Leasing & Capacity Orchestration Platform — Requirements

**Version:** 2.0  
**Status:** Draft  
**Owner:** Platform Engineering  
**Last Updated:** 2023-10-27  

---

## Table of Contents
1. [Purpose & Vision](#1-purpose--vision)
2. [Definitions & State Model](#2-definitions--state-model)
3. [Scope](#3-scope)
4. [Functional Requirements](#4-functional-requirements)
5. [Non-Functional Requirements](#5-non-functional-requirements)
6. [Architecture & Design Constraints](#6-architecture--design-constraints)
7. [Acceptance Criteria](#7-acceptance-criteria)
8. [Appendix A: API Contract](#appendix-a-api-contract)
9. [Appendix B: Database Schema](#appendix-b-database-schema)
10. [Appendix C: Implementation Backlog](#appendix-c-implementation-backlog)

---

## 1. Purpose & Vision

Build a platform that **allocates, reserves, and governs scarce/shared resources** (e.g., GPUs, test environments, license tokens, IPs) across teams and workloads. The goal is to maximize utilization efficiency while preventing contention, ensuring fairness, and providing operational safety.

**Key Value Propositions:**
1.  **Eliminate Contention:** Strong consistency guarantees (no double-booking).
2.  **Maximize Utilization:** Auto-reclamation of idle resources via TTL.
3.  **Governance:** Quotas, priorities, and audit trails for chargeback/compliance.
4.  **Developer Velocity:** Self-service APIs and Kubernetes integration.

---

## 2. Definitions & State Model

### 2.1 Core Entities
*   **Resource**: A unique allocatable unit (e.g., `gpu-node-01`).
*   **Pool**: A logical grouping of resources with shared characteristics (e.g., `A100-Cluster-East`).
*   **Lease**: A time-bound contract granting an Owner exclusive access to a Resource.
*   **Waitlist Entry**: A queued request for a resource when none are currently available.
*   **Policy**: Algorithm determining which Resource satisfies a Claim and which Claim gets priority.

### 2.2 Lease State Machine
A Lease must transition through explicit states:
1.  `PENDING`: Request received, awaiting resource or policy decision.
2.  `ACTIVE`: Resource allocated, TTL counting down.
3.  `EXPIRING`: TTL threshold reached (e.g., 5 mins left), notification sent.
4.  `RELEASED`: Owner explicitly released; resource entering cooldown.
5.  `EXPIRED`: TTL reached; system forcibly reclaimed.
6.  `REVOKED`: Admin or Preemption forcibly terminated lease.
7.  `WAITING`: No resource available; queued for notification.

---

## 3. Scope

### 3.1 In Scope (v1.0)
*   CRUD for Pools, Resources, Policies.
*   Synchronous Allocation with `SELECT FOR UPDATE` consistency.
*   Lease Lifecycle (TTL, Renew, Release, Expire).
*   Waitlisting (Queueing) for unavailable resources.
*   RBAC (Admin, Operator, User, Viewer).
*   Audit Logging & Prometheus Metrics.
*   Postgres Backend (Single Primary).

### 3.2 Out of Scope (v1.0)
*   Actual Payment Processing (Cost attribution only).
*   Multi-Region Active-Active (Active-Passive DR only).
*   Hardware Provisioning (Bare-metal bring-up).
*   Complex Topology Affinity (e.g., "Same Rack" constraints).

---

## 4. Functional Requirements

### 4.1 Resource & Pool Management
| ID | Requirement |
| :--- | :--- |
| **FR-1** | System shall support **Resource Types** with schema validation for attributes (e.g., `vram_gb` must be integer). |
| **FR-2** | System shall support **Pools** with capacity limits (max total resources) and allocation policies. |
| **FR-3** | System shall support **Resource Registration** via API or Discovery Agent (heartbeat). |
| **FR-4** | System shall support **Health States**: `Healthy`, `Unhealthy` (auto-fail), `Draining` (no new leases), `Disabled`. |
| **FR-5** | System shall support **Labels/Tags** on resources for constraint matching (e.g., `zone=us-east-1`). |

### 4.2 Allocation & Deallocation
| ID | Requirement |
| :--- | :--- |
| **FR-6** | **Allocate Endpoint:** `POST /leases` shall accept constraints, TTL, Owner, and Idempotency Key. |
| **FR-7** | **Consistency:** Allocation must be **linearizable**. A resource cannot be leased to two owners simultaneously. |
| **FR-8** | **Waitlisting:** If no resource matches, the request shall optionally enter a **Waitlist** (FIFO or Priority) instead of failing immediately. |
| **FR-9** | **Release Endpoint:** `DELETE /leases/{id}` shall immediately free the resource and trigger cooldown logic. |
| **FR-10** | **Deallocate by Owner:** `DELETE /leases?owner=X&type=Y` shall release the newest matching lease (LIFO) to support ephemeral workload teardown. |

### 4.3 Lease Lifecycle & Reclamation
| ID | Requirement |
| :--- | :--- |
| **FR-11** | **TTL Enforcement:** Every lease requires `created_at` + `ttl_seconds`. |
| **FR-12** | **Renewal:** `PATCH /leases/{id}` extends `expires_at`. Must validate against Max TTL policy. |
| **FR-13** | **Reaper Service:** Background worker shall scan for `EXPIRED` leases every `N` seconds (configurable, e.g., 10s) and force transition to `RELEASED`. |
| **FR-14** | **Cooldown:** Released resources shall enter a `COOLDOWN` state (configurable duration) before becoming `Healthy` again to prevent flapping. |

### 4.4 Scheduling & Policies
| ID | Requirement |
| :--- | :--- |
| **FR-15** | **Matching Strategies:** Support `FirstAvailable`, `LeastRecentlyUsed` (LRU), and `Random`. |
| **FR-16** | **Priority Queues:** Support Priority Classes (e.g., `Critical`, `Standard`, `BestEffort`). Higher priority skips Waitlist. |
| **FR-17** | **Quotas:** Enforce `MaxActiveLeases` per Tenant/Pool. Reject allocation if quota exceeded. |
| **FR-18** | **Preemption (v1.0 Lite):** Support "Soft Preemption" where `Critical` requests can notify `BestEffort` owners to release, but do not forcibly kill (unless configured). |

### 4.5 Cost & Attribution
| ID | Requirement |
| :--- | :--- |
| **FR-19** | **Cost Tags:** Leases shall accept `cost_center` or `project_id` tags. |
| **FR-20** | **Usage Reporting:** System shall aggregate `lease_duration * resource_rate` for export (CSV/JSON) for chargeback calculations. |

### 4.6 Observability & Audit
| ID | Requirement |
| :--- | :--- |
| **FR-21** | **Audit Log:** Immutable log of all state transitions (Who, What, When, Before, After). |
| **FR-22** | **Metrics:** Expose `allocation_latency_histogram`, `active_leases_gauge`, `waitlist_depth`, `reclaim_count`. |
| **FR-23** | **Tracing:** Propagate `trace_id` through API -> DB -> Reaper for debugging latency. |

### 4.7 Access Control
| ID | Requirement |
| :--- | :--- |
| **FR-24** | **AuthN:** Support OIDC (JWT) and API Keys. |
| **FR-25** | **AuthZ:** RBAC Matrix (Admin, Operator, User, Viewer). |
| **FR-26** | **Tenancy:** Logical isolation. Users cannot see leases from other tenants unless `Shared Pool` policy is enabled. |

### 4.8 Integrations
| ID | Requirement |
| :--- | :--- |
| **FR-27** | **Webhooks:** Notify on `lease.granted`, `lease.expiring_soon`, `lease.revoked`. |
| **FR-28** | **Kubernetes CRD:** `ResourceClaim` kind that maps to Platform Lease. |

---

## 5. Non-Functional Requirements (NFR)

### 5.1 Correctness & Concurrency
| ID | Requirement |
| :--- | :--- |
| **NFR-1** | **No Double Allocation:** Enforced via Database Row-Level Locking (`SELECT FOR UPDATE SKIP LOCKED`) or Optimistic Locking (`version` column). |
| **NFR-2** | **Idempotency:** Same `idempotency_key` within `T` seconds returns identical result without side effects. |
| **NFR-3** | **Clock Skew:** System shall rely on Database time (`NOW()`) for TTL calculations, not application server time. |

### 5.2 Performance & Scale
| ID | Requirement |
| :--- | :--- |
| **NFR-4** | **Throughput:** Support 1,000 allocate requests/sec (p99 < 100ms) on standard hardware. |
| **NFR-5** | **Capacity:** Support 100,000 Resources and 50,000 concurrent Active Leases. |
| **NFR-6** | **Reaper Lag:** Expired leases must be reclaimed within 30s of expiry. |

### 5.3 Reliability & DR
| ID | Requirement |
| :--- | :--- |
| **NFR-7** | **Availability:** 99.9% uptime SLA. |
| **NFR-8** | **Recovery:** RTO < 1 hour, RPO < 5 minutes (via DB snapshots/WAL). |
| **NFR-9** | **Graceful Degradation:** If Waitlist service fails, allocation fails fast (no silent drops). |

### 5.4 Security
| ID | Requirement |
| :--- | :--- |
| **NFR-10** | **Secrets:** No PII or Secrets in logs. |
| **NFR-11** | **Encryption:** TLS 1.3 for transit; Encryption at Rest for DB. |
| **NFR-12** | **Rate Limiting:** API endpoints rate-limited per Tenant to prevent DoS. |

---

## 6. Architecture & Design Constraints

| ID | Constraint |
| :--- | :--- |
| **ADC-1** | **Database:** PostgreSQL 14+ required (for `SKIP LOCKED` and JSONB support). |
| **ADC-2** | **Language:** Go or Rust preferred for concurrency safety and performance. |
| **ADC-3** | **Locking Strategy:** Use `pg_advisory_xact_lock` or Row-Level Locking on Resource table during allocation to prevent race conditions. |
| **ADC-4** | **Stateless API:** API nodes must not store lease state in memory; all state in DB. |

---

## 7. Acceptance Criteria (Definition of Done)

1.  **Concurrency Test:** 100 parallel requests for 1 unique resource results in exactly 1 success, 99 waitlist/reject.
2.  **TTL Test:** Lease expires automatically; resource becomes available after cooldown.
3.  **Audit Test:** Every state change is queryable in the audit table.
4.  **Load Test:** System sustains NFR-4 targets for 1 hour without memory leaks.
5.  **Security Test:** Pen-test confirms no tenant data leakage.

---

## Appendix A: API Contract

```yaml
openapi: 3.0.0
info:
  title: Resource Orchestration API
  version: 1.0.0

paths:
  /pools:
    post:
      summary: Create a Resource Pool
      requestBody:
        content:
          application/json:
            schema:
              type: object
              properties:
                name: { type: string }
                type: { type: string }
                policy: { type: string, enum: [FIFO, LRU, PRIORITY] }
    
  /resources:
    post:
      summary: Register a Resource
      requestBody:
        content:
          application/json:
            schema:
              type: object
              properties:
                pool_id: { type: string }
                external_id: { type: string }
                attributes: { type: object } # e.g. { "vram": 80 }

  /leases:
    post:
      summary: Allocate a Lease
      operationId: allocateLease
      requestBody:
        content:
          application/json:
            schema:
              type: object
              properties:
                pool_type: { type: string }
                constraints: { type: object } # e.g. { "vram_min": 40 }
                ttl_seconds: { type: integer }
                idempotency_key: { type: string }
                waitlist: { type: boolean, default: false }
      responses:
        200:
          description: Lease Granted
          content:
            application/json:
              schema:
                $ref: '#/components/schemas/Lease'
        202:
          description: Added to Waitlist
        409:
          description: Quota Exceeded

    delete:
      summary: Release Leases
      parameters:
        - name: owner_id
          in: query
        - name: type
          in: query

  /leases/{lease_id}/renew:
    patch:
      summary: Renew a Lease
      requestBody:
        content:
          application/json:
            schema:
              type: object
              properties:
                extension_seconds: { type: integer }

components:
  schemas:
    Lease:
      type: object
      properties:
        id: { type: string }
        resource_id: { type: string }
        owner_id: { type: string }
        status: { type: string, enum: [ACTIVE, EXPIRING, RELEASED] }
        expires_at: { type: string, format: date-time }
```

---

## Appendix B: Database Schema

```sql
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
    external_id TEXT NOT NULL, -- Physical ID (e.g., hostname)
    status TEXT NOT NULL DEFAULT 'Healthy', -- Healthy, Unhealthy, Draining
    attributes JSONB DEFAULT '{}', -- e.g. {"vram": 80}
    version BIGINT DEFAULT 0, -- For optimistic locking
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
    status TEXT NOT NULL DEFAULT 'ACTIVE', -- ACTIVE, EXPIRED, RELEASED
    created_at TIMESTAMPTZ DEFAULT NOW(),
    expires_at TIMESTAMPTZ NOT NULL,
    idempotency_key TEXT, 
    cost_center TEXT,
    UNIQUE(resource_id, status) -- Prevent double active lease logic at DB level
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
-- Partition audit_log by month in production for performance
```

**Critical Concurrency Note:**
For Allocation, use a transaction with `SELECT ... FOR UPDATE SKIP LOCKED` on the `resources` table to find a healthy resource, then `INSERT` into `leases`. This ensures two API nodes never grab the same resource row simultaneously.

---

## Appendix C: Implementation Backlog

### Milestone 1: The Core (Weeks 1-4)
- [ ] **Setup:** Repo init, CI/CD pipeline, Dockerfile, Local Dev Env (Docker Compose with Postgres).
- [ ] **DB:** Implement Schema (Appendix B) and migrations.
- [ ] **API:** Implement `CreatePool`, `RegisterResource`, `Allocate`, `Release`.
- [ ] **Logic:** Implement `SELECT FOR UPDATE SKIP LOCKED` allocation logic.
- [ ] **Test:** Write concurrency test (100 goroutines hitting 1 resource).

### Milestone 2: Lifecycle & Safety (Weeks 5-8)
- [ ] **Reaper:** Build background worker to scan `expires_at` and update status.
- [ ] **Renewal:** Implement `PATCH /leases/{id}/renew`.
- [ ] **Auth:** Integrate JWT Middleware and RBAC checks.
- [ ] **Audit:** Implement database triggers or app-side logging to `audit_log`.
- [ ] **Metrics:** Expose Prometheus endpoints for lease counts and latency.

### Milestone 3: Advanced Features (Weeks 9-12)
- [ ] **Waitlist:** Implement Queue logic and Webhook notification on grant.
- [ ] **Quotas:** Add logic to check `COUNT(leases) WHERE tenant_id = X` before allocate.
- [ ] **UI:** Build minimal Admin Dashboard (React/HTMX) to view pools/leases.
- [ ] **K8s:** Create `ResourceClaim` CRD and Controller skeleton.
- [ ] **Load Test:** Run k6 or Locust script to validate NFR-4.

### Milestone 4: Hardening (Weeks 13-14)
- [ ] **DR:** Document Backup/Restore procedure.
- [ ] **Security:** Pen-test and dependency scan.
- [ ] **Docs:** Write `README.md`, API Swagger, and Runbook for Ops.
- [ ] **Release:** v1.0.0 Tag.
```