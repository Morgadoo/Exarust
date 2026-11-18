## MODIFIED Requirements

### Requirement: Painter magnifies linearly
The CPU painter SHALL sample a paint job's own maps (shade, masks, item normal, and the factory shade scratch's inputs) as the word `0x133` loads them: the bilinear mix of the four level-0 texels around each painted texel's centre, whether the map is magnified or minified, with clamped edges; a map exactly the region's size SHALL read its texels as they are.

#### Scenario: A half-size mask
- **WHEN** a two-texel mask of 0 and 255 is painted over four texels
- **THEN** the texels read 0, 0.25, 0.75 and 1

#### Scenario: A double-size mask
- **WHEN** a four-texel mask of 0, 255, 0, 255 is painted over two texels
- **THEN** both texels read 0.5

## ADDED Requirements

### Requirement: Painter filters library textures linearly
The CPU painter SHALL sample every library texture and emblem picture as the word `0x70133` loads them: the bilinear mix of the four level-0 texels around the sample point of the fragment's centre, repeating.

#### Scenario: A two-texel damage map
- **WHEN** a damage-scale texture two texels wide is sampled at painted column 511
- **THEN** it reads close to the mean of its two texels
