# AI Agent Prompt: Open-Source Documentation Readiness — Atropos

> **Purpose:** Execute all changes required to bring the Atropos project to full open-source readiness.
> **Date:** 2026-03-03
> **Project:** Atropos — Resource Leasing & Capacity Orchestration Platform (Rust)
> **Repo root:** `C:\Users\Root\Workspace\RustProjects\ResourceAllocator`

---

## Context

Atropos is a Rust project using Axum, SQLx (Postgres), Tonic (gRPC), Tokio, and a Hexagonal Architecture (Domain → Application → Infrastructure → API). The project is functionally complete but has documentation gaps that must be fixed before it can be published as an open-source repository.

**What already exists (do NOT recreate):**
- `README.md` — features, architecture overview, getting started, API examples, roadmap
- `CONTRIBUTING.md` — coding standards, PR checklist, conventional commits
- `LICENSE` — MIT
- `docs/architecture.md`, `docs/deployment.md`, `docs/api-guide.md`
- `docs/adr/0001-postgres-skip-locked.md`, `0002-hexagonal-architecture.md`, `0003-reaper-service.md`
- `RUNBOOK.md` — disaster recovery, scaling, monitoring
- `openapi.yaml` — full OpenAPI 3.0 spec
- `.github/workflows/ci.yml` — fmt → clippy → migrations → tests
- `.env.example` — documented env vars
- `Dockerfile` — multi-stage build
- `PROJECT_STATE.md` — project retrospective / milestone tracking

---

## TASK 1: Create `SECURITY.md` (new file — repo root)

Create a security vulnerability disclosure policy. This is **critical** for any open-source project.

**File:** `SECURITY.md`

**Requirements:**
- Title: "Security Policy"
- Section "Supported Versions": Table showing version `0.1.x` as currently supported (✅) with a note that only the latest release receives security patches.
- Section "Reporting a Vulnerability":
  - Instruct reporters to **NOT** open a public GitHub Issue for security vulnerabilities.
  - Provide an email address for private disclosure: `security@atropos-project.dev` (placeholder — the maintainer should replace this).
  - Ask reporters to include: description of the vulnerability, steps to reproduce, affected version, and potential impact.
  - Commit to acknowledging reports within **48 hours** and providing a fix timeline within **7 days**.
- Section "Disclosure Policy":
  - State that the project follows **Coordinated Disclosure** — the reporter and maintainers agree on a public disclosure date (typically 90 days after the report).
  - Security advisories will be published via GitHub Security Advisories.
- Section "Scope":
  - List what is in scope: the `atropos` binary, the REST API (`/pools`, `/resources`, `/leases`, `/health`), the gRPC API (port 50051), the Postgres repository layer, and the Docker image.
  - List what is out of scope: the demo scripts (`demo.ps1`, `verify_full.ps1`), the load test (`load_test.js`), third-party dependencies (report upstream), and the local development `docker-compose.yml`.
- Tone: Professional, concise.

---

## TASK 2: Create `CODE_OF_CONDUCT.md` (new file — repo root)

**File:** `CODE_OF_CONDUCT.md`

**Requirements:**
- Adopt the **Contributor Covenant v2.1** in full (https://www.contributor-covenant.org/version/2/1/code_of_conduct/).
- Replace the `[INSERT CONTACT METHOD]` placeholder with `conduct@atropos-project.dev` (placeholder email).
- This is a standard, widely-adopted text — use the official version verbatim. Do not summarize or abbreviate it.

---

## TASK 3: Create `CHANGELOG.md` (new file — repo root)

Backfill a changelog from the project history recorded in `PROJECT_STATE.md`.

**File:** `CHANGELOG.md`

**Requirements:**
- Follow the [Keep a Changelog](https://keepachangelog.com/en/1.1.0/) format exactly.
- Follow [Semantic Versioning](https://semver.org/).
- Include a header linking to Keep a Changelog and SemVer.
- Create **one released version** and one **Unreleased** section:

### `[Unreleased]`
- Empty (or a placeholder note: "No unreleased changes.")

### `[0.1.0] - 2026-03-03`
Populate this from `PROJECT_STATE.md` milestones. Organize entries into the standard categories:

**Added:**
- Core REST API with Axum (endpoints: `POST /pools`, `POST /resources`, `POST /leases`, `DELETE /leases/:id`, `PATCH /leases/:id/renew`, `GET /health`).
- Atomic resource allocation using PostgreSQL `SELECT FOR UPDATE SKIP LOCKED` — zero double-bookings under concurrent load.
- Time-bound lease model with configurable TTL (Time-To-Live) per allocation.
- Hexagonal Architecture with strict layer separation: Domain → Application → Infrastructure → API.
- Type-safe domain model using Newtype pattern (`PoolId`, `ResourceId`, `LeaseId`) and enum state machines (`LeaseStatus`, `ResourceStatus`, `AllocationPolicy`).
- Background `ReaperService` — autonomous TTL enforcement that reclaims expired leases every 10 seconds.
- Background `MaintenanceService` — automatic pruning of leases older than 30 days and audit logs older than 90 days.
- Prometheus metrics exporter on port 9000 (`/metrics` endpoint) with `reclaim_count` counter.
- Structured logging via `tracing` with configurable `RUST_LOG` env var.
- OpenTelemetry OTLP distributed tracing scaffolding.
- Durable waitlisting — database-backed queue for resource pools at capacity.
- Preemption request support (scaffolding, returns error if invoked).
- Idempotency key support on lease allocation.
- Cost center tagging on leases.
- gRPC API via Tonic/Protobuf on port 50051 (`Allocate` RPC).
- Admin dashboard with Askama templates, Tailwind CSS, and HTMX for real-time stats and audit log streaming.
- Append-only audit log recording every `ALLOCATE` and `RELEASE` action.
- In-memory pool caching via `moka` (30-second TTL, 100 entries max).
- Full OpenAPI 3.0 specification (`openapi.yaml`).
- Kubernetes CRD definition (`k8s/crd.yaml`).
- Multi-stage Docker build with dependency caching.
- GitHub Actions CI pipeline (format check → clippy lint → database migrations → tests).
- Database migrations via `sqlx` (3 migration sets: init, advanced features, lease constraint fix).
- Cucumber BDD test scenarios for allocation and lifecycle flows.
- Unit tests and doc-tests for domain logic (lease expiration).
- Comprehensive documentation: architecture docs, deployment guide, API integration guide, 3 ADRs, disaster recovery runbook.
- Graceful shutdown with SIGINT/SIGTERM signal handling.

**Fixed:**
- Replaced `UNIQUE` constraint on leases with a partial index to allow unlimited historical `RELEASED`/`EXPIRED` records while preventing multiple `ACTIVE` leases per resource.
- Fixed missing `preempt` parameter in `AllocateRequest` struct.
- Fixed crate resolution in `main.rs`.

---

## TASK 4: Add Rust Doc Comments (`///`) to All Public Items

This is the largest task. Every public `struct`, `enum`, `trait`, function, and method across all source files must have a `///` doc comment. The comments should be **concise** (1-3 lines) and describe **what** the item does, not **how** it does it. Use the project's domain language (pools, resources, leases, TTL, reaper, etc.).

Below is the **exact specification** for every file, listing every public item that needs a doc comment. Items that already have doc comments are marked ✅ — do NOT modify those.

---

### 4.1 `src/lib.rs`
Current content:
```rust
pub mod domain;
pub mod infrastructure;
pub mod application;
pub mod api;
```

**Add:**
- A module-level doc comment (`//!`) at the top of the file:
  ```
  //! # Atropos
  //!
  //! A high-performance resource leasing and capacity orchestration platform.
  //!
  //! This crate implements a strictly-consistent allocation engine using PostgreSQL
  //! `SKIP LOCKED` for zero-double-booking guarantees under concurrent load.
  //!
  //! ## Architecture
  //!
  //! The crate follows Hexagonal (Ports & Adapters) architecture:
  //!
  //! - [`domain`] — Pure business logic, entities, and repository trait definitions (Ports).
  //! - [`application`] — Use-case orchestration services.
  //! - [`infrastructure`] — PostgreSQL-backed repository implementations (Adapters).
  //! - [`api`] — HTTP (Axum) and gRPC (Tonic) delivery mechanisms (Adapters).
  ```

---

### 4.2 `src/domain/mod.rs`
Add doc comments to every public item:

| Item | Doc Comment |
|---|---|
| `mod.rs` top (module-level `//!`) | `//! Core domain types, entities, and enums.\n//!\n//! This module contains the pure business logic with zero framework dependencies.` |
| `struct PoolId` | `/// Strongly-typed identifier for a resource pool.` |
| `PoolId::new()` | `/// Generates a new random pool identifier.` |
| `struct ResourceId` | `/// Strongly-typed identifier for an individual resource.` |
| `ResourceId::new()` | `/// Generates a new random resource identifier.` |
| `struct LeaseId` | `/// Strongly-typed identifier for a lease contract.` |
| `LeaseId::new()` | `/// Generates a new random lease identifier.` |
| `enum AllocationPolicy` | `/// Strategy used to select which resource satisfies a lease request.` |
| `enum ResourceStatus` | `/// Health and availability state of a resource.` |
| `enum LeaseStatus` | `/// Lifecycle state of a lease contract.` |

---

### 4.3 `src/domain/pool.rs`
| Item | Doc Comment |
|---|---|
| Module-level `//!` | `//! Resource pool entity.` |
| `struct Pool` | `/// A logical grouping of resources with a shared type and allocation policy.\n///\n/// Pools define the boundary within which resources are allocated to tenants.` |
| `Pool::new()` | `/// Creates a new pool with a generated ID and the current timestamp.` |

---

### 4.4 `src/domain/resource.rs`
| Item | Doc Comment |
|---|---|
| Module-level `//!` | `//! Resource entity.` |
| `struct Resource` | `/// An individual allocatable unit within a pool (e.g., a GPU node, a license token).\n///\n/// Resources track their health status and carry arbitrary metadata as JSON attributes.` |
| `Resource::new()` | `/// Creates a new healthy resource with a generated ID and the current timestamp.` |

---

### 4.5 `src/domain/lease.rs`
| Item | Doc Comment |
|---|---|
| Module-level `//!` | `//! Lease entity and expiration logic.` |
| `struct Lease` | `/// A time-bound contract granting an owner exclusive access to a resource.\n///\n/// Leases are the central unit of work in the allocation engine. They transition\n/// through a state machine: `Active` → `Released` \| `Expired` \| `Revoked`.` |
| `Lease::new()` | `/// Creates a new active lease with a computed expiration time (`now + ttl_seconds`).` |
| ✅ `Lease::is_expired()` | Already documented — do NOT change. |

---

### 4.6 `src/domain/error.rs`
| Item | Doc Comment |
|---|---|
| Module-level `//!` | `//! Domain error types.` |
| `enum DomainError` | `/// Errors that can occur within the domain and application layers.\n///\n/// Infrastructure-specific errors are wrapped in the [`InfrastructureError`](DomainError::InfrastructureError) variant\n/// to maintain the hexagonal boundary.` |

---

### 4.7 `src/domain/repository.rs`
| Item | Doc Comment |
|---|---|
| Module-level `//!` | `//! Repository trait definitions (Ports).\n//!\n//! These traits define the persistence contract that infrastructure adapters must implement.` |
| `trait PoolRepository` | `/// Persistence operations for resource pools.` |
| `PoolRepository::create()` | `/// Persists a new pool.` |
| `PoolRepository::find_by_id()` | `/// Retrieves a pool by its unique identifier, returning `None` if not found.` |
| `trait ResourceRepository` | `/// Persistence operations for individual resources.` |
| `ResourceRepository::create()` | `/// Persists a new resource.` |
| `ResourceRepository::find_by_id()` | `/// Retrieves a resource by its unique identifier, returning `None` if not found.` |
| `trait AllocationRepository` | `/// Core allocation and lease management operations.\n///\n/// This is the primary port through which the application layer interacts\n/// with the persistence backend for atomic resource allocation.` |
| ✅ `AllocationRepository::allocate_resource()` | Already documented — do NOT change. |
| `AllocationRepository::release_lease()` | `/// Transitions an active lease to the `Released` state, freeing its resource.` |
| `AllocationRepository::renew_lease()` | `/// Extends the expiration time of an active lease by the given number of seconds.` |
| `AllocationRepository::waitlist_resource()` | `/// Adds a tenant to the waitlist queue for a resource pool that is at capacity.` |
| `AllocationRepository::get_summary_stats()` | `/// Returns aggregate statistics for the admin dashboard.` |
| `AllocationRepository::get_recent_audit_logs()` | `/// Returns the most recent audit log entries, ordered by creation time descending.` |
| `struct SummaryStats` | `/// Aggregate platform statistics for the admin dashboard.` |
| `struct AuditLogEntry` | `/// A single entry in the append-only audit trail.` |

---

### 4.8 `src/application/mod.rs`
| Item | Doc Comment |
|---|---|
| Module-level `//!` | `//! Application services (use-case orchestration).\n//!\n//! These services coordinate domain entities and repository ports to fulfill\n//! business use cases. They contain no HTTP or SQL logic.` |

---

### 4.9 `src/application/allocation_service.rs`
| Item | Doc Comment |
|---|---|
| Module-level `//!` | `//! Allocation use-case service.` |
| `struct AllocationService` | `/// Orchestrates the lease allocation workflow including fallback to waitlisting and preemption.\n///\n/// This service sits between the API layer and the repository port, applying\n/// business rules before delegating to the persistence backend.` |
| `AllocationService::new()` | `/// Creates a new service instance with an in-memory pool cache (100 entries, 30s TTL).` |
| `AllocationService::allocate()` | `/// Attempts to allocate a resource lease. Falls back to waitlisting or preemption\n/// if the pool is exhausted and the caller opted in via the `waitlist`/`preempt` flags.` |
| `AllocationService::release()` | `/// Releases an active lease, returning its resource to the available pool.` |
| `AllocationService::renew()` | `/// Extends an active lease by the specified number of seconds.` |
| `AllocationService::get_stats()` | `/// Retrieves aggregate platform statistics for the admin dashboard.` |
| `AllocationService::get_recent_logs()` | `/// Retrieves the most recent audit log entries.` |

---

### 4.10 `src/application/pool_service.rs`
| Item | Doc Comment |
|---|---|
| Module-level `//!` | `//! Pool management use-case service.` |
| `struct PoolService` | `/// Service for creating and managing resource pools.` |
| `PoolService::new()` | `/// Creates a new pool service backed by the given repository.` |
| `PoolService::create_pool()` | `/// Creates a new resource pool with the specified name, type, and allocation policy.` |

---

### 4.11 `src/application/resource_service.rs`
| Item | Doc Comment |
|---|---|
| Module-level `//!` | `//! Resource registration use-case service.` |
| `struct ResourceService` | `/// Service for registering resources into pools.` |
| `ResourceService::new()` | `/// Creates a new resource service backed by the given repository.` |
| `ResourceService::register_resource()` | `/// Registers a new resource into the specified pool with the given external ID and attributes.` |

---

### 4.12 `src/application/reaper.rs`
| Item | Doc Comment |
|---|---|
| Module-level `//!` | `//! Background reaper service for TTL enforcement.\n//!\n//! The reaper runs on a fixed interval and transitions all expired active leases\n//! to the `Expired` state, returning their resources to the available pool.` |
| `struct ReaperService` | `/// Autonomous background worker that enforces lease TTL expiration.\n///\n/// Runs every 10 seconds, executing a bulk `UPDATE` against the `leases` table.\n/// Safe to run across multiple API instances — PostgreSQL handles the concurrency.` |
| `ReaperService::new()` | `/// Creates a new reaper service connected to the given database pool.` |
| `ReaperService::run()` | `/// Starts the infinite reaper loop. This method never returns under normal operation.` |

---

### 4.13 `src/application/maintenance.rs`
| Item | Doc Comment |
|---|---|
| Module-level `//!` | `//! Background maintenance service for database hygiene.\n//!\n//! Periodically prunes old leases and audit log entries to keep the database lean.` |
| `struct MaintenanceService` | `/// Autonomous background worker that prunes stale data from the database.\n///\n/// Runs every hour. Deletes released/expired/revoked leases older than 30 days\n/// and audit log entries older than 90 days.` |
| `MaintenanceService::new()` | `/// Creates a new maintenance service connected to the given database pool.` |
| `MaintenanceService::run()` | `/// Starts the infinite maintenance loop. This method never returns under normal operation.` |

---

### 4.14 `src/infrastructure/mod.rs`
| Item | Doc Comment |
|---|---|
| Module-level `//!` | `//! Infrastructure adapters.\n//!\n//! Implements the domain repository traits (Ports) using concrete technologies.` |

---

### 4.15 `src/infrastructure/postgres_repository.rs`
| Item | Doc Comment |
|---|---|
| Module-level `//!` | `//! PostgreSQL-backed repository implementation.\n//!\n//! This module implements all domain repository traits using `sqlx` and PostgreSQL.\n//! The critical concurrency logic (`SELECT FOR UPDATE SKIP LOCKED`) lives here.` |
| `struct PostgresRepository` | `/// PostgreSQL implementation of all domain repository traits.\n///\n/// Wraps a `sqlx::PgPool` connection pool and translates between domain entities\n/// and database rows.` |
| `PostgresRepository::new()` | `/// Creates a new repository instance wrapping the given connection pool.` |

---

### 4.16 `src/api/mod.rs`
| Item | Doc Comment |
|---|---|
| Module-level `//!` | `//! API delivery layer (HTTP and gRPC).\n//!\n//! Contains Axum REST handlers, route definitions, the admin UI, and the Tonic gRPC service.` |

---

### 4.17 `src/api/routes.rs`
| Item | Doc Comment |
|---|---|
| Module-level `//!` | `//! Axum router and shared application state.` |
| `struct AppState` | `/// Shared application state injected into all Axum handlers via `State` extractor.\n///\n/// Holds `Arc`-wrapped application services for thread-safe concurrent access.` |
| `fn create_router()` | `/// Builds the Axum router with all REST and admin UI routes.` |

---

### 4.18 `src/api/handlers.rs`
| Item | Doc Comment |
|---|---|
| Module-level `//!` | `//! REST API request handlers.\n//!\n//! These handlers are intentionally "anemic" — they only handle HTTP-specific concerns\n//! (deserialization, status code mapping) and delegate all business logic to application services.` |
| `struct CreatePoolRequest` | `/// JSON request body for creating a new resource pool.` |
| `struct RegisterResourceRequest` | `/// JSON request body for registering a resource into a pool.` |
| `struct AllocateRequest` | `/// JSON request body for requesting a lease allocation.` |
| `struct RenewRequest` | `/// JSON request body for extending an active lease.` |
| `fn health_check()` | `/// Returns an HTML health status indicator. Used by monitoring and the admin UI.` |
| `fn create_pool()` | `/// `POST /pools` — Creates a new resource pool.` |
| `fn get_pool_by_name()` | `/// `GET /pools/:name` — Looks up a pool by name. Currently returns 501 Not Implemented.` |
| `fn register_resource()` | `/// `POST /resources` — Registers a new resource into an existing pool.` |
| `fn allocate_lease()` | `/// `POST /leases` — Allocates a resource lease. Returns 409 if the pool is exhausted.` |
| `fn renew_lease()` | `/// `PATCH /leases/:id/renew` — Extends an active lease's TTL.` |
| `fn release_lease()` | `/// `DELETE /leases/:id` — Releases an active lease, returning the resource to the pool.` |

---

### 4.19 `src/api/ui.rs`
| Item | Doc Comment |
|---|---|
| Module-level `//!` | `//! Admin dashboard UI handlers.\n//!\n//! Renders server-side HTML using Askama templates with HTMX for partial page updates.` |
| `struct AdminTemplate` | `/// Askama template context for the main admin dashboard page.` |
| `fn admin_dashboard()` | `/// `GET /` and `GET /admin` — Renders the admin dashboard with live platform statistics.` |
| `struct AuditLogTemplate` | `/// Askama template context for the audit log partial (HTMX fragment).` |
| `fn audit_log_stream()` | `/// `GET /admin/audit-log` — Returns an HTML fragment of the latest audit log entries for HTMX polling.` |

---

### 4.20 `src/api/grpc.rs`
| Item | Doc Comment |
|---|---|
| Module-level `//!` | `//! gRPC service implementation using Tonic.\n//!\n//! Provides a high-speed binary RPC interface for data center integrations\n//! alongside the REST API.` |
| `mod atropos_v1` | `/// Auto-generated Protobuf types and service traits from `proto/atropos.proto`.` |
| `struct GrpcAllocationService` | `/// Tonic gRPC implementation of the `AllocationService` RPC.\n///\n/// Delegates to the same application-layer `AllocationService` used by the REST API.` |
| `GrpcAllocationService::new()` | `/// Creates a new gRPC service wrapping the given application service.` |

---

## TASK 5: Fix Placeholder URLs in `README.md`

**File:** `README.md`

Make the following exact replacements:

1. **Line 5** — CI badge URL:
   - Old: `https://github.com/your-org/resource-allocator/actions/workflows/ci.yml/badge.svg`
   - New: `https://github.com/your-org/atropos/actions/workflows/ci.yml/badge.svg`
   - Also fix the link target: `https://github.com/your-org/resource-allocator/actions/workflows/ci.yml` → `https://github.com/your-org/atropos/actions/workflows/ci.yml`

2. **Line 72** — Clone URL:
   - Old: `git clone https://github.com/your-org/resource-allocator.git`
   - New: `git clone https://github.com/your-org/atropos.git`

3. **Line 73** — cd command:
   - Old: `cd resource-allocator`
   - New: `cd atropos`

> Note: Keep `your-org` as a placeholder — the maintainer will replace it with their actual GitHub org/username. The key fix is `resource-allocator` → `atropos` to match the actual `Cargo.toml` package name.

---

## TASK 6: Fix Dockerfile Binary Name

**File:** `Dockerfile`

The `Cargo.toml` package name is `atropos`, so the compiled binary is `atropos`, not `resource-allocator`.

1. **Line 29** — Change:
   ```dockerfile
   COPY --from=builder /app/target/release/resource-allocator .
   ```
   to:
   ```dockerfile
   COPY --from=builder /app/target/release/atropos .
   ```

2. **Line 36** — Change:
   ```dockerfile
   CMD ["./resource-allocator"]
   ```
   to:
   ```dockerfile
   CMD ["./atropos"]
   ```

---

## TASK 7: Create GitHub Issue and PR Templates

### 7.1 Bug Report Template

**File:** `.github/ISSUE_TEMPLATE/bug_report.md`

```markdown
---
name: Bug Report
about: Report a bug to help us improve Atropos
title: "[BUG] "
labels: bug
assignees: ''
---

## Describe the Bug
A clear and concise description of what the bug is.

## To Reproduce
Steps to reproduce the behavior:
1. Send request to '...'
2. With payload '...'
3. Observe error '...'

## Expected Behavior
A clear and concise description of what you expected to happen.

## Environment
- **Atropos version:** [e.g., 0.1.0]
- **Rust version:** [e.g., 1.75.0]
- **PostgreSQL version:** [e.g., 15.4]
- **OS:** [e.g., Ubuntu 22.04, Windows 11]
- **Deployment:** [e.g., Docker, bare metal, Kubernetes]

## Logs / Error Output
```
Paste relevant log output here.
```

## Additional Context
Add any other context about the problem here.
```

### 7.2 Feature Request Template

**File:** `.github/ISSUE_TEMPLATE/feature_request.md`

```markdown
---
name: Feature Request
about: Suggest a new feature or improvement for Atropos
title: "[FEATURE] "
labels: enhancement
assignees: ''
---

## Problem Statement
A clear description of the problem this feature would solve. Ex: "I'm always frustrated when [...]"

## Proposed Solution
A clear description of what you want to happen.

## Alternatives Considered
A description of any alternative solutions or features you've considered.

## Additional Context
Add any other context, diagrams, or references about the feature request here.
```

### 7.3 Pull Request Template

**File:** `.github/PULL_REQUEST_TEMPLATE.md`

```markdown
## Description
Brief description of the changes in this PR.

## Related Issue
Closes #(issue number)

## Type of Change
- [ ] Bug fix (non-breaking change that fixes an issue)
- [ ] New feature (non-breaking change that adds functionality)
- [ ] Breaking change (fix or feature that would cause existing functionality to change)
- [ ] Documentation update
- [ ] Refactoring (no functional changes)

## Checklist
- [ ] My code follows the project's coding standards (SOLID, strong typing, no primitive obsession)
- [ ] I have added `///` doc comments to all new public items
- [ ] I have added tests that prove my fix is effective or my feature works
- [ ] `cargo fmt --all` passes
- [ ] `cargo clippy -- -D warnings` passes
- [ ] `cargo test` passes
- [ ] I have updated `README.md` if this changes the public API
- [ ] I have updated `CHANGELOG.md` under `[Unreleased]`
```

---

## TASK 8: Update `CONTRIBUTING.md` — Reference New Files

**File:** `CONTRIBUTING.md`

Add the following after the existing "Code of Conduct" section (line 6):

```markdown
Please read it carefully: [CODE_OF_CONDUCT.md](../../docs/policy/CODE_OF_CONDUCT.md).

For security-related issues, please see our [Security Policy](../../docs/policy/SECURITY.md) — **do not** open a public issue for vulnerabilities.
```

The existing line `This project adheres to the [Rust Code of Conduct](https://www.rust-lang.org/policies/code-of-conduct).` should be replaced with:
```markdown
This project adheres to the [Contributor Covenant Code of Conduct](../../docs/policy/CODE_OF_CONDUCT.md).
```

---

## TASK 9: Update `README.md` — Reference New Files

**File:** `README.md`

### 9.1 Add Security & Code of Conduct links to the documentation section

After the existing `[Disaster Recovery Runbook](docs/project/RUNBOOK.md)` bullet, add:
```markdown
*   [Security Policy](docs/policy/SECURITY.md)
*   [Code of Conduct](docs/policy/CODE_OF_CONDUCT.md)
*   [Changelog](docs/project/CHANGELOG.md)
```

### 9.2 Add a "Security" section before the "License" section

```markdown
## 🔒 Security

If you discover a security vulnerability, please see our [Security Policy](docs/policy/SECURITY.md) for responsible disclosure instructions. **Do not** open a public GitHub issue for security concerns.
```

---

## Execution Order

1. TASK 1 — `SECURITY.md`
2. TASK 2 — `CODE_OF_CONDUCT.md`
3. TASK 3 — `CHANGELOG.md`
4. TASK 4 — Doc comments (all `src/**/*.rs` files)
5. TASK 5 — Fix README placeholder URLs
6. TASK 6 — Fix Dockerfile binary name
7. TASK 7 — GitHub templates
8. TASK 8 — Update `CONTRIBUTING.md`
9. TASK 9 — Update `README.md` with new links and security section

## Validation

After all changes, run:
```bash
cargo doc --no-deps --document-private-items 2>&1
cargo fmt --all -- --check
cargo clippy -- -D warnings
```

All three must pass with zero warnings.

