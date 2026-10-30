# Spec Delta

## ADDED Requirements

### Requirement: Journey bubble step
A teleport journey's bubble SHALL be stepped once a tick, by its journey; the plain bubble tick SHALL leave it.

#### Scenario: The rest ends
- **WHEN** a journey's resting bubble passes its 500 ms rest on a tick
- **THEN** after that tick it is in the shrink's first phase, and only the next tick shrinks it
