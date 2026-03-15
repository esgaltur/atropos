# Track: Implement Automated Waitlist Fulfillment

## Overview
This track aims to automate the process of fulfilling waitlisted resource requests. Currently, when a lease expires or is released, the resource simply returns to the available pool. Users on the waitlist must manually retry or wait for a subsequent allocation request. This track will integrate waitlist fulfillment into the resource reclamation lifecycle.

## Objectives
- **Automatic Fulfillment**: When a resource is reclaimed by the `ReaperService` or released manually, the system should check for waiting users and automatically allocate the resource to the highest-priority entry.
- **Efficiency**: Use atomic database operations (e.g., `SKIP LOCKED` and transactions) to ensure consistency and prevent double-allocations.
- **Observability**: Track waitlist fulfillment events in the audit log and metrics.

## Technical Details
- **Logic Location**: The core logic should reside in the `infrastructure` layer (Postgres implementation) to ensure atomicity, orchestrated by the `application` layer (likely within `ReaperService` or a shared orchestration utility).
- **Key Files**: 
    - `src/domain/repository.rs` (Trait updates)
    - `src/infrastructure/postgres_repository.rs` (Implementation)
    - `src/application/reaper.rs` (Integration)
    - `src/application/allocation_service.rs` (Integration for manual releases)
