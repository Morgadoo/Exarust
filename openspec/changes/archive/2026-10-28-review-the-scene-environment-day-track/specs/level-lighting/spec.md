## ADDED Requirements

### Requirement: Built-in day track
The scene environment SHALL start with the engine's built-in six-record day track keyed 0.38, 0.44, 0.47, 0.54, 0.62 and 0.70, with the image's values, keep its colours in sRGB as the shipped game does (the start-up linearise-and-scale step is modelled but off), give a record with a black `o` colour the colour a quarter of the way from its away sky colour to `j`, and linearise the frame's horizon colour while the track is not linearised.

#### Scenario: Outdoor arena at the default time
- **WHEN** arenao3 is lit at the default time (daylight factor 0.4978)
- **THEN** the record blends the third and fourth records, the fog colour's red is 0.0236 (0.375 of the blend-linearised fog), and the sun light's red is 0.329 (the record's sun colour as stored)

#### Scenario: Optional linearising step
- **WHEN** the track's start-up pass runs with its linearising step
- **THEN** the third record's away sky red is 0.0139 (0.145 through the sRGB curve, times 0.75) and k's fourth slot stays 6

### Requirement: Frame environment values
Each frame the scene environment SHALL publish −25 as its sun-height value when the sun height is above 0.4 in double precision and −100000 otherwise, the current rain times 0.3 as the wind, and the tint colour times the tint's fourth slot.

#### Scenario: Thresholds
- **WHEN** the sun height is 0.39, then the single-precision 0.4
- **THEN** the value is −100000, then −25; a rain of 0.5 gives a wind of 0.15 and a tint of (0.5, 0.25, 0.75, 2) gives (1, 0.5, 1.5)
