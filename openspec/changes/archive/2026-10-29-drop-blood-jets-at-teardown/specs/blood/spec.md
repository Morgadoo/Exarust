## ADDED Requirements

### Requirement: Jet frame precision
A jet's axis and its first axis SHALL be made unit by the processor's approximate reciprocal square root of (x² + z²) + y² (a zero vector staying zero); its third axis SHALL be their cross product as it comes. The frame SHALL be tied to the struck part by the transpose of the part's turn: each axis becomes its dots with the part's axes, (x·aₓ + y·a_y) + z·a_z, and the origin the same of its offset from the part's origin.

#### Scenario: A jet along −X
- **WHEN** a jet starts along (−4, 0, 0)
- **THEN** its axis is (−4·r, 0, 0) with r the approximate reciprocal square root of 16, within 0.001 of −X

#### Scenario: A turned part
- **WHEN** a part turned a quarter about Y at (10, 130, 0) is struck at (10, 140, 5) along +X
- **THEN** the jet sits at (−5, 10, 0) in the part's frame and its first droplet's carry is (0, 0, 2) there

### Requirement: Jets dropped at teardown
A world teardown SHALL drop every running jet. Droplets already flying, the shared emission clock and the ring's slot sizes SHALL stay. A level load without a teardown SHALL leave the jets.

#### Scenario: Level change during a spray
- **WHEN** the world is torn down while a jet still has droplets to send
- **THEN** the jet sends no more and the droplets in flight fall on

### Requirement: Weapon blood clamp
A weapon's blood SHALL become the sum of its blood and the new blood, then 1 when 1 is below the sum, else 0 when 0 is above it; a NaN sum and −0 SHALL pass unchanged.

#### Scenario: Soaking
- **WHEN** a weapon holding 0.9 draws 0.2285
- **THEN** it holds 1
