## MODIFIED Requirements

### Requirement: Arc character query
Each tendril segment but the last SHALL be given to the limb's swept query whole (start, end, unit direction and length) as a sphere of 20 cm radius swept from its start to its end through the limb's collision mesh in the limb's frame (through its stored spheres when it has no mesh); the first segment whose sweep touches a limb of another character SHALL shock it, once per character when the emitter shocks once, and the tendril SHALL send a branch toward the limb from the point before.

#### Scenario: The first segment meets
- **WHEN** the first 4 cm segment meets a limb
- **THEN** the query received a length of 4 and the radius 20, and the tendril branches from its first point

#### Scenario: A sweep that stops short
- **WHEN** a 20 cm sphere swept toward a limb's mesh from 50 cm in front ends 15 cm in front of it, and another ends 30 cm in front
- **THEN** the first touches the limb and the second does not

## ADDED Requirements

### Requirement: Arc electricity program
Live electric arcs SHALL draw with the electricity program and the `_ElecMat01` record's emissive layer `BeamNoise01`, with EleCol (0.65, 0.4, 1), EdgCol (0.15, 0.2, 0.4) and a texture offset of (0, the emitter's shown scroll), additive and two-sided.

#### Scenario: A drawn arc
- **WHEN** an arc whose shown scroll is 0.25 is drawn
- **THEN** its material's texture offset is (0, 0.25)
