# Architecture Decision Record: 0001
## Title: Use PostgreSQL `SKIP LOCKED` for Atomic Allocation

### Status
Accepted

### Context
The core requirement of this platform is to allocate scarce resources to competing tenants without ever double-booking. In a high-concurrency environment (the "Thundering Herd" problem), multiple API requests will simultaneously attempt to claim the same free resource.

### Options Considered
1.  **Application-Level Mutex:** Lock state in memory within the Rust app. 
    *   *Drawback:* Breaks the moment we horizontally scale to multiple API instances.
2.  **Redis Distributed Locks (Redlock):** Acquire a lock in Redis before modifying the database.
    *   *Drawback:* Adds a heavy external infrastructure dependency. High latency due to network hops. Potential for split-brain locking issues.
3.  **PostgreSQL `SELECT ... FOR UPDATE` (Standard):** 
    *   *Drawback:* Concurrent requests block and wait for the first transaction to finish. This exhausts the database connection pool rapidly and degrades throughput.
4.  **PostgreSQL `SELECT ... FOR UPDATE SKIP LOCKED`:** 
    *   *Benefit:* The database acts as a highly optimized concurrent queue.

### Decision
We will use **PostgreSQL `SKIP LOCKED`** within a single, atomic transaction to both find the free resource and insert the lease record.

### Consequences
*   **Positive:** Absolute guarantee against double-booking. Extremely high throughput as blocked queries instantly skip rather than waiting. Eliminates the need for Redis.
*   **Negative:** Ties our infrastructure layer specifically to PostgreSQL (or MySQL 8.0+), meaning a migration to a NoSQL database like DynamoDB would require completely rewriting the concurrency logic in the `AllocationRepository`. (Mitigated by Hexagonal Architecture).
