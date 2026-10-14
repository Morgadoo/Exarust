## ADDED Requirements

### Requirement: Stature
The system SHALL scale a clip's root position by the posed body's leg length (thigh plus shin) relative to the human body's, so bodies of every race stand on the floor.

#### Scenario: Golem
- **WHEN** `exarust --race 16 character Idle` runs
- **THEN** the golem stands with its feet on the floor
