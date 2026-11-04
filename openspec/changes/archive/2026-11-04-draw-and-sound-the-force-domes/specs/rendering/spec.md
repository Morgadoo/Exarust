## ADDED Requirements

### Requirement: Force dome programs
The system SHALL draw the force Barrier with the shipped `effectbarrier_v` and `effectbarrier_f` and the force Bubble with `effectbarrier_v` and `effectbubble_f`, over meshes in the node's frame in centimetres, under the `_ForceBarrier01` record's premultiplied blend (blend word 0x163), one-sided, the output clamped to [0, 1].

#### Scenario: A dome at the start of its grow
- **WHEN** a dome's `Fade` is 1
- **THEN** the Barrier's fragments have alpha 0 after the clamp, and the Bubble shows only where `(1.5 + n.y)` is under 1

### Requirement: Force dome uniforms
The system SHALL bind a force dome's `ViewPos` as the camera in the node's frame, `TexPos` as the dome's texture position, `ColPt` as its eight ripple points, `Fade` as its fade and `EmiTex` as the `_ForceNoise` picture.

#### Scenario: A ripple
- **WHEN** a dome has one ripple
- **THEN** `ColPt[0]` holds its place and shown value and the other seven have w 0
