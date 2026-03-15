# Implementation Plan: waitlist_fulfillment_20260315

## Phase 1: Repository Layer Enhancement [checkpoint: 3ecfd90]
- [x] Task: Implement `fulfill_next_waitlist_entry` in `PostgresRepository`. (76758c4)
    - [x] Write Failing Tests: Create an integration test that seeds a waitlist entry and an available resource, then verifies fulfillment.
    - [x] Implement Feature: Implement atomic waitlist fulfillment logic using `FOR UPDATE SKIP LOCKED`.
    - [x] Verify: Ensure tests pass and audit logs are correctly generated.
- [x] Task: Conductor - User Manual Verification 'Phase 1: Repository Enhancement' (Protocol in workflow.md) (3ecfd90)

## Phase 2: Reaper Service Integration [checkpoint: 476e8ba]
- [x] Task: Integrate waitlist fulfillment into the `ReaperService` reclamation cycle. (33b965e)
    - [x] Write Failing Tests: Update reaper integration tests to verify that reclaimed resources are immediately assigned to waitlisted users.
    - [x] Implement Integration: Update `ReaperService` to call fulfillment logic after successful reclamation.
    - [x] Verify: Ensure the end-to-end flow works under load.
- [x] Task: Conductor - User Manual Verification 'Phase 2: Reaper Integration' (Protocol in workflow.md) (476e8ba)

## Phase 3: Manual Release Integration
- [x] Task: Integrate waitlist fulfillment into the manual release flow. (5185865)
    - [x] Write Failing Tests: Verify that calling `release_lease` automatically fulfills the next waitlist entry.
    - [x] Implement Integration: Update `AllocationService::release` to trigger fulfillment.
    - [x] Verify: Ensure no regressions in manual release performance.
- [ ] Task: Conductor - User Manual Verification 'Phase 3: Manual Release Integration' (Protocol in workflow.md)
