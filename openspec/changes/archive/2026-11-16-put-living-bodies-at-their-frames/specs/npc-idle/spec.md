## MODIFIED Requirements

### Requirement: The revive reset's roam move
A roaming AI that is not a statue SHALL, in that reset, move over two rounds along a wander plan whose path spans more than 150 cm and then a home-side plan over the same span, each planned from where the last ended; it SHALL be put living there, its pelvis keeping its height over the ground, on its own turn, and, when the space query finds its body meeting another, at the grid's nearest walkable point; it SHALL leave combat and return to idle.

#### Scenario: A roam is put down elsewhere
- **WHEN** a roaming AI with lasting health 0.6 and current 0.2 is reset, and the wander plans span 173 cm
- **THEN** its current health is 0.6, its contacts are empty, it leaves combat and it ends at the second wander endpoint, planned from the first
