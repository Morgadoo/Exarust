## ADDED Requirements

### Requirement: Terrain sun shadows from drawn blocks
A terrain block SHALL cast into the sun's shadow map only when the live scene's visible pass drew it this frame. A block the scene does not hold SHALL keep casting. `EXA_TERRAIN_SHADOW_ALL=1` SHALL turn the rule off.

#### Scenario: Block behind the camera
- **WHEN** a terrain block inside a sun cascade is outside the main view
- **THEN** it draws nothing into the sun's map

#### Scenario: Block in view
- **WHEN** the visible pass drew a terrain block
- **THEN** it casts into the sun's map as before
