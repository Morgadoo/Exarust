## ADDED Requirements

### Requirement: Removals survive values in flight
With the free mode on, a component one side removes SHALL stay removed when the other side sends a value for it that it wrote before it had seen the removal.

#### Scenario: A dead body's clip player
- **WHEN** the frame removes a dying body's clip player while the simulation's steps in flight still carry it
- **THEN** the clip player does not come back on either side, and the body lies as in lockstep
