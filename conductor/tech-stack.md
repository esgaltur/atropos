# Atropos Technology Stack

## Core Language & Runtime
- **Rust (1.75+):** High-performance, memory-safe systems programming language.
- **Tokio:** Event-driven, non-blocking I/O platform for building asynchronous applications.

## API & Web
- **Axum:** Ergonomic and modular web framework built on top of `tokio` and `tower`.
- **HTMX + Askama:** Lightweight, server-side rendered dashboard components for real-time monitoring without heavy JavaScript.

## Database & Persistence
- **PostgreSQL:** Reliable relational database for consistent resource state management.
- **SQLx:** Async, compile-time checked SQL toolkit for Rust.
- **`SKIP LOCKED`:** Primitive for atomic, linearizable resource allocation without global application locks.

## Architecture
- **Hexagonal Architecture:** Clean separation of concerns (Domain, Application, Infrastructure, API).
- **Domain-Driven Design (DDD):** Modeling the core business logic in a pure domain layer.
- **Newtype Pattern:** Strong type safety for all entity IDs and resource states.

## Observability & Quality
- **Tracing:** Structured logging and distributed tracing with OpenTelemetry.
- **Prometheus:** High-precision metrics for capacity and allocation performance.
- **Cucumber:** Behavior-Driven Development (BDD) for high-level feature validation.
