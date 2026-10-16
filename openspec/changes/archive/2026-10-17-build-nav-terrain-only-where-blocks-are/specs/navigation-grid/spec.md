## ADDED Requirements

### Requirement: Terrain blocks in the grid
The system SHALL fill the grid's terrain a 6 m block at a time over the blocks the level's terrain builds: in a level with a tile grid only those whose grid cell carries the terrain flag, otherwise every block of every tile. A block's 20 × 20 cells SHALL take the ground's height where a whole-tile fill takes it, and an absent block SHALL leave its cells without terrain.

#### Scenario: Rock between rooms
- **WHEN** exanima01's grid is built from the installation
- **THEN** terrain fills its 682 flagged blocks only, and of the cells over its 477 closed grid cells only 1120, at the walls' edges, have a floor (186 721 when whole tiles were filled)

#### Scenario: A tile with two blocks
- **WHEN** a tile whose blocks (0, 0) and (5, 2) alone exist is filled
- **THEN** 800 cells get a floor, each at the voxel a whole-tile fill gives it, and the cells of block (1, 0) get none

#### Scenario: No tile grid
- **WHEN** a level without a tile grid has a terrain tile
- **THEN** all its 64 blocks fill, 25 600 cells

## MODIFIED Requirements

### Requirement: Built at load
The system SHALL build the grid when a level finishes loading, from what is under the level root once it has world transforms, off the main thread, and keep it until the next level; until then the level has none (the original's fallback: no obstacle, no cost).

#### Scenario: Shipped levels
- **WHEN** the grids of arena, arenao1 and exanima01 are built from the installation
- **THEN** arena has 1935 floor cells, arenao1 14677 lower and 3657 upper, exanima01 219292 lower and 792 upper, and exanima01's start reaches its stair room's foot a metre lower
