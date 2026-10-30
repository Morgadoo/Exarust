# Spec Delta

## ADDED Requirements

### Requirement: Torrent stream ribbon
A torrent stream of N samples (N at least 3) SHALL repeat its last sample and build 2N vertices: each sample, then its Hermite midpoint toward the next. It SHALL emit one quad call for each vertex but the last, and its strip SHALL not join another stream's. Its texture coordinate SHALL advance by the squared vertex distance times 0.005 while charging or 0.0003 while running, in double precision.

#### Scenario: Three samples
- **WHEN** sixteen streams of three samples are drawn
- **THEN** the ribbon has 160 vertices and 384 indices, and the second stream's first triangle starts at vertex 10

#### Scenario: The midpoint weights
- **WHEN** a midpoint between `a` and `b` is built
- **THEN** it is `0.5·a + 0.5·b`, plus `(b − previous)·0.0625` after the first segment, plus `(next − a)·−0.0625` where a next sample exists, else `(b − a)·−0.125`

#### Scenario: The last real midpoint
- **WHEN** samples at −10, 0 and 10 cm along z end a stream
- **THEN** the midpoint between 0 and 10 sits at 5.625 cm

#### Scenario: Texture step
- **WHEN** a 3 cm segment is walked while charging from 0
- **THEN** the coordinate is the single of 0x3d3851ec

### Requirement: Force clamps keep the second operand
The blast and torrent limb reach scale and the blast light's strength clamp SHALL use the engine's MINSS/MAXSS rule: a NaN in either operand SHALL give the second operand.

#### Scenario: A NaN reach
- **WHEN** a body's inverse mass is NaN
- **THEN** its limb reach scale is 3
