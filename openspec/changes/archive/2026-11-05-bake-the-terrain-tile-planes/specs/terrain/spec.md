# Spec Delta

## MODIFIED Requirements

### Requirement: Height pass
The system SHALL displace each terrain vertex by its own texel of the tile's baked 244² height plane (cleared to 0.5), moving it `(byte − 127) × 1.25 × (2 − normal y)` cm; each material contributes `f16 + (H − f17) · f18`, the first overwriting whatever its weight and later ones mixed in by their merged weight boosted by 1.5, with no blend-mask sharpening.

#### Scenario: Neutral ground over a dungeon floor
- **WHEN** neutral terrain is painted over a dungeon floor at the same height
- **THEN** the terrain rests just above the floor instead of z-fighting with it

#### Scenario: A fifth material covering fully
- **WHEN** a tile's fifth material has full weight and value 1
- **THEN** its height byte is 255 whatever the first four gave

### Requirement: Terrain tile streaming bits
The system SHALL keep each tile's streaming bits: the reader's 7, the worker's `(state & ~0x10) | 0x828` plus the bake bit 0x1000 when bake planes are read back, and a frame hook that first takes a pending read-back (0x2000 off, upload 0x4000 on), then walks the tiles with bit 0x800 and handles the first bake (to 0x2000), insert (`(state & 0x309f) == 0xf`) or upload (`(state & 0x4080) == 0x4080`).

#### Scenario: An idle tile is passed
- **WHEN** the first streaming tile is already inserted and the second is built
- **THEN** the frame hook inserts the second tile

#### Scenario: A baked tile
- **WHEN** a tile is built with the shipped feature word
- **THEN** the hook bakes it on one frame and inserts it on the next, after taking its read-back

## ADDED Requirements

### Requirement: Terrain live tile hook
The system SHALL spawn a built terrain tile hidden, mark it built only once its entity exists, show it when the frame hook inserts it, insert every tile before the level shows, and after the load run the frame hook and then the block selection once a frame, skipping both while the refresh delay counts down; the hook's bake step SHALL leave the tile's visibility unchanged.

#### Scenario: Every tile shows after the load
- **WHEN** a level with terrain finishes loading
- **THEN** every tile is inserted and visible

### Requirement: Terrain tile bake
The system SHALL bake each built tile's mask plane (240² RGB) and height plane (244² red) from the merged weight bytes, sampled bilinearly at each pixel's centre on the passes' quads, in groups of four layers with later groups blending `ONE, SRC_ALPHA`, the mask plane written sRGB-encoded.

#### Scenario: Half a cell of grass
- **WHEN** a grass layer with green 1 has full weight on one vertex column only
- **THEN** the mask texel beside it holds the sRGB encoding of 0.5

### Requirement: Terrain bake material maps
Each material's mask (M) and height (H) map SHALL be sampled with its upload's levels, nearest texel and level, the bind's LOD bias and the quad's constant footprint, and the mask pass SHALL sharpen a masked layer by `(s, t/s + 1)` with GLSL's NaN-to-0 clamp.

#### Scenario: A zero sharpness
- **WHEN** a masked layer's sharpness `s` is 0 and its weight is 0
- **THEN** it adds nothing to the mask plane

### Requirement: Terrain plant density from the mask plane
A point's plant density SHALL be the green byte of its mask-plane texel times 1/255, less 0.2.

#### Scenario: Full green
- **WHEN** a texel's green byte is 255
- **THEN** the density is 0.8

## REMOVED Requirements

### Requirement: Terrain live frame hook
**Reason**: The frame hook's bit 0x1000 is the tile's bake, not an unload; the live hook never unloads a tile.
**Migration**: See "Terrain live tile hook" and "Terrain tile streaming bits".
