# Leptos Full Switch Plan (Contract-First)

## Goal
Move from Askama templates to a full Leptos-based UI while keeping:
- Separate frontend and backend modules/projects.
- One deployable executable (Axum binary serves the Leptos SSR UI).
- Contract-first development with shared DTOs used by both backend and frontend.

## Current Direction (Confirmed)
- Shared contract crate: contracts (atropos_contracts)
- Frontend crate: frontend (atropos_frontend)
- Backend crate: root atropos binary
- Backend UI handler now calls frontend SSR renderers

## Target Architecture
1. contracts crate
- Owns request/response DTOs and UI data contracts.
- Backend handlers deserialize/serialize contract types.
- Frontend consumes same contract types for view models.

2. frontend crate
- Owns Leptos views and render functions.
- Exposes SSR rendering functions consumed by backend.
- No database or domain logic.

3. backend crate (root)
- Owns domain, application, infrastructure, API routes/handlers.
- Converts domain entities to contract DTOs.
- Serves HTTP API + SSR HTML through Axum.

## Scope of Full Switch
1. Remove Askama usage from dependencies and code.
2. Remove dependency on templates/admin.html runtime rendering path.
3. Keep route behavior equivalent:
- / and /admin render dashboard HTML.
- /admin/audit-log returns HTML fragment for refresh.
4. Keep one executable startup path in src/main.rs.

## Contract-First Rules
1. No ad-hoc handler structs in src/api/handlers.rs when equivalent contract type exists.
2. OpenAPI is the source of truth for REST contract.
3. Route methods must match OpenAPI (example: renew endpoint method alignment).
4. Domain-to-contract mapping happens at API boundary only.

## Implementation Plan

## Phase 1: Stabilize Shared Contracts
1. Keep all HTTP request DTOs in contracts/src/lib.rs.
2. Keep all HTTP response DTOs in contracts/src/lib.rs.
3. Keep UI DTOs there too (DashboardStats, AuditLogItem).
4. Add conversion helpers in backend API layer (not in domain layer).

Deliverable:
- Backend handlers can compile using contract request types.

## Phase 2: Backend Handler Refactor
1. Replace local handler request structs with atropos_contracts types.
2. Return contract response DTOs from handlers.
3. Ensure renew route and OpenAPI method are consistent.
4. Keep existing status code semantics.

Deliverable:
- src/api/handlers.rs uses contract-first types consistently.

## Phase 3: Leptos SSR Completion
1. Keep dashboard and audit log rendering in frontend/src/lib.rs.
2. Ensure backend ui handlers only map domain -> contract and call frontend render functions.
3. Remove remaining Askama code paths.

Deliverable:
- src/api/ui.rs has no template engine dependency.

## Phase 4: Remove Legacy Template Surface
1. Remove askama and askama_axum dependencies from root Cargo.toml.
2. Keep templates/admin.html only if intentionally archived; otherwise remove.
3. Update docs to indicate Leptos SSR is authoritative UI implementation.

Deliverable:
- No runtime dependency on Askama templates.

## Phase 5: Compile and Test to Green
1. Run cargo check and fix all compile errors.
2. Run cargo test and fix test breakages related to API signatures/contracts.
3. Validate startup and route smoke tests:
- GET /admin
- GET /admin/audit-log
- GET /health
- POST /leases
- DELETE /leases/{id}
- renew endpoint method defined in OpenAPI
4. Confirm the app remains single-executable.

Deliverable:
- Clean compile and passing tests.

## Operational Acceptance Criteria
1. Single binary run command still works: cargo run
2. No Askama compile/runtime dependency in active code path.
3. Frontend and backend are separate modules/crates.
4. API and frontend both consume atropos_contracts.
5. OpenAPI contract and route methods are aligned.

## Risks and Mitigations
1. Risk: Contract drift between OpenAPI and code.
- Mitigation: Add OpenAPI review check to PR checklist.

2. Risk: Duplicate DTOs reintroduced in handlers.
- Mitigation: Enforce contract-first guideline in CONTRIBUTING.

3. Risk: UI regressions in dashboard fragment refresh.
- Mitigation: Keep /admin/audit-log response snapshot test.

## Work Log Checklist
- [x] Created contracts crate and frontend crate.
- [x] Connected backend UI to frontend SSR renderers.
- [x] Refactor handlers to use contract types fully.
- [x] Align route method with OpenAPI for renew endpoint.
- [x] Remove any remaining Askama/template coupling.
- [x] cargo check clean.
- [ ] cargo test clean.

## Suggested Follow-up Files to Update After Green Build
1. README.md
2. docs/architecture.md
3. docs/api-guide.md
4. USER_GUIDE.md
