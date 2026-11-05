# Spec Delta

## MODIFIED Requirements

### Requirement: Spawn snap
On the player's third behaviour tick with a navigation grid and a body, unless it is down, getting up or dead, the player SHALL be moved across the ground onto the grid's nearest walkable point to its frame's origin (where it stands, at its pelvis's height), once, and its body SHALL then be placed on the ground from that point's floor.

#### Scenario: The third tick
- **WHEN** the player's behaviour ticks three times with a grid, then again
- **THEN** nothing moves it on the first two, the third moves it across the ground to the nearest walkable point and places its body on the ground from that point's floor, and none after does

#### Scenario: Down on the third tick
- **WHEN** the player is down on its third behaviour tick
- **THEN** it is not moved, then or later
