# Atropos 🚀

[![Rust](https://img.shields.io/badge/rust-1.75%2B-blue.svg)](https://www.rust-lang.org)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)
[![Rust CI](https://github.com/esgaltur/atropos/actions/workflows/ci.yml/badge.svg)](https://github.com/esgaltur/atropos/actions/workflows/ci.yml)
[![Architecture: Hexagonal](https://img.shields.io/badge/Architecture-Hexagonal-green.svg)](#architecture)

A high-performance, strictly consistent **Resource Leasing & Capacity Orchestration Platform** built in Rust. It enables teams to allocate, reserve, and govern scarce or shared resources (e.g., GPUs, license tokens, test environments) with strong linearizable guarantees.

---

### 📖 [Read the User & Use Case Guide](USER_GUIDE.md)
*Learn why Atropos exists, how to use it for GPU clusters, CI/CD farms, and license management.*

---

## 🌟 Key Features

*   **Atomic Allocation:** Zero double-bookings using PostgreSQL `SELECT FOR UPDATE SKIP LOCKED`.
*   **Time-Bound Leases:** Automated resource reclamation via TTL (Time-To-Live).
*   **Hexagonal Architecture:** Clean separation between Domain logic and Infrastructure (Postgres/Axum).
*   **Type-Safe Domain:** Newtype patterns for IDs and strong enums for state transitions.
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

*   [System Architecture & Design](docs/architecture.md)
*   [Deployment & Scaling Guide](docs/deployment.md)
*   [ADR 0001: Why we use Postgres SKIP LOCKED](docs/adr/0001-postgres-skip-locked.md)
*   [ADR 0002: Hexagonal Architecture](docs/adr/0002-hexagonal-architecture.md)
*   [ADR 0003: The Background Reaper](docs/adr/0003-reaper-service.md)
*   [Disaster Recovery Runbook](RUNBOOK.md)

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
    docker-compose up -d
    ```

3.  **Setup Environment:**
    ```bash
    cp .env.example .env
    # Adjust DATABASE_URL in .env if necessary
    ```

4.  **Run Migrations:**
    ```bash
    sqlx migrate run
    ```

5.  **Run the Server:**
    ```bash
    cargo run
    ```

---

## 📖 API Documentation

### Allocate a Lease
`POST /leases`
```json
{
  "pool_type": "A100-Cluster",
  "owner_id": "team-alpha",
  "tenant_id": "project-omega",
  "ttl_seconds": 3600
}
```

### Release a Lease
`DELETE /leases/:id`

---

## 🗺 Roadmap

- [x] **Milestone 1:** Core REST API & Postgres Persistence (Atomic Allocation).
- [x] **Milestone 2:** Lifecycle Reaper (Background worker for TTL reclamation) & Metrics.
- [x] **Milestone 3:** Waitlisting logic, Admin UI, & K8s CRD.
- [x] **Milestone 4:** Runbook for DR, Load testing, and fully operational release.

---

## 🤝 Contributing

We welcome contributions! Please see [CONTRIBUTING.md](CONTRIBUTING.md) for our coding standards and pull request process.

---

## 📄 License

Distributed under the MIT License. See `LICENSE` for more information.
