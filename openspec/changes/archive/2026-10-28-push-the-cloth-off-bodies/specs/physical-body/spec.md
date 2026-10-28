## ADDED Requirements

### Requirement: Cloth body contact pushes triangles
A cloth SHALL visit each body's spheres from the last to the first, each padded by 2 cm and swept from its last to its current place, and walk its triangles with the swept query over each triangle's smallest sphere at the last tick and now. It SHALL take a triangle when the sphere started in front of the face and ends within the padded radius in front of it, or started behind and ends within that radius behind it.

#### Scenario: Out of reach
- **WHEN** a sphere of padded radius 5 cm stands 10 cm over a triangle
- **THEN** the walk returns the triangle, but the triangle is not taken and nothing is pushed

### Requirement: Cloth body push shares by weight
For a taken triangle, the sweep's midpoint SHALL be put on the face, and the triangle's closest point to it SHALL be found with three weights. The depth SHALL be the sphere's end along the direction from its start to that point, plus the radius, less the point's. Each vertex SHALL be pushed along that direction by the depth times its weight over the largest weight.

#### Scenario: A sphere over the face
- **WHEN** a sphere of padded radius 5 cm stands 3 cm over the middle of a triangle whose closest-point weights are 0.5, 0.25 and 0.25
- **THEN** the three vertices are pushed 2, 1 and 1 cm away from it

#### Scenario: A sweep through the face
- **WHEN** a sphere of padded radius 5 cm sweeps from 4 cm in front of a face to 4 cm behind it
- **THEN** the vertex of weight 0.5 is pushed 9 cm along the sweep

### Requirement: Cloth push buffer
Each body push SHALL go into a per-vertex buffer. For a push pointing along the buffered one, the buffer SHALL add only what goes beyond the longer one's projection.

#### Scenario: The same way twice
- **WHEN** a vertex buffered 1 cm along x takes a push of 2 cm along x
- **THEN** its buffer is 2 cm along x

### Requirement: Cloth countdown trade
After a body has pushed a cloth, the lower of the two sleep countdowns SHALL take the higher. A sleeping body SHALL NOT wake a cloth or keep it awake.

#### Scenario: Sleeping and moving bodies
- **WHEN** a sleeping cloth lies under a sleeping body's sphere, and then under a moving body's sphere with countdown 256
- **THEN** it stays asleep and unmoved under the first, and wakes with countdown 256 under the second

### Requirement: Cloth push pass
After the body stage, an awake cloth SHALL apply its push buffer. A vertex with a push of at least 10⁻⁶ cm² SHALL first have its travel across the push cancelled, when that travel's square is at most 0.2 cm², or else shortened by √0.1 cm. It SHALL then move out by the push. The push SHALL become the vertex's plane for three more constraint passes, each of which SHALL end by putting a vertex behind its plane back on it. The buffer SHALL then be cleared.

#### Scenario: Stopped and shortened travel
- **WHEN** a vertex pushed 2 cm up has travelled 0.3 cm across the push, and another has travelled 1 cm across it
- **THEN** the first returns across to where it was, and the second keeps 1 − √0.1 cm of its travel; both move 2 cm up

#### Scenario: A cloth resting on a sleeping prop
- **WHEN** a loosely pinned cloth is bulged 1 cm by a sleeping body's sphere
- **THEN** it settles on the sphere and falls asleep

## MODIFIED Requirements

### Requirement: Cloth body sphere cull
Each body part offered to a cloth SHALL be handed over with its body's countdown and its spheres from the last to the first, each padded by 2 cm. A sphere SHALL be kept only when it reaches the box around the cloth's particles. A sphere that moved more than 0.01 cm² over the tick (compared in double precision) SHALL be swept, and its sweep SHALL meet the box. Any other sphere SHALL be tested where the last tick left it. A part with no kept sphere SHALL NOT be handed over.

#### Scenario: A narrow box
- **WHEN** the cloth's box spans x = −74 to −46 cm and a 5 cm sphere sits at −80 cm and a 10 cm sphere at −100 cm
- **THEN** only the 5 cm sphere is kept, with a padded radius of 7 cm

### Requirement: Cloth level contact
After its step, a cloth SHALL meet each static mesh in order, particle by particle. A particle that moved over 0.1 cm² SHALL sweep a 2 cm sphere from its last position. Any other SHALL test a 2 cm sphere where it is. The sphere SHALL be tested first against the mesh's box, then against the front faces of the mesh's triangles near it, in list order, each push moving it before the next. A level push SHALL NOT wake the cloth.

#### Scenario: A falling particle
- **WHEN** a particle sweeps from 8 cm to −8 cm through a floor
- **THEN** it is put back 2 cm over the floor and slides to 2.1 cm

#### Scenario: A large floor
- **WHEN** a still particle sits 1 cm over one strip of an 80-triangle floor
- **THEN** it ends at 1.9 cm after fewer than 20 triangle tests

### Requirement: Cloth sleep reads the velocity sum
The cloth's countdown SHALL read the squared length of its particles' summed velocity from the last tick, before drag, gravity and wind. It SHALL count down under √(vertices × 10⁻⁶). It SHALL count up over twice that, squared, and only a count that rose SHALL be capped at 32. At the tick's end, an awake cloth whose count is under 2 SHALL fall asleep where it is. The rustle and the drag's mean velocity (the sum times the reciprocal of the vertex count) SHALL read the same sum.

#### Scenario: Held against gravity
- **WHEN** one particle rests on a plane under gravity for 33 ticks
- **THEN** its countdown reaches 0

### Requirement: Cloth stage counters
With `EXA_DEBUG_CLOTH=1`, the cloth stage SHALL log once a second (83 ticks) the per-tick means of its time and work, then one line per cloth for the last tick. The work counted SHALL include:
- awake, sleeping and skipped cloths;
- body parts offered and handed over;
- spheres kept and swept;
- the body stage's triangle tests;
- static records and own-node skips;
- level pieces, triangles and particle-triangle tests.

Without the flag, the stage SHALL take no timings.

#### Scenario: The level stage's tests
- **WHEN** two of three particles reach a level piece's box holding one triangle
- **THEN** the cloth's level-test count is 2
