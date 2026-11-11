## ADDED Requirements

### Requirement: Terrain cube-face shadows from drawn blocks
A point light's cube face rendered in a frame SHALL leave out every terrain block the live scene's visible pass did not draw that frame. A block the scene does not hold SHALL keep casting. `EXA_TERRAIN_SHADOW_ALL=1` SHALL turn the rule off.

#### Scenario: Block out of view
- **WHEN** a dirty cube face renders while a linked terrain block is outside the main view
- **THEN** the block draws nothing into that face

#### Scenario: Block in view
- **WHEN** the visible pass drew a linked terrain block
- **THEN** it casts into the face as before

### Requirement: Cube faces kept as rendered
Leaving a terrain block out of a cube face SHALL NOT dirty the face, and a block's visible state changing SHALL NOT dirty it either; the face SHALL be rendered again only when the light or a linked caster dirties it.

#### Scenario: Block comes into view
- **WHEN** a block left out of a cached face comes into view and nothing else changes
- **THEN** the face is not rendered again and still lacks the block's shadow

### Requirement: Unused terrain blur
The terrain rule pass SHALL read rule input 2 as zero, and the texture unit's Gaussian blur kernel SHALL NOT be applied to any terrain plane, since the original builds the kernel and its scratch planes at init and never reads them.

#### Scenario: A rule on input 2
- **WHEN** a palette rule selects input 2
- **THEN** it reads zero at every vertex
