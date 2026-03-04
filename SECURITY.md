# Security Policy

## Supported Versions

The following versions of Atropos are currently supported with security updates:

| Version | Supported          |
| ------- | ------------------ |
| 0.1.x   | :white_check_mark: |

Note: Only the latest minor release receives security patches. Users are encouraged to upgrade to the latest version to ensure they have the most recent security fixes.

## Reporting a Vulnerability

**Please do NOT open a public GitHub Issue for security vulnerabilities.**

If you discover a potential security vulnerability in Atropos, please report it privately via email to: `security@esgaltur.dev`.

When reporting a vulnerability, please include the following information:

- A detailed description of the vulnerability.
- Clear steps to reproduce the issue (including proof-of-concept code if possible).
- The affected version(s) of Atropos.
- The potential impact of the vulnerability.

We commit to acknowledging receipt of your report within **48 hours** and providing a preliminary timeline for a fix within **7 days**.

## Disclosure Policy

Atropos follows a **Coordinated Disclosure** policy. We ask that you give us a reasonable amount of time to address the vulnerability before making any information public. Typically, this is 90 days after the initial report, though this may be adjusted by mutual agreement.

Security advisories will be published via GitHub Security Advisories once a fix is available and the disclosure date is reached.

## Scope

### In Scope
- The `atropos` binary.
- The REST API (`/pools`, `/resources`, `/leases`, `/health`).
- The gRPC API (port 50051).
- The PostgreSQL repository layer (`src/infrastructure/postgres_repository.rs`).
- The official Docker image.

### Out of Scope
- Demo scripts (`demo.ps1`, `verify_full.ps1`).
- Load testing scripts (`load_test.js`).
- Third-party dependencies (please report these to their respective upstream maintainers).
- Local development environment configurations (e.g., `docker-compose.yml` for local dev).
