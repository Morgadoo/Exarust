## ADDED Requirements

### Requirement: The queued step bends at the target
When the body takes a queued footwork step whose vector points along its look and the body has a target, the vector SHALL be bent at the target's 60 cm × scale circle before the step aims at the chest plus it, in the same balance pass, for the player and the AI.

#### Scenario: A step into the target's circle
- **WHEN** a 70 cm queued step is taken toward a target 80 cm ahead
- **THEN** the step target is 20 cm ahead of the chest
