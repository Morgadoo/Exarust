## ADDED Requirements

### Requirement: Strikes program
The shell program SHALL have a strikes variant reproducing the shipped `effectfrcstrk_f`: fixed 0.01 texture coordinates, four ripple points, its own rim, the `Gradient` dimming and no colour limit on the alpha. The shell uniform SHALL carry `Gradient` at byte 256 of 272, in Rust and in WGSL.

#### Scenario: The strikes variant validates
- **WHEN** the shell program is composed for a rigid mesh with the strikes definition
- **THEN** naga validates it

### Requirement: Rigid shell copies
A held body's or a weapon's aura SHALL be drawn as copies of the body's rigid meshes moved out 2 cm along their normals, with `LclMat` taking the world into the body's first part's frame and the fade plane from the meshes' box in that frame.

#### Scenario: The part frame
- **WHEN** a rigid aura's uniform is built for a turned part
- **THEN** a world point's local position is the point in the part's frame in centimetres
