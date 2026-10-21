# Spec Delta

## MODIFIED Requirements

### Requirement: GI light selection
The system SHALL inject into each GI sector only the lights its 6 m cell records from the light flood (up to twelve), as the original does: every registered light, over its stored radius whatever its range flag, an unranged light flooding with unlimited reach. A spot-shaped light SHALL be left out of a sector it no longer carries down to. A level without a wall grid SHALL draw the scene's lights in one group of all its sectors, from the bottom slice.

#### Scenario: Large level
- **WHEN** the camera is in a part of a level with hundreds of lights
- **THEN** each sector of the GI volume is lit only by the lights its cell lists, at most twelve

#### Scenario: Unranged light
- **WHEN** a level light has no range flag
- **THEN** it reaches every cell its flood passes and its GI falls off over its stored radius

## ADDED Requirements

### Requirement: GI sector marks
The system SHALL mark, every frame, the GI cells the camera needs as the original's first pass does: a cell at most six cells from the focus cell on each axis (and from 1 to the grid size − 2), at most 36 cells squared from it and built, whose box meets the view. The box is 300 cm either side of the cell's centre and runs from its floor slice to the block's top.

#### Scenario: Cell past the reach
- **WHEN** a cell lies five cells from the focus cell on both axes
- **THEN** it is not marked, at 50 cells squared

### Requirement: GI sector box test
The system SHALL count a GI sector box out of view when its centre lies beyond a view plane by more than 848.53 cm, or by more than the box reaches towards that plane, each half extent taking the plane normal's sign on top of its own.

#### Scenario: Box past a plane
- **WHEN** a box 300 cm wide lies with its centre 500 cm beyond a view plane
- **THEN** its cell is not marked

#### Scenario: Centre past the radius
- **WHEN** a box 1280 cm high lies with its centre 900 cm beyond a plane
- **THEN** its cell is not marked

#### Scenario: Cell without geometry
- **WHEN** a cell's floor is 255, so its box's half height is negative
- **THEN** the half height reaches away from a plane whose outward normal points up

### Requirement: GI mark widening
The system SHALL draw a sector whose cell passes the first pass's window, reach and built tests and whose own first mark, or a neighbour's, is set. The neighbours are read at the cell's row-major index ± 1 and ± the grid's height.

#### Scenario: Sector in view
- **WHEN** only one cell's box meets the view
- **THEN** that cell and its four side neighbours are drawn, the diagonal ones are not

#### Scenario: Unbuilt cell
- **WHEN** a cell next to a marked one is not built
- **THEN** it is not drawn

#### Scenario: Grid deeper than wide
- **WHEN** the grid is 16 cells wide and 12 deep
- **THEN** the second pass reads the marks 12 cells before and after a cell in row-major order

### Requirement: GI undrawn voxels
The system SHALL clear the voxels of unmarked sectors, of sectors without geometry, of the margins, of the slices below a group's floor and of the top slice every step, as the original clears its target and draws only the groups, and SHALL still add the voxel lights into them.

#### Scenario: Sector leaving the view
- **WHEN** the camera turns away from a lit sector
- **THEN** the sector's light is zero from the next step, apart from voxel lights

### Requirement: GI sector groups
The system SHALL group a block's marked sectors with geometry as the original does: a sector joins the first group with the same light set (as many lights, each found, in any order) whose floor lies within two slices, lowering the group's floor, or starts a group.

#### Scenario: Same lights, near floors
- **WHEN** sectors list the same two lights in different orders with floors 46, 47 and 48
- **THEN** they form one group with floor 46

### Requirement: GI group merges
The system SHALL, in two passes, fold a later GI group into an earlier one when they share a light (or both have none), their union stays within twelve lights, and the merged fill estimate (lights + 2) × sectors × ((depth − 1) − floor) + 80 is below the two estimates apart. The later group's new lights are appended, and the group takes the lower floor.

#### Scenario: Merge that costs more
- **WHEN** two one-light groups share their light with floors 10 and 100 in a block 128 slices high
- **THEN** they stay apart, because together they cost 782 against 431 + 161

#### Scenario: Merge that costs less
- **WHEN** the floors are 10 and 14
- **THEN** they merge with floor 10, 782 against 431 + 419
