## ADDED Requirements

### Requirement: Mind-sight quad
The sight quad SHALL anchor the eye at the origin plus (anchor_a + 15)·m1 + anchor_b·m2, run its direction normalized toward the camera, blend the axis cross products by 1 − |view·dir.x|, and scale U and V by 0.025.

#### Scenario: A level camera
- **WHEN** the frame is the identity, the anchors are 0 and the camera lies ten units back
- **THEN** U and V each measure 0.025 and are perpendicular
- **THEN** the eye output is the negated view direction
