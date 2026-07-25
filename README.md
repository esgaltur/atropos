# Atropos 🚀

[![Rust](https://img.shields.io/badge/rust-1.75%2B-blue.svg)](https://www.rust-lang.org)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)
[![Rust CI](https://github.com/esgaltur/atropos/actions/workflows/ci.yml/badge.svg)](https://github.com/esgaltur/atropos/actions/workflows/ci.yml)
[![Architecture: Hexagonal](https://img.shields.io/badge/Architecture-Hexagonal-green.svg)](#architecture)

A high-performance, strictly consistent **Resource Leasing & Capacity Orchestration Platform** built in Rust. It enables teams to allocate, reserve, and govern scarce or shared resources (e.g., GPUs, license tokens, test environments) with strong linearizable guarantees.

---

### 📖 [Read the User & Use Case Guide](docs/project/USER_GUIDE.md)
*Learn why Atropos exists, how to use it for GPU clusters, CI/CD farms, and license management.*

---

## 🌟 Key Features

*   **Atomic Allocation:** Zero double-bookings using PostgreSQL `SELECT FOR UPDATE SKIP LOCKED`.
*   **Priority-Based Preemption:** High-priority workloads can automatically reclaim resources from lower-priority leases.
*   **Attribute-Aware Selection:** Request resources based on specific metadata (VRAM, Region, OS) using native JSONB containment queries.
*   **Tenant Quotas:** Prevent resource monopolization by enforcing limits on concurrent leases per tenant.
*   **Production Resilience:** Includes Lease Heartbeating (zombie detection), Waitlist Priority Aging (starvation prevention), and Rack-Aware Anti-Affinity.
*   **Reliable Events (Outbox):** Transactional outbox ensures downstream systems are notified of every grant and revocation without fail.
*   **Auto-Draining:** Maintenance service automatically monitors hardware health and drains degraded resources.
*   **High Concurrency:** Built on `tokio` and `sqlx` to handle thousands of requests per second.

---

## 🏗 Architecture

The project follows **Domain-Driven Design (DDD)** and **Hexagonal (Ports & Adapters) Architecture** to ensure long-term maintainability and testability.

### The Domain Layer (`src/domain/`)
The "Heart" of the system. Contains pure business logic, entities (Pool, Resource, Lease), and Repository interfaces (Traits). It has **zero** dependencies on external frameworks.

### The Application Layer (`src/application/`)
Orchestrates use cases. The `AllocationService` coordinates the flow by calling Domain traits, ensuring that business rules are followed before persistence.

### The Infrastructure Layer (`src/infrastructure/`)
Implements the Domain Repository traits using **PostgreSQL**. This is where the critical concurrency logic (Atomic Transactions) resides.

### The API Layer (`src/api/`)
The entry point. Implements the REST contract using **Axum**. It handles HTTP-specific concerns like JSON serialization and status code mapping.

---

## 📚 Comprehensive Documentation

For deep technical insights, Architecture Decision Records (ADRs), and operational runbooks, please explore our comprehensive documentation suite:

*   [System Architecture & Design](docs/project/architecture.md)
*   [Deployment & Scaling Guide](docs/project/deployment.md)
*   [ADR 0001: Why we use Postgres SKIP LOCKED](docs/adr/0001-postgres-skip-locked.md)
*   [ADR 0002: Hexagonal Architecture](docs/adr/0002-hexagonal-architecture.md)
*   [ADR 0003: The Background Reaper](docs/adr/0003-reaper-service.md)
*   [Disaster Recovery Runbook](docs/project/RUNBOOK.md)

---

## 🚀 Getting Started

### Prerequisites
*   [Rust](https://www.rust-lang.org/tools/install) (1.75+)
*   [Docker](https://www.docker.com/get-started) (for local PostgreSQL)
*   [sqlx-cli](https://github.com/launchbadge/sqlx/tree/main/sqlx-cli) (`cargo install sqlx-cli --no-default-features --features postgres`)

### Local Setup
1.  **Clone the Repository:**
    ```bash
    git clone https://github.com/esgaltur/atropos.git
    cd atropos
    ```

2.  **Start the Database:**
    ```bash
    docker-compose -f infra/docker-compose.yml up -d
    ```

3.  **Setup Environment:**
    ```bash
    cp .env.example .env
    # Adjust DATABASE_URL in .env if necessary
    ```

4.  **Run Migrations:**
    ```bash
    sqlx migrate run --source infra/migrations
    ```

5.  **Run the Server:**
    ```bash
    cargo run
    ```

---

## 📖 API Documentation

> **Authentication:** When `ATROPOS_API_TOKEN` is set, all mutating endpoints
> (and the gRPC `AllocationService`) require `Authorization: Bearer <token>`.
> Read-only endpoints (dashboard, `/health`, pool lookup) remain public.

### Allocate a Lease
`POST /leases`
```json
{
  "pool_type": "A100-Cluster",
  "owner_id": "team-alpha",
  "tenant_id": "project-omega",
  "ttl_seconds": 3600,
  "priority": 100,
  "waitlist": true,
  "preempt": false,
  "idempotency_key": "optional-retry-safe-key"
}
```
Responses:
* `200 OK` — lease granted (returns the `Lease`).
* `202 Accepted` — pool full and the request was queued on the waitlist.
* `409 Conflict` — no resources available (and not waitlisted).
* `429 Too Many Requests` — tenant quota exceeded.

Retries carrying the same `idempotency_key` return the original active lease
instead of creating a duplicate. Preemption of lower-priority leases only occurs
when `"preempt": true` is supplied.

### Release a Lease
`DELETE /leases/:id`

### Look up a Pool
`GET /pools/:name`

### Query Leases
`GET /leases?tenant_id=&owner_id=&status=&limit=` — list leases with optional filters.
`GET /leases/:id` — fetch a single lease.

### Pools & Capacity
Pools accept an optional `max_capacity` on creation (`POST /pools`). Once a pool
holds that many resources, `POST /resources` returns `409 Conflict`.
`GET /pools/:name/utilization` reports total/healthy resources, active leases,
availability and a utilization percentage.

### Quotas (weighted / soft limits)
`PUT /quotas` upserts a tenant quota:
```json
{
  "tenant_id": "project-omega",
  "pool_type": "A100-Cluster",
  "max_active_leases": 10,
  "soft_limit": 8,
  "weight": 5
}
```
`GET /quotas/:tenant_id/:pool_type` reads it back.

### Reservations (future capacity)
`POST /reservations` schedules capacity for a future `start_at`; a background
promoter allocates a real lease when it becomes due (`FULFILLED`/`FAILED`).
`GET /reservations/:id` reports status and the resulting `lease_id`.

### Resource Status
`PATCH /resources/:id/status` sets a resource's operational status
(`Healthy`, `Unhealthy`, `Draining`, `Disabled`, `Cooldown`).

### Waitlist Position
`GET /waitlist/:id` returns a queued request's position and the total waiting,
using the same priority-aging order the fulfiller applies.

### Labels & Cost Reporting
`PATCH /leases/:id/labels` attaches arbitrary JSON labels to a lease.
`GET /reports/cost?group_by=tenant|cost_center` aggregates active leases.

### Webhooks
Lease-granted/revoked events are delivered to rows in the `webhooks` table
matching the event type (or `*`). When `ATROPOS_WEBHOOK_SECRET` is set, bodies
are signed with HMAC-SHA256 (`X-Atropos-Signature`). Delivery failures are
retried and dead-lettered by the outbox worker.

---

## 🗺 Roadmap

- [x] **Milestone 1:** Core REST API & Postgres Persistence (Atomic Allocation).
- [x] **Milestone 2:** Lifecycle Reaper (Background worker for TTL reclamation) & Metrics.
- [x] **Milestone 3:** Waitlisting logic, Admin UI, & K8s CRD.
- [x] **Milestone 4:** Runbook for DR, Load testing, and fully operational release.

---

## 🤝 Contributing

We welcome contributions! Please see [CONTRIBUTING.md](docs/policy/CONTRIBUTING.md) for our coding standards and pull request process.

---

## 📄 License

Distributed under the MIT License. See `LICENSE` for more information.
