# Spec Delta

## ADDED Requirements

### Requirement: Signalled open
A signalled open SHALL play the unlock sound once when there is at least one door, and SHALL play nothing when there are none. A latched or motorised door SHALL wait 120 ticks. A door that is neither SHALL not wait. While the countdown is at least 2 it SHALL fall by 1. Below 2 the wait SHALL end.

#### Scenario: One sound, then 120 ticks
- **WHEN** there are no doors, a door that is neither latched nor motorised is signalled, and a latched door is signalled and then stepped until the wait ends
- **THEN** the empty list plays nothing, the plain door plays the sound and does not wait, and the latched door ends its wait on the 120th tick
