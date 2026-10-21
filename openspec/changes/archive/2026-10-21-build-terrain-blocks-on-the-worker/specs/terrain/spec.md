# Spec Delta

## ADDED Requirements

### Requirement: Terrain block meshes
The system SHALL build each 6 m block of a terrain tile as its own mesh from one triangle patch shared by every block: 30 × 30 cells in serpentine rows (the first along +X, the next back along −X), each quad split along the diagonal from its low corner to its high corner, the patch's indices flattened into the tile's 241-vertex rows and offset to the block's first vertex. Quads whose low corner is a hole SHALL be left out.

#### Scenario: A full tile
- **WHEN** a tile outside a grid level is built
- **THEN** it has 64 block meshes, each covering only its own 6 m square

#### Scenario: The coarse patch
- **WHEN** the patch is built over 4 × 4 cells with a shift of one
- **THEN** it has two rows of two quads, each two cells wide and two rows deep

### Requirement: Terrain built off the main thread
The system SHALL build a terrain tile's heights, material masks and block meshes on a worker while the loading screen keeps drawing, and SHALL start the next tile's build while the previous one's textures load and its blocks spawn. The level built a step at a time SHALL equal the level built at once.

#### Scenario: A story level
- **WHEN** exanima01's terrain is built through the worker
- **THEN** all 20 tiles, their plants and the height query exist when the terrain stage ends

### Requirement: Terrain height query
The system SHALL answer the ground height under a world point as the original does: the tile coordinate floored toward −∞ from `width/2 + x/48 m` (NaN and out-of-range giving no tile), the tile taken from slot `tz · width + tx` anywhere in the grid's slots, the 6 m block from the fraction times eight and only where the level builds it, the height bilinear between the four vertices around the point, no ground where any of them is a hole by the byte-wise mask, and no ground at or below −100 m.

#### Scenario: A tile centre
- **WHEN** the query is asked 10 cm off the centre of each shipped exanima01 tile
- **THEN** it answers that tile's own bilinear height there

#### Scenario: Past a row's end
- **WHEN** a point lies one tile past the end of a row
- **THEN** the query answers from the next row's first tile

### Requirement: Terrain block priorities
The system SHALL rank a terrain block by round-to-even(6000 − d), d its distance in centimetres, within 60 m and 0 beyond or for NaN, and SHALL set priorities from the lowest and highest ranks of the wanted, visible blocks, found in one pass that lowers the lowest or else raises the highest.

#### Scenario: Falling ranks
- **WHEN** two wanted, visible blocks carry ranks 300 and then 100
- **THEN** the priority base is 6000 and the blocks take half their old rank plus 6000 minus their distance rank

### Requirement: Terrain composite selection
The system SHALL model the original's composite selection: the focus 10 m ahead of the camera snapped to a 6 m block centre; a composite for each block whose squared distance does not compare above 4440² cm² (NaN kept), high resolution below 1500² cm²; at most 189 blocks; 21 high and 43 low sets; and the 3 × 3 tile gather with empty placeholders.

#### Scenario: The focus
- **WHEN** the camera stands at (20, −4) m looking along −Z
- **THEN** the composite focus is (21, −15) m

#### Scenario: The cut-offs
- **WHEN** blocks lie 14.99 m, 15 m, 44.4 m and 44.41 m from the focus
- **THEN** the first is high resolution, the next two low, and the last has none

## MODIFIED Requirements

### Requirement: Terrain composite
The system SHALL shade terrain from the original's composite rules. Gloss comes from the layers' gloss textures (roughness 1 − gloss, with no floor, a full gloss being a mirror without a highlight), the albedo is × 0.75 and the specular is masked by the red of that albedo (`min(255·r, 1)`), the normal map and the parallax view vector are taken in the frame of +X projected onto the surface and its cross with the vertex normal, and parallax comes from the bump slot's height at 0.007 per 6 m block.

#### Scenario: Dry dungeon floor
- **WHEN** a torch lights a terrain floor of cobblestones
- **THEN** the stones look matte, with soft relief, not wet

#### Scenario: A sloped tile
- **WHEN** a normal-mapped terrain slope is lit
- **THEN** its normal map's axes stay at right angles to each other and to the vertex normal

#### Scenario: Full gloss
- **WHEN** a sand layer's gloss reaches 1
- **THEN** it shows no highlight and no invalid pixel
