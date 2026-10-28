# level-lighting Specification

## Purpose
Lights levels like the original: per-level ambient colour, sunlight and fog
from the level's environment record and the game's per-level presets.

## Requirements

### Requirement: Environment record
The system SHALL decode the environment record of a level file (ambient colours, vertical extent, sun settings) and report a malformed record as a level error without failing the level.

#### Scenario: Corpus
- **WHEN** `exa-tool corpus levels` runs over the installed game
- **THEN** every level reports an environment record and no section errors

### Requirement: Level lighting presets
The system SHALL light story levels and indoor arenas without a sun, light outdoor arenas with a shadow-casting sun and fog, and elsewhere add a sun only when the record asks for it or, in auto mode, when the level has no lights.

#### Scenario: Outdoor arena
- **WHEN** `exarust level arenao3` runs
- **THEN** the level is lit by a sun from the record's altitude and azimuth, with shadows and fog

#### Scenario: Story level
- **WHEN** a story level is shown
- **THEN** no sun is added and only its own lights and the ambient light it from the record

### Requirement: Outdoor sky
The system SHALL render an open sky over levels lit by the outdoor preset, scattering the level's sunlight, and no sky over other levels.

#### Scenario: Outdoor arena
- **WHEN** `exarust level arenao3` runs
- **THEN** a daylight sky appears above the horizon instead of a flat background

### Requirement: Scene environment model
The system SHALL model the original's scene environment: its constructor defaults, the sun and moon placement and daylight factor computed with the original's precision and approximations, the six-record day track's pick and blend, cloud dimming, the fog's span and per-frame near and far distances, the clear and horizon colours, the automatic sunlight rule, rain fading and lightning with the original's random generators.

#### Scenario: Default sky
- **WHEN** the sky is turned on at the default time of day and azimuth
- **THEN** the sun is chosen with a daylight factor of about 0.4978 and the outdoor fog of 1 km is narrowed to about 831 m

#### Scenario: Lightning
- **WHEN** a strike 316 cm away starts at time 10 000 ms
- **THEN** it ends after 100 ms or more, its thunder is due at 10 583 ms, and while it runs the record's fog colour gains the lightning blue in proportion to I/(I + 2)·3

### Requirement: Linear outdoor fog
The system SHALL draw the outdoor preset's fog with a linear falloff between the environment's near and far distances.

#### Scenario: Outdoor arena
- **WHEN** an outdoor arena is lit
- **THEN** its fog starts at the camera and reaches full strength at the model's far distance

### Requirement: Unchanged indoor lighting
The system SHALL light story levels and levels that follow their record exactly as before the environment model: the record's ambient through the sRGB curve, the record's sun and colour where it asks for one, and no fog or sky indoors.

#### Scenario: Story level
- **WHEN** exanima01 or exanima02 is lit from its record
- **THEN** the ambient equals the record's linearised ambient and no sun, fog or sky is added

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
