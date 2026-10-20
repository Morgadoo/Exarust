## ADDED Requirements

### Requirement: Creation list step
Stepping SHALL rebuild the list at the index less one when stepping back and the index is positive, and at the index plus one when stepping forward. The index SHALL be returned with whether the rebuild ran.

#### Scenario: Back, forward and blocked
- **WHEN** the index is 2 and the step is back
- **THEN** the rebuild runs at 1
- **WHEN** the index is 0 and the step is back
- **THEN** no rebuild runs
- **WHEN** the index is 2 and the step is forward
- **THEN** the rebuild runs at 3
