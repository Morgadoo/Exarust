## ADDED Requirements

### Requirement: Terrain blocks in grid levels
The system SHALL build, in a level with a tile grid, a terrain tile's 6 m block only where its grid cell carries the terrain flag (plane A `0x800000`), the grid offset by (cells − 8 × tiles)/2 halved towards zero on each axis, and SHALL give each block's mesh and plants the room of the block's centre. Without a grid every block is built.

#### Scenario: Rock between rooms
- **WHEN** `exanima01` is built
- **THEN** terrain exists on exactly its 682 flagged cells, and none of the closed cells between its rooms shows ground

#### Scenario: Outdoor arena
- **WHEN** `arenao3` is built
- **THEN** terrain covers its flagged 16 × 15 cell rectangle around the arena
