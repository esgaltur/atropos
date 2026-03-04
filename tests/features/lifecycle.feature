Feature: Lease Lifecycle Management

  Scenario: Renewing an active lease
    Given a research team has an "ACTIVE" lease for a GPU
    When they request a renewal for 60 seconds
    Then the renewal should be "Successful"

  Scenario: Releasing an active lease
    Given a research team has an "ACTIVE" lease for a GPU
    When they release the active lease
    Then the release should be "Successful"

  Scenario: Attempting to release a non-existent lease
    Given no active lease exists for ID "404-Lease"
    When they attempt to release a non-existent lease
    Then the result should be "Not Found"
    And the reason should be "Lease not found"
