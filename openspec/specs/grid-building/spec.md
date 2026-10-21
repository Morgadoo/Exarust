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

### Requirement: Colour-only materials
The system SHALL colour untextured materials with their stored diffuse colour and SHALL not draw geometry using the `_null` material.

#### Scenario: Wall caps
- **WHEN** wall tops use the `_black` material
- **THEN** they render black rather than default white

### Requirement: Piece strips and occluders
The system SHALL use a wall, corner or special-tile piece's own `+F` child as that edge's floor strip (with the floor set's surface material) and its `+R` child as the ceiling strip in place of the sets' strips. It SHALL merge every piece's `[` occluder children per room into one black occluder shell and never spawn them as scene pieces. Only top-level set nodes are pieces.

#### Scenario: Wall with its own floor strip
- **WHEN** a wall piece with a `+F` child is placed on a cell edge
- **THEN** that strip is drawn with the floor set's material and no floor-set strip is added on that edge

#### Scenario: Black wall hollow
- **WHEN** the arena hub is shown with the player in the bar
- **THEN** the 55 cm hollow between each wall facade and the cell edge is black up to the occluder's top, with no floor showing through it

### Requirement: Floor texture mapping
The system SHALL map tile-grid floors as the original does. Every `+`-material triangle of a cell's floor pieces (centre variant, edge strips and the pieces' `+F` strips) gets planar UVs u = x/6 + ½, v = ½ − z/6 in cell-local metres, after the piece's rotation. That is one square texture repeat per cell, aligned to its borders. Walls, ceilings and other materials keep their stored UVs.

#### Scenario: Menu floor
- **WHEN** the main menu scene is shown at the original's camera
- **THEN** the floor's bricks are the same size as in the original

### Requirement: Room lids
The system SHALL merge each room's ceiling strips, the sets' ceiling slot pieces its cells select and its pieces' `+R` children, into one black lid per room, and SHALL never spawn them as scene geometry.

#### Scenario: Lid over a room
- **WHEN** the arena hub's lids are built
- **THEN** seen from above, every cell of every room is covered by its room's lid with up-facing triangles

### Requirement: Ceiling caps
The system SHALL treat an open cell without a wall set that carries the terrain flag as a ceiling cap: it adds ceiling strips from the first wall set to its room's lid, with the slot choice of a normal cell and the wall and corner pieces' own `+R` strips, and builds no wall, floor or occluder.

#### Scenario: Cap cells of the first story level
- **WHEN** `exanima01` is built
- **THEN** its 104 terrain cells without a wall set add lid strips to their rooms and no pieces

### Requirement: Rotated special masks
The system SHALL turn a special tile's edge and corner mask by the cell's rotation (each edge nibble and corner triple one side per quarter turn, flag bits kept) before suppressing the covered pieces.

#### Scenario: Rotated trim
- **WHEN** a cell's special tile of type 2 has rotation 1
- **THEN** the mask 7070FFFF becomes 0707FFFF and the other two edges lose their regular pieces

### Requirement: Special piece choice
The system SHALL place the nth special piece of the cell's type; past the last it SHALL place the last, and with no piece of the type the wall set's first special piece.

#### Scenario: Index past the last
- **WHEN** a cell asks for the third piece of a type the set has two of
- **THEN** the second is placed

### Requirement: Terrain floor set
The system SHALL skip, without loading it, only a floor set whose name is exactly `Terrain`.

#### Scenario: Terrain-named set
- **WHEN** a grid names the floor sets `Terrain` and `GEFS_Terrain01`
- **THEN** the first brings no floor pieces and the second is loaded as usual

### Requirement: Node room lookup
The system SHALL sort a node or character into the room of the grid cell it stands in, `floor(x / 6 m) + half the grid`, clamped to the grid for nodes.

#### Scenario: Just below zero
- **WHEN** a node stands 10 cm on the negative side of a cell boundary
- **THEN** it belongs to the cell on that side, not the nearer centre
