# BUILD STAGE
FROM rust:1.75-slim-bookworm AS builder

# Install build dependencies
RUN apt-get update && apt-get install -y pkg-config libssl-dev && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# 1. Pre-build dependencies for caching
COPY Cargo.toml Cargo.lock ./
RUN mkdir src && echo "fn main() {}" > src/main.rs
RUN cargo build --release
RUN rm -rf src

# 2. Build the actual application
COPY . .
RUN cargo build --release

# RUNTIME STAGE
FROM debian:bookworm-slim

# Install runtime dependencies (OpenSSL is needed for SQLx)
RUN apt-get update && apt-get install -y libssl3 ca-certificates && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Copy binary from builder
COPY --from=builder /app/target/release/atropos .
COPY migrations ./migrations

# Expose the API port
EXPOSE 3000

# Standard entrypoint
CMD ["./atropos"]
