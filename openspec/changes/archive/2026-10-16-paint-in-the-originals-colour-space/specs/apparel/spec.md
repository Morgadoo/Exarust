## ADDED Requirements

### Requirement: Painted colour space
The system SHALL read painted apparel colour in the original's colour space.
- The finishes SHALL read the shade map, layer masks and library textures as stored, without sRGB decoding, and SHALL take the palette colour as computed.
- The painted colour SHALL keep the finishes' values as sRGB texels, decoded to linear light when sampled for lighting.
- Its mip levels SHALL average colour in linear light.
- The painted surface and normal maps SHALL be read as linear data.

#### Scenario: Pale grey on a mid-grey shade map
- **WHEN** palette colour 1 is painted by a finish without its own painter over a mid-grey shade map (128)
- **THEN** the texel is 160 and the surface is lit with 0.3515

#### Scenario: Rough cloth
- **WHEN** rough cloth in palette colour 1, at quality 160 and damage 64, is painted over a mid-grey shade map and mid-grey library textures
- **THEN** the texel is 96 and the surface is lit with 0.117, as the original's cloth program gives

#### Scenario: Seen from a distance
- **WHEN** a painted colour map's four texels are one white and three black
- **THEN** their mip texel is 137 (linear 0.25), not 64
