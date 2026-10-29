## MODIFIED Requirements

### Requirement: Mover refresh
A refresh SHALL drain the ring, grow each side by three cells and start the working floors from the floors as built. A sample voxel `y·0.1 + ½ − base·0.1` from 1 to 254 SHALL raise a missing working floor; one less than fifteen above SHALL take it; one fifteen or more above, or at or under the lower slab's underside, SHALL raise the upper floor when it is missing or less than fifteen below. Changed floors SHALL be written and the costs run again.

#### Scenario: A mover gone
- **WHEN** a sample at 2 m raises a cell's floor of 10 and a later refresh of that cell has no sample, and samples at 2.0, 2.04 and 1.96 m are rounded
- **THEN** the floor is 20, then 10 again with no roughness, and the voxels are 20, 21 and 20

#### Scenario: A body in another room
- **WHEN** a rectangle on one cell is refreshed with a body whose points reach that cell but whose node stands two 6 m rooms away, then with the body standing in the cell's room
- **THEN** the first refresh raises nothing and the second raises the cell

## ADDED Requirements

### Requirement: Refresh in play
Once per fixed tick, after the bodies step, the system SHALL refresh every queued rectangle of the level's grid from the moving meshes standing in the rooms it spans, and do nothing when none is queued. Letting go of an item lifted onto the cursor SHALL queue the held node's box at the frame the lift took it from. A body moved by the simulation alone SHALL queue nothing.

#### Scenario: An item let go
- **WHEN** a sack that raised its cell at the build is lifted and let go 60 cm away
- **THEN** the next tick puts the cell it left back to its floor as built and raises the cell it rests on

#### Scenario: A door swung open
- **WHEN** a door leaf that raised its doorway at the build swings open under the simulation
- **THEN** the doorway keeps its raised floor

### Requirement: Refresh of moved meshes on request
With `--nav-refresh-moved`, a moving mesh whose node has held a frame for 32 fixed ticks away from where the grid last saw it SHALL queue the rectangles of both frames.

#### Scenario: A leaf come to rest
- **WHEN** with the option a leaf moves 60 cm and holds still
- **THEN** nothing is queued for 32 ticks, then both places are queued and the next refresh frees the cell it left
