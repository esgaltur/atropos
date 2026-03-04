# Architecture Decision Record: 0002
## Title: Hexagonal Architecture (Ports and Adapters)

### Status
Accepted

### Context
Enterprise software tends to degrade into "Big Ball of Mud" architectures over time as framework code (HTTP routers) and database code (SQL queries) bleed into the business logic. We need a structure that protects the core domain from external changes.

### Decision
We will enforce a strict **Hexagonal Architecture**.
1.  `src/domain`: Contains absolute pure Rust. No HTTP, no SQL. Defines the Interfaces (Ports).
2.  `src/application`: Orchestrates the domain.
3.  `src/infrastructure`: Implements the Ports (Adapters) using SQLx.
4.  `src/api`: Implements the delivery mechanism (Adapters) using Axum.

### Consequences
*   **Positive:** The core logic is highly testable. We can unit test lease expirations and allocations without standing up a database. We can easily swap out Axum for gRPC in the future.
*   **Negative:** Requires more boilerplate (mapping Domain Models to Database Models, defining Traits). Slightly higher cognitive load for junior developers unfamiliar with Dependency Inversion.
