## ADDED Requirements

### Requirement: No step after a teardown
With the free mode on, a level's teardown SHALL first hold the simulation and apply every step it took before the hold, and SHALL keep it held until the next level is in; no step value SHALL despawn or replace an entity or resource the frame made after the step began.

#### Scenario: A teardown while the simulation steps
- **WHEN** the simulation steps entities and the frame tears them down and spawns new ones at the same indices
- **THEN** the new entities and resources survive, the old ones stay gone, and the steps go on without a gap after the release
