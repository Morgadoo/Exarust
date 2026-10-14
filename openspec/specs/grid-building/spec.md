# grid-building Specification

## Purpose
Assembles the walls, floors and ceilings of tile-grid dungeons from the
original modular sets so levels look as they do in the original game.

## Requirements

### Requirement: Cell placement
The system SHALL build each open cell of a tile grid as a 6 m square centred at its grid position, the grid being centred on the level origin.

#### Scenario: Arena room
- **WHEN** the arena level is shown
- **THEN** its room is enclosed by wall pieces along the cell edges and floored with the floor set's pieces

### Requirement: Edge and corner codes
The system SHALL place per side a floor edge strip for an open edge, a straight, door, left-end or right-end wall piece for wall codes, and per corner a corner piece or a floor corner, each rotated a quarter turn per side.

#### Scenario: Wall variants
- **WHEN** a cell edge has a wall code and a variant number
- **THEN** the wall set's piece with that variant is placed, falling back to variant 0 when the set lacks it

### Requirement: Special tiles
The system SHALL place a cell's special tile from the wall set's trim pieces and suppress the edges and corners its type covers.

#### Scenario: Trim tile
- **WHEN** a cell selects a special tile type, index and rotation
- **THEN** the matching trim piece is placed with that rotation and the covered edges get no regular pieces

### Requirement: Ceilings on demand
The system SHALL build ceiling pieces only when asked to.

#### Scenario: Overhead view
- **WHEN** a level is shown without the ceiling option
- **THEN** no ceiling pieces are spawned

### Requirement: Colour-only materials
The system SHALL colour untextured materials with their stored diffuse colour and SHALL not draw geometry using the `_null` material.

#### Scenario: Wall caps
- **WHEN** wall tops use the `_black` material
- **THEN** they render black rather than default white

### Requirement: Piece strips and occluders
The system SHALL use a wall, corner or special-tile piece's own `+F` child as that edge's floor strip (with the floor set's surface material) and its `+R` child as the ceiling strip in place of the sets' strips, and SHALL never draw `[` occluder children.

#### Scenario: Wall with its own floor strip
- **WHEN** a wall piece with a `+F` child is placed on a cell edge
- **THEN** that strip is drawn with the floor set's material and no floor-set strip is added on that edge

### Requirement: Floor texture mapping
The system SHALL map tile-grid floors as the original does. Every `+`-material triangle of a cell's floor pieces (centre variant, edge strips and the pieces' `+F` strips) gets planar UVs u = x/6 + ½, v = ½ − z/6 in cell-local metres, after the piece's rotation. That is one square texture repeat per cell, aligned to its borders. Walls, ceilings and other materials keep their stored UVs.

#### Scenario: Menu floor
- **WHEN** the main menu scene is shown at the original's camera
- **THEN** the floor's bricks are the same size as in the original
