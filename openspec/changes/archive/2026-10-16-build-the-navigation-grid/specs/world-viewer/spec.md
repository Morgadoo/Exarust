## ADDED Requirements

### Requirement: Navigation grid overlay
`--debug-nav`, or F6 in any view, SHALL draw the level's navigation grid around where the camera looks: each floor cell within 8 m as a square at its floor's height, the lower layer green through yellow as its clearance rises to 180 and red beyond, the upper layer cyan through blue and magenta beyond.

#### Scenario: Arena floor
- **WHEN** `exarust --debug-nav play arena` runs
- **THEN** the floor around the player is covered in green squares, cut by the walls
