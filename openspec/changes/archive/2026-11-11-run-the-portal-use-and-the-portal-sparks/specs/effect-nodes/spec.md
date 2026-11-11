## ADDED Requirements

### Requirement: Level portal sparks
A level portal marker loaded into the game's scene SHALL become a `_PortalPrt` particle system at its place: the template of 140 particles a second living 1.5 s, half size 36 cm, speed 16 cm/s, velocity tripling a second, growth 1, stretch 1.5, spread 1, a pool of 2048, flags `0x4400_2B01`, six blue colour keys, the `Portal01` loop at 0.1, scale 1 and the portal effect's own spawn; it SHALL draw with `_PrtlPrt01` (`PrtDot02`, ONE and ONE).

#### Scenario: A loaded marker's sparks
- **WHEN** the game sees a new portal marker node
- **THEN** its spark system has a ring of 2048, lives of 124 ticks and names the `Portal01` loop

### Requirement: Portal use
A portal use on a body SHALL build a `_PrtlDisp` particle system (60 a second, 0.5 s lives, half size 36 cm, speed 48 cm/s, growth 1, stretch 2, spread 1, a pool of 2048, flags `0x4000_2B11`, six keys, no light, no sound) at scale 1, seed it over the body's model nodes around the body's centre, and play `portaluse01` at the body (volume 0.6, pitch 1, flags 0). The seeded particles SHALL then age and fly as the system's own and draw with `_PrtlPrt01`.

#### Scenario: The player uses a portal
- **WHEN** the portal use runs on a body of two parts
- **THEN** one `_PrtlDisp` system holds the seeded particles over both parts' boxes

## MODIFIED Requirements

### Requirement: Level portal light
The system SHALL give a loaded level portal marker's spark system a light of colour (0.49, 0.69, 1), intensity 0.75, radius 500 cm and softness 3, 50 cm above it.

#### Scenario: A loaded marker
- **WHEN** the game sees a new portal marker node
- **THEN** one spark system hangs under it at its place and one portal light under that system 0.5 m up

### Requirement: Portal spray particle
The level portal effect's spawn SHALL move the ring index on before it fills a slot; past the capacity it SHALL clear the state word's `0x2` bit and start again at slot 0 unless the ring's head is at slot 1, which parks the index at the capacity and spawns nothing; an index landing just before the head SHALL step back and spawn nothing. Each particle SHALL lie on an ellipse of 80·s·scale by 115·s·scale cm (s = 0.8 + 0.4·r) at the node's place, its previous position the point and its position 1.0015 times the point.

#### Scenario: The first particles
- **WHEN** a ring of capacity three with its head at 0 spawns four times
- **THEN** it fills slots 1, 2, 3 and then 0

### Requirement: Portal import and dissolve ring
A portal marker loaded into a world whose flags carry `0x10000` SHALL be replaced by a fresh marker at its place keeping its render value, its 4-byte payload read and unused and a shorter payload refused; a seeding pass SHALL empty the system's ring, write its seeded particles into the slots from slot 0 in order up to the capacity, measure the system's box over them and clear the state word's `0x2` bit.

#### Scenario: A short payload
- **WHEN** a marker carrying the import flag has a 3-byte payload
- **THEN** no marker is made

#### Scenario: A full ring
- **WHEN** five particles are seeded into a ring of capacity three
- **THEN** three are written, in order, and the box spans them with 5 cm to spare

### Requirement: Portal spray life
A portal spray particle SHALL start with the particle system's full lifetime and keep its slot's random size, and its look index SHALL be written only under the 3-bit or 5-bit look flags.

#### Scenario: No look flag
- **WHEN** a spray without look flags spawns into a slot whose look is 7
- **THEN** the slot's life is the system's lifetime and its look stays 7
