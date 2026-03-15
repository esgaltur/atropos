# Initial Concept
Atropos is a high-performance resource orchestration platform that provides strictly consistent resource leasing and capacity management.

# Product Definition
Atropos is a high-performance resource orchestration platform that provides strictly consistent resource leasing and capacity management. It's designed to ensure zero double-bookings of scarce or shared resources (e.g., GPU clusters, CI/CD farms, license tokens) using strong linearizable guarantees.

## Vision
Atropos is a high-performance resource orchestration platform that provides strictly consistent resource leasing and capacity management. It's designed to ensure zero double-bookings of scarce or shared resources (e.g., GPU clusters, CI/CD farms, license tokens) using strong linearizable guarantees.

## Target Audience
- **DevOps/Infrastructure Teams:** Managing shared computing or physical resources.
- **SaaS Providers:** Offering metered or reserved resource access.
- **Enterprise Platforms:** Governing internal shared services and license compliance.

## Core Value Proposition
- **Guaranteed Consistency:** Atomic allocation using PostgreSQL `SKIP LOCKED` ensures resources are never over-allocated.
- **Automated Lifecycle:** Built-in reaper service reclaims expired leases and automatically fulfills waitlists, ensuring maximum resource utilization.
- **Architectural Rigor:** Hexagonal architecture ensures the system is maintainable, testable, and future-proof.
- **Operational Visibility:** Native Prometheus metrics and structured tracing for deep observability.

## Key Features
- **Atomic Resource Allocation:** Linearizable allocation of pool resources.
- **Time-Bound Leasing:** Automated reclamation of resources via TTL.
- **Automatic Waitlist Fulfillment:** Instant reallocation of freed resources to high-priority waitlisted users during reclamation or manual release.
- **Hexagonal Architecture:** Strict separation between domain logic and infrastructure.
- **Type-Safe Domain:** Newtype patterns for all entity IDs (`PoolId`, `ResourceId`, `LeaseId`).
- **High Concurrency Support:** Built with Rust, Tokio, and SQLx for scale.
- **Interactive Dashboard:** Lightweight HTMX + Askama for resource monitoring.

## Success Metrics
- **Zero Double-Bookings:** 100% consistency in resource allocation.
- **Reclamation Latency:** Expired leases are reclaimed within 60 seconds.
- **API Performance:** Allocation requests completed under 50ms (p95).
