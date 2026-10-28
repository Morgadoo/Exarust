## ADDED Requirements

### Requirement: Start capture half-lean
A plain start capture SHALL replace the physical turn of the lumbar and the thorax (the midline bones between the legs and the head) with that turn whose forward axis is moved halfway toward the same turn taken out of the twist frame, whose up axis is made square to that forward and whose side is their cross product. Whole captures and the held pose SHALL keep the measured turns.

#### Scenario: Twisted upright spine
- **WHEN** a plain start captures an upright thorax while the twist is 0.6 rad of yaw
- **THEN** the captured thorax turn is −0.3 rad of yaw

### Requirement: Half-lean twist choice
A start made before the tick's twist block SHALL half-lean against the previous tick's twist; a follow-up started by the clip clock SHALL use the current tick's.

#### Scenario: Start before the twist
- **WHEN** the twist moves from 0.6 to 0.62 rad this tick and the behaviour started a clip earlier in the tick
- **THEN** the capture is half-leaned against the 0.6 rad twist

### Requirement: Step gate plant request
The non-player fighter's step gate SHALL stay shut while the body's trailing-foot request is nonzero.

#### Scenario: Leaving the walk
- **WHEN** a clip has just replaced Walk and the request's 500 ms deadline has not passed
- **THEN** the fighter does not take a footwork step

### Requirement: Struck item sound
A blow on a held item SHALL set the item's sound clock to the game clock plus 200 ms less 80 times the strength rounded half to even, and SHALL play the item record's class set (1 to 3) when the record has class bits and the blow projects on the item's long axis, over its length, strictly below the record's split, otherwise the item's own sound, at the strength's volume and 1.2 times the pitch.

#### Scenario: Low blow
- **WHEN** a blow of strength 0.5 lands 10 cm up an 80 cm item whose record of class 2 splits at a quarter, at clock 1000
- **THEN** set 2 plays and the item's clock is 1160

### Requirement: Placement resets
Placing a body at a matrix SHALL reset its twist to the identity and its move vector's reach to 0, take its yaw from the matrix's flattened forward by the engine's arc-cosine heading, and hold the legs' knee and hip corrections at the identity until the leg IK writes them again, keeping the corrections' angles.

#### Scenario: Diagonal placement
- **WHEN** a body is placed at a matrix whose forward is (1, 0.5, 1)
- **THEN** its yaw is 0.785179 rad and its twist is the identity
