# Spec Delta

## ADDED Requirements

### Requirement: Cube face targets
The system SHALL upload a cube image's stored faces 0 to 5 to the faces −X, −Y, −Z, +X, +Y, +Z.

#### Scenario: Open sky cube
- **WHEN** `genref02r` is uploaded
- **THEN** its stored face 4, the sky, is the +Y (up) face and its stored face 1 the −Y face

### Requirement: Sky cube tint
The system SHALL tint the open sky's cube through `genref02rm` by the environment record's sky tint after each sun placement, every texel of every face's chain mixed from white towards the tint by its mask byte / 255, multiplied and rounded to a byte, and SHALL upload the result over the bound cube.

#### Scenario: Noon record
- **WHEN** the sun placement leaves a sky tint of (0.85, 0.9, 1.0)
- **THEN** a texel whose mask byte is 255 keeps 85 % of its red and 90 % of its green
