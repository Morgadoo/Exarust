## ADDED Requirements

### Requirement: Terrain GI block per tile
The terrain draw SHALL sample the GI block that the tile's position falls in for every block of the tile.

#### Scenario: A block at a tile's edge
- **WHEN** a terrain block near a tile's edge lies over another GI cell than the tile's centre
- **THEN** it samples the GI block of the tile's centre

### Requirement: Terrain composite size from the setting
The live terrain SHALL size its composite pools from the game's texture-detail setting.

#### Scenario: Detail 2
- **WHEN** the texture-detail setting is 2
- **THEN** the pools take the 4096 detail size and the high level 4
