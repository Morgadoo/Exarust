## ADDED Requirements

### Requirement: Procedural noise images
The system SHALL generate noise as the original does: white noise from the sequence s = s·0x08088405 + 1 (a byte is the high word of s·255, a float s as a signed integer times 2⁻³²), byte octaves k = 1, 2, … read from the first (w >> k)·(h >> k) white bytes, blended by the original's two-half bilinear rule with (old + sample)·½ clamped and rounded to even, the ridge fold of a byte map, and ridged float octaves mixed in at half weight.

#### Scenario: White noise
- **WHEN** four bytes of white noise are drawn from seed 0
- **THEN** they are 0, 8, 219 and 51

#### Scenario: An eight-square map
- **WHEN** an 8×8 byte map is built from seed 137 with two octaves
- **THEN** its first row is 138, 143, 140, 171, 172, 131, 187, 203

#### Scenario: Ridge fold
- **WHEN** the bytes 0, 64, 127, 128 and 200 are folded
- **THEN** they become 0, 65, 255, 255 and 48

### Requirement: Animated noise field
The system SHALL cross-fade four ridged noise planes as the original's force and spirit powers do. The phase is offset + t/period. At phase fraction f the three oldest planes weigh amplitude·⅔·sin²((f + 2 − i)·π/3). The fourth plane is built row by row from fresh white noise as ridge·0.27 + old·0.45, and the planes rotate on each whole phase.

#### Scenario: Weights
- **WHEN** the field is ticked to phase fraction 0.025
- **THEN** its output is the three oldest planes weighed about 0.485, 0.515 and 0.0005

### Requirement: Noise field timing
The field's amplitude SHALL ease toward its target by 0.05 per second up and 0.1 down, or snap when within 0.11 of the tick. The tick SHALL be clamped to 0.1 s, and ticks closer than 16 ms SHALL do nothing.

#### Scenario: A short tick
- **WHEN** a field last ticked at 0 ms is ticked at 15 ms
- **THEN** nothing changes

#### Scenario: A whole phase
- **WHEN** a field with a one-second period, amplitude 1 and target 0 is ticked at 1250 ms
- **THEN** its planes rotate once, its phase fraction is 0.25 and its amplitude is 0.99

### Requirement: Mip filtering space
A generated chain SHALL be filtered in the space the stored format word picks: centred for signed data (bit 28), linear for words with bit 16 or 17, and otherwise with colour squared and taken back through the square root, alpha linear for RGBA and grey-alpha words. Block formats decoded on the CPU SHALL be filtered linearly.

#### Scenario: A checker in gamma space
- **WHEN** a 4×4 checker of 0 and 255 in a plain RGBA8 texture is chained
- **THEN** its 2×2 level's colour is about 180 and its alpha about 128
