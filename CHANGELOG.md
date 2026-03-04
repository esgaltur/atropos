# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

- No unreleased changes.

## [0.1.0] - 2026-03-03

### Added
- **Core Persistence:** Atomic resource allocation using PostgreSQL `SKIP LOCKED` for linearizable guarantees.
- **Hexagonal Architecture:** Strict isolation between Domain, Application, Infrastructure, and API layers.
- **Autonomous Reaper Service:** Background task for automated TTL-based resource reclamation.
- **Waitlisting:** Durable, database-backed queuing for oversubscribed resource pools.
- **gRPC Interface:** High-performance binary RPC layer using Tonic and Protobuf.
- **Admin Dashboard:** Modern web interface built with HTMX and Tailwind CSS for real-time monitoring.
- **Observability:** Integrated Prometheus metrics and structured tracing (OpenTelemetry).
- **Maintenance Service:** Automated background pruning of historical leases and audit logs.
- **Type Safety:** Newtype pattern for all domain IDs to prevent primitive obsession.

### Changed
- **Database Schema:** Replaced strict unique constraints on resource leases with partial indexes to allow unlimited historical records while maintaining single-active-lease integrity.
- **Docker Image:** Optimized multi-stage build process for production-grade deployments.

### Fixed
- **Unique Constraint Bug:** Resolved issue preventing resource reuse after lease expiration.
- **API Spec Mismatch:** Fixed `AllocateRequest` parameter inconsistencies.
- **Concurrency Races:** Hardened allocation logic verified via high-concurrency stress tests.
