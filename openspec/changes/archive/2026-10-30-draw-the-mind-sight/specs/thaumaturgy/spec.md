# Spec Delta

## ADDED Requirements

### Requirement: Mind-sight sphere
The sight SHALL draw each entry on the engine's UV sphere of 16 segments and radius 15 cm with normals, its points shaped once by the sight's shaping and its normals left the unshaped sphere's. Its object matrix SHALL map a local point p to `p.x·s.x·m0 + p.y·s.y·m1 + p.z·s.z·m2 + origin`, m the head node's axes and s the record's scales.

#### Scenario: North pole
- **WHEN** the sphere is built
- **THEN** it has 151 vertices and its north pole (0, 15, 0) lies at (0, 10.0776, 3.2) with normal (0, 1, 0)

#### Scenario: Turned node
- **WHEN** a node's axes are (+y, −x, +z), its scales (1, 2, 3) and its origin (100, 200, 300) cm
- **THEN** a local (0, 1, 0) cm lands at (98, 200, 300) cm

### Requirement: Mind-sight fragment
The sight fragment SHALL cut the noise by `MindCol1.w` and `MindCol2.w` (each `clamp((tex − t)·8, 0, 1)`) to mix the three colours, weight the result by `clamp(clamp(1 − facing, 0, 1)·−0.3 + tex, 0, 1)`, clamp it to 0–1, write its square times four as colour and `MindCol0.w·facing` as alpha.

#### Scenario: Bright colour
- **WHEN** all three colours are (3, 0.5, −1) with thresholds 0, the noise is 1 and the facing 1
- **THEN** the colour is (4, 1, 0)

### Requirement: Mind-sight noise
The sight's noise SHALL be a 256² field of four octaves at amplitude 1 with both plane-building weights 0.5, then set to four octaves, a 0.25 s period and target 1; the spirit screen's SHALL be 512² with a 0.8 s period. Nothing SHALL be made or ticked while no entry is anchored.

#### Scenario: No anchor
- **WHEN** no body carries a sight anchor
- **THEN** the draw makes no sphere and no noise picture
