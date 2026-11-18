## ADDED Requirements

### Requirement: Grass hook draw state
The terrain plants SHALL be drawn after every opaque and alpha-tested mesh and the terrain, before the decals, without blending and with depth writes off, cutting the texels whose diffuse alpha is below 0.1.

#### Scenario: A decal behind a plant
- **WHEN** a plant chunk stands in front of a decal on the ground behind it
- **THEN** the decal, drawn later, still shows over the plant's texels, which wrote no depth
