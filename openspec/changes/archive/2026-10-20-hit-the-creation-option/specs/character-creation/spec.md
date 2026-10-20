## ADDED Requirements

### Requirement: Creation option hit
Hitting SHALL be false for key 8 or a missing list; otherwise it SHALL be the hit result, and a hit SHALL select the last entry. The result SHALL be true.

#### Scenario: Hit, key 8 and no list
- **WHEN** the key is 3, the list is present and the hit test succeeds
- **THEN** the hit is true, the last entry is selected and the result is true
- **WHEN** the key is 8
- **THEN** nothing is hit or selected
- **WHEN** the list is missing
- **THEN** nothing is hit or selected

### Requirement: Creation singleton
The singleton SHALL be created when absent and returned otherwise.

#### Scenario: First and later
- **WHEN** the singleton is absent
- **THEN** the creator runs and the singleton is ready
- **WHEN** the singleton is present
- **THEN** the creator does not run and the singleton is ready
