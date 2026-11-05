## ADDED Requirements

### Requirement: Alarm zones in the simulation
The golem alarm's zone check SHALL run in the simulation's once-a-frame work after the rise and its restore, alike with and without the bridge.

#### Scenario: The player crosses an alarm zone
- **WHEN** the player, without the zone's pass, moves into an alarm zone
- **THEN** the alarm rings at the same tick with and without the bridge
