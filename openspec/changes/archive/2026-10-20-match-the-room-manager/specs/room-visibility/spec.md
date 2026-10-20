## ADDED Requirements

### Requirement: Room cell conversion
The system SHALL convert world points to cells by multiplying by the original's f32 1/600 and flooring, and SHALL convert a room's cell rectangle to centimetre bounds with the 0–280 cm ceiling band.

#### Scenario: A room rectangle
- **WHEN** a 32×32 grid's room spans cells (2, 3) to (5, 4)
- **THEN** its world bounds are x −8400..−6000 cm and z −7800..−6600 cm
- **WHEN** a point is at 5.99 m on X in a 6-wide grid
- **THEN** its cell is (3, 1)

### Requirement: Edge rotation
The system SHALL rotate a packed edge word by quarter turns: the low four nibbles cyclically and the high four nibbles' low three bits, with bit 3 of each high nibble fixed.

#### Scenario: One quarter turn
- **WHEN** the word 0x12345678 rotates once
- **THEN** it becomes 0x23416785
- **WHEN** 0x00000B07 rotates once
- **THEN** it becomes 0x0000B070

### Requirement: Direction step
The system SHALL step one cell opposite to a direction vector using `atan2(x, z)` and the four strict quadrants.

#### Scenario: Four directions
- **WHEN** the vector is (0, 1)
- **THEN** the step is (0, −1)
- **WHEN** the vector is (1, 0)
- **THEN** the step is (−1, 0)
