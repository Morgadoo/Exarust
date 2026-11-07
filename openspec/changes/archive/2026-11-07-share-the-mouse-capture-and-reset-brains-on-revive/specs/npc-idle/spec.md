## MODIFIED Requirements

### Requirement: The level-leave reset
The story revive's world pass SHALL reset every AI character that is not collapsing, dead or dormant: it SHALL return its current health to its lasting pool when that pool is over 0.01 and forget its contacts and target. Entering the scene SHALL NOT apply this reset.

#### Scenario: A collapsed body
- **WHEN** a collapsed AI is reset
- **THEN** nothing changes

## ADDED Requirements

### Requirement: The revive reset's roam move
A roaming AI that is not a statue SHALL, in that reset, move over two rounds along a wander plan whose path spans more than 150 cm and then a home-side plan over the same span, each planned from where the last ended; it SHALL be placed there and, when the space query finds its body meeting another, moved to the grid's nearest walkable point; it SHALL leave combat and return to idle.

#### Scenario: A roam is put down elsewhere
- **WHEN** a roaming AI with lasting health 0.6 and current 0.2 is reset, and the wander plans span 173 cm
- **THEN** its current health is 0.6, its contacts are empty, it leaves combat and it ends at the second wander endpoint, planned from the first
