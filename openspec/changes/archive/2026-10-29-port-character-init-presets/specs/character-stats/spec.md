## ADDED Requirements

### Requirement: Race presets at spawn
A character's derived balance bonus SHALL be 2.0 for race bytes 5, 6 and 7 and 1.5 for race byte 8, and its arm relaxation SHALL be 0 for race byte 5, 0.5 for race byte 7, 0.5 for golems and 0 for race byte 17; any other race SHALL keep its race record's values.

#### Scenario: A human keeps its record's factors
- **WHEN** a human record spawns of a race whose balance bonus is 1.25 and arm relaxation 0.75
- **THEN** its stats carry 1.25 and 0.75

#### Scenario: A Fael
- **WHEN** a Fael record spawns
- **THEN** its balance bonus is 2.0 and its arm relaxation 0

### Requirement: Footstep volume at spawn
A character's footstep volume SHALL be its body scale, doubled when the scale exceeds 1.3 in a double-precision compare, and 5 for race byte 3.

#### Scenario: An ogre
- **WHEN** an ogre record spawns
- **THEN** its footstep volume is 5

#### Scenario: A scale of exactly 1.3
- **WHEN** the body scale is 1.3 in single precision
- **THEN** the footstep volume is 1.3, not doubled

### Requirement: Golems at spawn
A race byte with bit 0x10 that is neither the skeleton's nor the wraith's SHALL mark a golem, which walks on the golem footstep set whatever its race names.

#### Scenario: A golem's footsteps
- **WHEN** a record of race byte 0x12 spawns
- **THEN** it is a golem and walks on the golem footstep set
