## ADDED Requirements

### Requirement: The flight's writes
On a flee outcome the brain SHALL end every live conversation it speaks in, attend the threat, and make it the body's target when it is a character (leaving the combat state) or its object focus otherwise; while at level 4 the target pass SHALL keep that threat as its foe.

#### Scenario: Fleeing the player
- **WHEN** a brain in combat with the player is classified to flee from it
- **THEN** its duellist targets the player and leaves the combat state, and the flight walk runs from the player

### Requirement: The flight ends
A fleeing brain whose threat is no longer fresh and stands in a room other than the brain's and its previous one, or that has neither a target nor an object focus, SHALL drop to level 0 with a calm clock 10 to 20 s out and resume its idle state.

#### Scenario: Out of sight two rooms away
- **WHEN** a fleeing brain has crossed into a second room and has lost sight of its threat
- **THEN** its level becomes 0 and its flight stops
