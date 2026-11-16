## ADDED Requirements

### Requirement: Painter magnifies linearly
The CPU painter SHALL sample a region's own maps (shade, masks, item normal) bilinearly around each painted texel's centre, wrapping, when the map is no larger than the region along both axes; a map exactly the region's size SHALL read its texels as they are, and a larger one the nearest texel.

#### Scenario: A half-size mask
- **WHEN** a two-texel mask of 0 and 255 is painted over four texels
- **THEN** the texels read 0.25, 0.25, 0.75 and 0.75
