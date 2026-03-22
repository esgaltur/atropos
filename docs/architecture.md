# System Architecture

## Overview
This platform acts as a high-performance Control Plane for managing scarce or shared resources (such as GPUs, large contiguous memory blocks, software licenses, or test environments). It uses a time-bound lease model with strong strict-serializable consistency to guarantee zero double-bookings.

## Hexagonal Architecture (Ports & Adapters)
We employ a strict Hexagonal Architecture to ensure the business logic is entirely decoupled from the delivery mechanism (HTTP/Axum) and the persistence layer (PostgreSQL).

### 1. Domain Layer (`src/domain/`)
The absolute core of the system.
*   **Entities:** `Pool`, `Resource`, `Lease`.
*   **Value Objects:** We use the Newtype pattern (e.g., `PoolId(Uuid)`) to prevent primitive obsession and ensure type safety.
*   **Interfaces (Ports):** Traits like `AllocationRepository` define *what* the system needs to store, not *how*.
*   **Rules:** Contains pure functions (e.g., `Lease::is_expired(timestamp)`) that are 100% deterministic and unit-testable without mocking a database.

### 2. Application Layer (`src/application/`)
The orchestrator of use cases.
*   **Services:** `AllocationService`, `PoolService`, `ResourceService`.
*   **Role:** These services take raw inputs, invoke the necessary Domain entities, enforce business rules (like Preemption or Quotas), and orchestrate the infrastructure repositories.
*   **Background Tasks:** The `ReaperService` lives here. It is an autonomous agent that runs on a `tokio::time::interval` to enforce the TTL lifecycle, returning expired resources to the available pool.

### 3. Infrastructure Layer (`src/infrastructure/`)
The implementation of the outward-facing Ports.
*   **PostgresRepository:** Implements the `AllocationRepository` trait. 
*   **Concurrency:** This layer houses the critical `SELECT ... FOR UPDATE SKIP LOCKED` logic. It encapsulates the SQL dialects and driver-specific (`sqlx`) errors, translating them into generic `DomainError` types.

### 4. API Layer (`src/api/`)
The delivery mechanism.
*   **Axum:** We use Axum for high-throughput async HTTP routing.
*   **Handlers:** These are intentionally "anemic". They handle HTTP semantics, convert domain objects to shared contract DTOs, and map `DomainError` to the correct HTTP status codes.
*   **UI:** The admin dashboard is server-side rendered through Leptos SSR in the dedicated `frontend/` crate, with Axum serving the resulting HTML.

## Concurrency Model
The platform is designed to handle "Thundering Herd" scenarios where hundreds of clients simultaneously request a single available resource.
Instead of using application-level Mutexes (which create bottlenecks) or Redis distributed locks (which add network hops and complexity), we push the concurrency control to the database using `SKIP LOCKED`.

When a request arrives:
1. It enters a transaction.
2. It attempts to find a row in `resources` that matches the criteria and has no active `leases`.
3. If multiple requests hit the exact same row, the first request locks it.
4. The subsequent requests *skip* that locked row instantly and move to the next available one.
5. If no rows are left, they immediately return `NoResourcesAvailable` (HTTP 409) rather than blocking the connection pool.

This guarantees linearizability and maximum throughput.
