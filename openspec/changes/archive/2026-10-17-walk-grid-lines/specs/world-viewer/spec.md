## MODIFIED Requirements

### Requirement: Navigation grid overlay
`--debug-nav`, or F6 in any view, SHALL draw the level's navigation grid around where the camera looks: each floor cell within 8 m as a square at its floor's height, the lower layer green through yellow as its clearance rises to 180 and red beyond, the upper layer cyan through blue and magenta beyond. It SHALL also draw the player's last line walk on the grid: the way asked, the walk to its end (green free, orange blocked), the blocking cell, and a goal the body walks to.

#### Scenario: Arena floor
- **WHEN** `exarust --debug-nav play arena` runs
- **THEN** the floor around the player is covered in green squares, cut by the walls

#### Scenario: Look-ahead at a wall
- **WHEN** `exarust --debug-nav --press W@100-850 play arena` walks the player into the −Z wall
- **THEN** its look-ahead is drawn orange to a ring 15 cm before the outlined blocking cell, and the clamped goal is marked
