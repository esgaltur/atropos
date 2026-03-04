Feature: Observability & Audit

  Scenario: Retrieval of aggregate resource statistics
    Given the resource pool "A100-Cluster" exists with 2 healthy resources
    And there is 1 active lease for "Team-Alpha"
    When the administrator requests summary statistics
    Then the active lease count should be 1
    And the total healthy resource count should be 2
    And the waitlist count should be 0

  Scenario: Retrieval of recent audit log entries
    Given the resource pool "A100-Cluster" exists with 1 healthy resource
    When a research team "Team-Beta" allocates a GPU
    And the administrator requests recent audit logs
    Then the latest audit log should show "ALLOCATE"
    And the logs should contain at least 1 entry
