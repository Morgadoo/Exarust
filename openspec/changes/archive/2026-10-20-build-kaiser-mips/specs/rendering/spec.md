## ADDED Requirements

### Requirement: Kaiser kernel
The Kaiser table of a level SHALL use n = 8·2^(level−1) taps, h = n/2, c = 2^−level, and w = I0(4√(1−t²))/I0(4)·sinc(πcu) inside |t| < 1, zero outside, normalised to sum 1.

#### Scenario: Level one
- **WHEN** the level-1 table is built
- **THEN** it has eight taps, is symmetric, sums to 1 and has larger weights near its centre

### Requirement: Mip axis pass
The axis pass SHALL use the integer ratio src/dst, start at −((n−ratio)/2), and wrap each tap modulo the source extent or clamp it to the ends.

#### Scenario: Average, clamp and wrap
- **WHEN** two RGBA texels are averaged by half weights
- **THEN** the output is their mean
- **WHEN** four taps start before the first texel
- **THEN** the clamped pass repeats the first texel and the wrapping pass takes the last
