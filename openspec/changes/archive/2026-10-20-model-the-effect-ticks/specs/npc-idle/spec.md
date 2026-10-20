## ADDED Requirements

### Requirement: Golem dormancy
The system SHALL scale stored values by 0.05 on fade-in and 0.02 on fade-out, in double precision.

#### Scenario: One scale each
- **WHEN** the source is 20
- **THEN** the fade-in value is 1
- **WHEN** the glow is 50
- **THEN** the fade-out value is 1

### Requirement: Golem homing
The system SHALL compute the homeward delay as distance + π + roll − 0.5 and the reach as 200 + offset.

#### Scenario: A centred roll
- **WHEN** the distance is 10 and the roll is 0.5
- **THEN** the delay is 10 + π and the reach for an offset of 5 is 205
