## ADDED Requirements

### Requirement: A collapsing or dead target is dropped
The combat state SHALL drop a target whose condition carries the collapsing bit (collapsing or dead) before it perceives: the tick SHALL find no foe and leave combat after the follow check and the down exit, a focus having been cleared when the foe was taken, and the dropped target SHALL not be perceived that tick.

#### Scenario: The foe collapses
- **WHEN** a duellist fights a target whose condition becomes collapsing, or dead
- **THEN** on the next tick it leaves combat, its combatant attends nobody, and the contact of the dead target records when it died
