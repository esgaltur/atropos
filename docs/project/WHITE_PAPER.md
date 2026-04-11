# Atropos: High-Performance Atomic Resource Leasing via Strict-Serializable Concurrency

**Abstract**  
This paper introduces Atropos, a high-performance orchestration platform designed for the allocation of scarce, shared resources in distributed environments. Atropos leverages the PostgreSQL `SKIP LOCKED` concurrency primitive to achieve non-blocking, atomic resource claims with strict serializability guarantees. By decoupling lifecycle management (via an autonomous Reaper Service) from the allocation request path, Atropos minimizes tail latency and maximizes throughput in "Thundering Herd" scenarios.

## 1. Introduction
In modern cloud computing and high-performance computing (HPC), the management of scarce resources—such as NVIDIA H100 GPUs, contiguous memory blocks, or high-cost software licenses—presents a critical scheduling challenge. Traditional "Lazy" allocation models often lead to double-bookings or resource starvation, while traditional locking mechanisms (Mutexes, Redis-based distributed locks) introduce significant latency and operational complexity.

## 2. The Thundering Herd Problem
When multiple tenants simultaneously request a single available resource, a "race condition" occurs.
*   **Naive Approach:** Application-level checking of availability followed by a write. This requires complex distributed locks.
*   **The Atropos Approach:** Utilize the database engine as a linearizable queue.

## 3. Concurrency Mechanism: `SKIP LOCKED`
Atropos utilizes the `SELECT ... FOR UPDATE SKIP LOCKED` primitive. This allows concurrent transactions to skip rows that are currently locked by other transactions. 
*   **Linearizability:** Guarantees that every request observes the same state transition order.
*   **Performance:** Transitions the system from a $O(N)$ blocking wait to a $O(1)$ constant-time skip operation for failed requests.

## 4. Architectural Elegance
Atropos implements a **Hexagonal (Ports and Adapters) Architecture**. This ensures that the core domain logic (Lease lifecycle, preemption policies) is isolated from infrastructure side-effects.
*   **Domain Layer:** Pure Rust business rules.
*   **Application Layer:** Use-case orchestration.
*   **Infrastructure Layer:** SQLx-based Postgres persistence.

## 5. Lifecycle Orchestration: The Reaper & Outbox
To prevent resource leakage, Atropos employs an autonomous background Reaper service. This service provides deterministic reclamation of expired leases and handles "Zombie" client detection via a liveness heartbeating mechanism.

Furthermore, Atropos utilizes the **Transactional Outbox Pattern** to provide reliable event notifications. Every state transition (Allocation, Preemption, Expiration) is persisted to an `outbox_events` table within the same transaction as the resource claim, guaranteeing "At-Least-Once" delivery to downstream webhooks or message brokers.

## 6. Advanced Scheduling: Priority, Quotas, and Anti-Affinity
Atropos extends basic pooling with sophisticated governance:
*   **Priority-Based Preemption:** Implements a multi-stage atomic transaction that can revoke active low-priority leases to fulfill high-priority requests if the pool is saturated.
*   **Attribute Routing:** Leverages PostgreSQL JSONB operators to filter candidate resources against arbitrary client-provided constraints.
*   **Tenant Quotas:** Enforces per-tenant concurrency limits within the allocation path to prevent resource monopolization.
*   **Waitlist Aging:** Mitigates low-priority starvation by dynamically increasing the "Effective Priority" of tasks based on their queue duration.
*   **Soft Anti-Affinity:** Optimizes workload distribution across physical failure domains (e.g., racks) by prioritizing resources in unused zones.

## 7. Conclusion
Atropos represents a leap forward in resource orchestration, moving away from fragile application-level synchronization toward database-native atomic operations coupled with advanced governance. Its design optimizes for high-concurrency data center workloads where performance, consistency, and fairness are non-negotiable.
