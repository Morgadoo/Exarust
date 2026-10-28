## REMOVED Requirements

### Requirement: Cloth contacts are opt-in
**Reason**: The cloth contacts now follow the original's cost rules and are live by default.
**Migration**: `EXA_CLOTH_CONTACTS=0` turns them off; see "Cloth contacts are live".

## ADDED Requirements

### Requirement: Cloth contacts are live
The level's cloths SHALL gather body and level contacts every tick unless the cloth contacts switch is off (`EXA_CLOTH_CONTACTS=0`). A cloth's bound radius SHALL be its particles' half-extent in centimetres, scaled by its node. A cloth SHALL NOT meet the static meshes of its own scene node. A sleeping cloth SHALL take only awake bodies, and it SHALL skip the stage while its frame is unchanged and no body sphere reaches it.

#### Scenario: A falling cloth lands and sleeps
- **WHEN** a free cloth falls onto a level floor and rests there
- **THEN** it gathers the floor's triangles, stays above the floor and falls asleep

#### Scenario: The cloth's own mesh
- **WHEN** the floor belongs to the cloth's own scene node
- **THEN** the cloth falls through it

### Requirement: Cloth level contact
After its step, a cloth SHALL meet each static mesh in order, particle by particle: a particle that moved over 0.1 cm² SHALL sweep a 2 cm sphere from its last position, any other SHALL test a 2 cm sphere where it is, first against the mesh's box and then against every triangle's front face in order, each push moving it before the next. A level push SHALL NOT wake the cloth.

#### Scenario: A falling particle
- **WHEN** a particle sweeps from 8 cm to −8 cm through a floor
- **THEN** it is put back 2 cm over the floor and slides to 2.1 cm

### Requirement: Cloth level slide
A particle a level mesh pushed SHALL lose 0.1 cm of its travel since its last position, or all of it when that travel is no longer than 0.1 cm.

#### Scenario: A resting particle
- **WHEN** a still particle sits 1 cm over a floor
- **THEN** it is pushed to 2 cm and keeps 0.9 cm of the push, ending at 1.9 cm, and a second pass leaves it there

### Requirement: Cloth sleep reads the velocity sum
The cloth's countdown SHALL read the squared length of its particles' summed velocity from the last tick, before drag, gravity and wind. It SHALL count down under √(vertices × 10⁻⁶) and count up over twice that, squared. The rustle and the drag's mean velocity (the sum times the reciprocal of the vertex count) SHALL read the same sum.

#### Scenario: Held against gravity
- **WHEN** one particle rests on a plane under gravity for 33 ticks
- **THEN** its countdown reaches 0

### Requirement: Cloth body sphere cull
Each body sphere offered to a cloth, padded by 2 cm, SHALL be kept only when it reaches the box around the cloth's particles. A sphere that moved more than 0.01 cm² over the tick (compared in double precision) SHALL be swept, and its sweep SHALL meet the box.

#### Scenario: A narrow box
- **WHEN** the cloth's box spans x = −74 to −46 cm and a 5 cm sphere sits at −80 cm and a 10 cm sphere at −100 cm
- **THEN** only the 5 cm sphere is kept
