## MODIFIED Requirements

### Requirement: Piece strips and occluders
The system SHALL use a wall, corner or special-tile piece's own `+F` child as that edge's floor strip (with the floor set's surface material) and its `+R` child as the ceiling strip in place of the sets' strips. It SHALL merge every piece's `[` occluder children per room into one black occluder shell and never spawn them as scene pieces. Only top-level set nodes are pieces.

#### Scenario: Wall with its own floor strip
- **WHEN** a wall piece with a `+F` child is placed on a cell edge
- **THEN** that strip is drawn with the floor set's material and no floor-set strip is added on that edge

#### Scenario: Black wall hollow
- **WHEN** the arena hub is shown with the player in the bar
- **THEN** the 55 cm hollow between each wall facade and the cell edge is black up to the occluder's top, with no floor showing through it

## REMOVED Requirements

### Requirement: Ceilings on demand
**Reason**: The ceiling pieces are never scene geometry in the original; they make the rooms' black lids (Room lids).
**Migration**: `--ceilings` is gone; the lids of the rooms behind the player's doors show instead.

## ADDED Requirements

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
