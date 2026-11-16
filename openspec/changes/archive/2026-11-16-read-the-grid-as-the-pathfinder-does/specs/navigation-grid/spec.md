# Spec Delta

## MODIFIED Requirements

### Requirement: Mover refresh
A refresh SHALL drain the ring, grow each side by three cells and start the working floors from the floors as built. A sample voxel from 1 to 254 SHALL raise a missing working floor; one less than fifteen above SHALL take it; one fifteen or more above, or at or under the lower slab's underside, SHALL raise the upper floor when it is missing or less than fifteen below. Changed floors SHALL be written and the costs run again.

#### Scenario: A mover gone
- **WHEN** a sample at 2 m raises a cell's floor of 10 and a later refresh of that cell has no sample, and samples at 2.0, 2.04 and 1.96 m are rounded
- **THEN** the floor is 20, then 10 again with no roughness, and the voxels are 20, 21 and 20

#### Scenario: A body in another room
- **WHEN** a rectangle on one cell is refreshed with a body whose points reach that cell but whose box centre stands two 6 m rooms away, then with the body standing in the cell's room
- **THEN** the first refresh raises nothing and the second raises the cell

### Requirement: Moving props raise floors
Once the layers are written and before the cost passes, every mesh with GI points whose node carries a rigid body SHALL raise the floors under it, level geometry or not: each point through the node's pre-scaled frame to its cell and voxel, inside cells 1 to W − 3 and D − 3 and voxels 0 to 254, raises its cell's current floors one point after another by the refresh's rule. The floors as built SHALL stay.

#### Scenario: A closed door
- **WHEN** a door leaf's points stand one voxel apart up its doorway, lowest first
- **THEN** the doorway's lower floor rises to the leaf's top and its floor as built stays

#### Scenario: A point high over the floor
- **WHEN** a point lies fifteen voxels or more over a cell's lower floor
- **THEN** it raises the cell's upper floor when that is missing

## ADDED Requirements

### Requirement: Pre-scaled sample frame
The build's raise and the refresh SHALL map a node-space point (cm) through the frame scaled to cells and voxels: each axis's x and z times 1/30 and its y times 0.1 in double, the translation's x and z `W/2 + (t/30 − ½)` and its y `(t·0.1 + ½) − base·0.1` in double, the point `(x·X + y·Y) + (z·Z + T)` in single precision, rounded ties to even.

#### Scenario: A turned frame and the ties
- **WHEN** a point 0.3 m along x is sampled through an identity frame on a 64-cell grid, and a point 0.95 m along a frame turned a quarter about y and moved 0.65 m
- **THEN** the first falls in cell 32 (the tie 32.5) and the second in cells 34 across and 28 along

### Requirement: Refresh rooms
A refresh SHALL take its samples from the simulation's 6 m rooms the grown rectangle spans, row by row and each room's bodies in turn, a body bucketed by its box centre: `floor(cm/600)` plus half the scene's room count on each axis.

#### Scenario: The shipped levels
- **WHEN** the installation is present and each level's nodes that queue at registration are refreshed after the build
- **THEN** arena and arenao1 change no floor and the refresh takes under 250 ms

### Requirement: Registration queue
A mesh whose render word has `0x100` SHALL queue its box under its frame in the refresh's pending list when it registers with the level's grid built; one registered while the level has no grid SHALL queue nothing. Either way it is queued once.

#### Scenario: Before and after the grid
- **WHEN** such a node registers before the grid exists, and another after, over a cell its floor as built leaves lower than now
- **THEN** the first leaves the cell, the second's tick refreshes it to its floor as built

### Requirement: Unchecked grid reads
The planner, its corner line walk and the nearest-walkable flood SHALL read a cell by the index `z·W + x` in 32 bits taken unsigned, so a column past the grid's side reads the neighbouring row's cell; an index past the grid's cells SHALL read no floor.

#### Scenario: Two rows joined past the east edge
- **WHEN** two rows of floor reach the east edge and the first column holds floor one row below the first to one row below the second
- **THEN** a plan from one row to the other reaches it through points past the edge, and without that column it does not

#### Scenario: The flood past the west edge
- **WHEN** a costly point stands in the first column and the previous row's last cell is cheap
- **THEN** the nearest walkable point is that cell's centre placed one cell west of the grid
