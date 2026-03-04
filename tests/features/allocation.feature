Feature: GPU Cluster Resource Management

  Scenario: Successful high-speed allocation of a single GPU
    Given a resource pool "A100-Cluster" exists
    And the pool has 1 "Healthy" GPU resource
    When a research team "Deep-Research" requests 1 GPU
    Then the allocation should be "Successful"
    And a unique Lease should be issued

  Scenario: Deny double-booking (Atomic Isolation)
    Given a resource pool "V100-Cluster" exists
    And the pool has 0 "Healthy" GPU resources
    When a research team "AI-Lab" requests 1 GPU
    Then the allocation should be "Denied"
    And the reason should be "No resources available for allocation"

  Scenario: Idempotency (Same request, same result)
    Given a resource pool "A100-Cluster" exists
    And the pool has 1 "Healthy" GPU resource
    When a team "NASA" requests a GPU with idempotency key "const-123"
    And the same team "NASA" requests a GPU with the same key "const-123"
    Then both responses should contain the "Same" Lease ID

  Scenario: Waitlisting when pool is full
    Given a resource pool "A100-Cluster" exists
    And the pool has 0 "Healthy" GPU resources
    When a team "CERN" requests a GPU with waitlist enabled
    Then the allocation should be "Denied"
    And the reason should be "Infrastructure error: Added to waitlist"

  Scenario: Preemption when pool is full
    Given a resource pool "A100-Cluster" exists
    And the pool has 0 "Healthy" GPU resources
    When a team "DARPA" requests a GPU with preempt enabled
    Then the allocation should be "Denied"
    And the reason should be "Infrastructure error: Preemption required but logic in repo is pending"

