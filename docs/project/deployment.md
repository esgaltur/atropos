# Deployment & Operations Guide

## Production Readiness
This project is compiled as a static, optimized binary. It is designed to be deployed as a containerized microservice.

### Docker Multi-Stage Build
The provided `infra/Dockerfile` uses a two-stage approach:
1.  **Builder (`rust:1.75-slim-bookworm`):** Compiles the heavy dependency tree.
2.  **Runtime (`debian:bookworm-slim`):** Copies *only* the compiled binary and necessary SSL certificates.

**Result:** A highly secure, minimal attack surface image.

### Environment Variables
The application requires the following environment variables:
*   `DATABASE_URL`: Standard Postgres connection string. Example: `postgres://user:pass@host:5432/dbname`
*   `RUST_LOG`: Controls logging verbosity. Set to `info` for production, `debug` or `trace` for troubleshooting.

## High Availability & Scaling
The API nodes are **100% Stateless**. 

You can horizontally scale the application by running multiple instances behind a standard Load Balancer (AWS ALB, NGINX, HAProxy).
Because the concurrency control (`SKIP LOCKED`) is handled entirely by the PostgreSQL database engine, running 10 API instances will never result in a split-brain double-booking scenario.

### Database Tuning
For production, ensure your PostgreSQL instance is tuned for high concurrency:
*   `max_connections`: Ensure this is set high enough to accommodate the `PgPoolOptions` configuration in `src/main.rs` multiplied by the number of API nodes. (e.g., 5 nodes * 100 pool size = minimum 500 `max_connections` on the DB).
*   Consider using **PgBouncer** if you are deploying a massive number of API replicas.

## Observability

### Health Checks
A lightweight `GET /health` endpoint is available for Kubernetes Liveness and Readiness probes.

### Prometheus Metrics
The application exposes an OpenTelemetry/Prometheus metrics exporter on port `9000`.
*   **Endpoint:** `http://<node-ip>:9000/metrics`
*   **Key Metrics:**
    *   `reclaim_count`: A counter tracking how many expired leases the background Reaper service has forcefully terminated.

Configure your Prometheus `scrape_configs` to hit this port.
