# Spec Delta

## ADDED Requirements

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
