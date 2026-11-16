## ADDED Requirements

### Requirement: Terrain blank blocks' plants are not drawn
Every camera's draw SHALL leave out the plant chunks standing on a terrain block whose composite state is blank after the frame's selection, as the grass main draw tests that block's blank bit before drawing its plant instances. The chunks SHALL stay shadow casters, since the grass sun and omni shadow draws have no blank test. `EXA_TERRAIN_DRAW_BLANK=1` SHALL turn this off with the blocks' rule.

#### Scenario: Plants past the reach
- **WHEN** a plant chunk stands on a block in view 61 m from the look-ahead point, and the camera and the sun's cascade list it
- **THEN** the camera leaves the chunk out and the cascade keeps it

#### Scenario: Plants in reach
- **WHEN** a plant chunk stands on a block within reach that holds a composite
- **THEN** the camera draws it
