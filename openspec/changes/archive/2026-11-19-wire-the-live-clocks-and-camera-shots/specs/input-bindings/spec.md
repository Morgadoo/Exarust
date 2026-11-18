## ADDED Requirements

### Requirement: Pad deadlines on the simulation clock
The gamepad's step repeat and Powers hold SHALL run on the simulation's clock, while the set's priming waits on the real clock.

#### Scenario: A held simulation
- **WHEN** Powers is held while the simulation's clock stands
- **THEN** the power tree is not asked for however long the real clock runs
