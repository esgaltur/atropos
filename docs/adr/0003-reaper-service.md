# Architecture Decision Record: 0003
## Title: Autonomous Background Reaper Service

### Status
Accepted

### Context
Resources are allocated with a Time-To-Live (TTL). When the TTL expires, the resource must be returned to the pool. We need a mechanism to enforce this.

### Options Considered
1.  **Lazy Evaluation (On Read):** Don't actually "release" the lease, just check if it's expired whenever a new request comes in asking for a resource.
    *   *Drawback:* Complex SQL queries. Doesn't trigger physical cleanup actions if needed (e.g., resetting a physical GPU).
2.  **External Cron Job (Kubernetes CronJob / Celery):** Run a script every minute to hit a cleanup API.
    *   *Drawback:* Adds operational complexity and infrastructure dependencies.
3.  **In-Process Background Task:** Spawn an asynchronous `tokio` task inside the main API server that continuously polls the database.

### Decision
We will implement an **In-Process Background Task (`ReaperService`)**. It will run on an interval, executing a bulk `UPDATE` query to transition all expired `ACTIVE` leases to `RELEASED`.

### Consequences
*   **Positive:** Zero external dependencies. The system is entirely self-contained. Fast and responsive reclamation.
*   **Negative:** If running multiple API instances, multiple reapers will fire simultaneously. However, because it is a simple `UPDATE ... WHERE expires_at <= NOW()` query, Postgres handles the concurrency safely. (In a massive scale scenario, we might move this to a dedicated leader-elected worker, but for now, it is safe and efficient).
