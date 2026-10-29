## ADDED Requirements

### Requirement: Placed cloth start
A cloth whose mesh a level placement record replaces SHALL take the record's positions from the level's frame into its node's own frame. It SHALL start with the record's word as its sleep countdown, asleep when the word is under 1. A cloth without such a record SHALL start awake at 32.

#### Scenario: A placed banner
- **WHEN** a banner is placed turned half round, and its record stores a point 10 cm in front of the placement with the word 0
- **THEN** the point sits 10 cm along the node's own z, and the cloth starts asleep
