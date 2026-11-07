## ADDED Requirements

### Requirement: Displacement tendril material
The displacement tendrils SHALL draw with the `_TDispMat01` record: each fragment the vertex colour times the `FlameletsA3` sample at the strip's first coordinate set times the record's opacity 0.5, clamped to [0, 1], blended ONE / ONE_MINUS_SRC_ALPHA on both faces with no shadows.

#### Scenario: A tendril fragment
- **WHEN** a strip vertex of colour (1, 1, 1, 1) samples a texel of 1
- **THEN** the fragment is 0.5 in every channel

### Requirement: AI Long Door destination
An AI's Long Door at phase 0x10 with no destination SHALL ask the navigation grid for a path from its subject (the caster's target, else the caster) toward the point 300 cm back along the subject's body's forward vector, and take the path's last point as its destination when the path has more than one point and that point lies more than √200 cm away across the floor; any other answer SHALL end the door.

#### Scenario: Open floor
- **WHEN** a duellist casts the Long Door at a player standing on open floor
- **THEN** it is placed 300 cm behind the player
