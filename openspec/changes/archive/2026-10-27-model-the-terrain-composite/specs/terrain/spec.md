# Spec Delta

## ADDED Requirements

### Requirement: Terrain wet floor
The system SHALL, when the palette's wet-floor source is above 0.4, shade the terrain with roughness `1 − min(G + 8c², 0.85)`, where `c = max((y + 35)·−0.03, 0)²·n.y`, G is the composited gloss and y is in world centimetres.

#### Scenario: A wet floor
- **WHEN** a sewer floor with gloss 0.2 lies at −135 cm with an upward normal
- **THEN** its gloss rises to 0.85, while ground at or above −35 cm keeps 0.2

### Requirement: Terrain composite selection order
The system SHALL sort the selected blocks stably by key: a visible blank block 0, a visible block wanting work `0xffff − (ring − priority)` (1 once the 64-bit difference reaches 0xffff), a hidden block its nearest light's distance rounded to even plus 0x10000. The walk SHALL bake every key-0 block and stop at the first non-zero key once the frame changed.

#### Scenario: One block a frame
- **WHEN** two baked, visible blocks both want work again
- **THEN** one is rebaked this frame and the other the next

#### Scenario: No high set to be had
- **WHEN** a block wants a high set, the high pool is empty and no block holds one at low resolution
- **THEN** the block keeps no set and the frame still counts as changed

### Requirement: Terrain composite set hand-out
The system SHALL hand out a low set from the pool, else from a scan down the list from a persistent cursor that ends at the first low-set holder and takes its set only when that block is hidden; and a high set from the pool, else from the last holder of a high set at low resolution, which keeps a low copy unless it is blank.

#### Scenario: A visible holder ends the scan
- **WHEN** the low pool is empty and the scan meets a visible block holding a low set before a hidden one
- **THEN** no low set is handed out

### Requirement: Terrain composite sets
The system SHALL size the composite pools from the texture detail (8192 at setting 3 and above, else 4096): the high level `floor(log2(size/8)) − 5`, the low level one below, `32 << level` texel sets with a half-size displacement target, 21 high and 43 low sets handed out from the ends of their free lists.

#### Scenario: The default pools
- **WHEN** the level build sizes the pools at the default texture detail
- **THEN** 21 sets are at level 5 (1024²) and 43 at level 4 (512²)

### Requirement: Terrain composite pass keys
The system SHALL key a composite pass as `layers | textured<<4 | masked<<8 | add<<28 | base_normal<<29` over groups of four layers, and give a block at level L the `TexPos` `(2^(L−1) − (bx >> (3 − L)), 2^(L−1) − (by >> (3 − L)))`, (0.5, 0.5) at level 0.

#### Scenario: An additive group
- **WHEN** a normal pass covers six layers
- **THEN** its second group of two layers carries the additive bit

### Requirement: Terrain tile streaming bits
The system SHALL keep each tile's streaming bits: the reader's 7, the worker's `(state & ~0x10) | 0x828`, and a frame hook that walks the tiles with bit 0x800, marks a ready, inserted tile's blocks wanted, and handles the first reload (0x1000), insert (`(state & 0x309f) == 0xf`) or upload (`(state & 0x4080) == 0x4080`), passing idle tiles.

#### Scenario: An idle tile is passed
- **WHEN** the first streaming tile is already inserted and the second is built
- **THEN** the frame hook inserts the second tile

### Requirement: Terrain wireframe overlay
The system SHALL draw every terrain block's triangle edges as lines when `EXA_TERRAIN_WIRE` is set, as the original's debug wireframe hook does.

#### Scenario: The overlay
- **WHEN** a level with terrain loads with `EXA_TERRAIN_WIRE` set
- **THEN** each block has a line-list child with three edges per triangle
