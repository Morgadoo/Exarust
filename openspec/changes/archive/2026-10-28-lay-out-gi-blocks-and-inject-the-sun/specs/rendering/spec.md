# Spec Delta

## MODIFIED Requirements

### Requirement: GI around the camera
The system SHALL lay the voxel GI blocks out once per level over the level grid's built cells, in blocks of eight cells slid inside the grid (one block at cells (3, 3) without a grid), and SHALL draw the block the camera falls in by the floored offset over 48 m, or else the block nearest the point 20 m ahead, so surfaces in view take their ambient from the GI anywhere in a large level.

#### Scenario: Far corner of a large level
- **WHEN** the camera looks at a room far from a large level's centre
- **THEN** the room's ambient comes from the GI volume, not the environment cube

#### Scenario: Point late in a block
- **WHEN** the camera stands 2.625 blocks from the block grid's edge
- **THEN** the third block is drawn, not the fourth

## ADDED Requirements

### Requirement: GI sun injection
The system SHALL inject the sun into every drawn GI sector's group ahead of its point lights, at its colour times 3 × 0.3 × 0.01 times the voxel's openness and its visibility in the sun's shadow map: the map's comparison alone under an open sky, otherwise that comparison less the one 0.01 deeper, at least 0. The sun SHALL be left out when its colour times 3 has a third component of at most 1.19e−7.

#### Scenario: Sun in a group
- **WHEN** a drawn sector lists five point lights and the sun is up
- **THEN** its group's first light is the sun and it counts towards the twelve

### Requirement: GI cube shadows
The system SHALL shadow each point light's GI injection by its cached cube shadow, comparing the distance over the light's radius plus 0.01 along the direction from the light to the voxel, and SHALL place the light at its position over the block's width on each axis with the height over the width.

#### Scenario: Light off the block's middle
- **WHEN** a light stands 6 m along world Z from a 51.2 m block's corner
- **THEN** its depth coordinate is 6 / 51.2

### Requirement: GI built-cell bounds
The system SHALL bound a level grid's built cells as the original's scan does: the first built cell inside the border gives the first row and column; each later row moves the left edge to its first built cell left of it, or else checks the span between the edges, both included; either reaches the bottom edge to that row, as does a built cell right of the span, which also moves the right edge. A grid without built cells SHALL give (−4, −4) to (3, 3).

#### Scenario: Column of cells
- **WHEN** only cells (1, 1) and (1, 4) are built
- **THEN** the bounds are (1, 1) to (1, 4)

### Requirement: GI shape draw box
The system SHALL draw a registered GI shape over its centre in voxels rounded half to even and its half extents rounded up, with the columns clipped to 8 and 248, from the centre less its vertical half extent for twice it plus one slices, and SHALL draw nothing when the centre slice is below 2 or the clipped box is empty.

#### Scenario: Shape near the floor
- **WHEN** a shape's centre lies in slice 1
- **THEN** it draws nothing

### Requirement: GI dynamic occupancy
The system SHALL update a moving GI mesh's occupancy by splatting its old points with weight −20 and its new points with +20, over a region padded 1.5 voxels on each side in world voxels and rounded half to even; a thin mesh SHALL weigh a quarter and a centred one SHALL add half a voxel to the height and leave its column's lowest slice alone.

#### Scenario: Tie in the padding
- **WHEN** a point lies 4 voxels from the block's corner
- **THEN** its region runs from voxel 2 to voxel 6
