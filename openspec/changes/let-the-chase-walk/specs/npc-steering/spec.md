## ADDED Requirements

### Requirement: One AI state drives the walk
When combat, a flight or the scene takes a roaming character's body, a leg of its idle walk that is under way SHALL end with the soft stop once; otherwise the idle walk SHALL NOT touch the AI's driver, so a fighter's chase walks its route through it.

#### Scenario: A fighter chasing
- **WHEN** an engaged fighter's chase has its driver driving and its idle walk has no route
- **THEN** after the idle walk's tick the driver still drives and the stopping bit is not set
