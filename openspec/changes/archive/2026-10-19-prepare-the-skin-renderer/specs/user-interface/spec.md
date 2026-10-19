## ADDED Requirements

### Requirement: Skin renderer preparation
When the global guard is not 1, preparation SHALL make two setup calls and return 1. When the guard is already 1, it SHALL make no calls and return 1.

#### Scenario: The first preparation calls setup
- **WHEN** preparation runs with a clear guard
- **THEN** the setup call count is 2 and the result is 1

#### Scenario: A repeated preparation skips setup
- **WHEN** preparation runs with the guard already set
- **THEN** the setup call count is 0 and the result is 1
