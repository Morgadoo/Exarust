## ADDED Requirements

### Requirement: Door shape
A link of at most 30 cm of slack whose body point is farther than 1 cm SHALL replace a door's hinge axis, the first one always and later ones only when they do not point down; the axis SHALL start straight up. The last longer link SHALL be the swing stop, with radius r the length of its body point and half swing α twice the engine's fast arcsine of half the slack over r.

#### Scenario: The shipped stop
- **WHEN** a pin points 242.9 cm up and the stop's point is 164.8 cm to the negative side, 0.7 cm back, with 134.04 cm of slack
- **THEN** the radius is 164.80 cm, the half swing is 0.83806 rad and the axis points up

### Requirement: Door frame
A stop whose point lies on the positive side SHALL make a door's swing sign negative, and a later stop SHALL not set it back. The frame's zero SHALL be the stop's anchor, so the closed angle SHALL be the half swing α and the range SHALL be −2α times the sign.

#### Scenario: Closed and the range
- **WHEN** the shipped stop is on the negative side, and a second door has its stop 164.8 cm to the positive side
- **THEN** the first door's closed angle is 0.83806 rad and its range −1.67612, and the second door's sign is −1 with a range of 1.6761

### Requirement: Door stored word
A door's stored word SHALL be written only by its commands: the mode in its low byte and the countdown in its top byte, read back unsigned. A tick's countdown SHALL not be written back.

#### Scenario: A command writes the word
- **WHEN** a door is toggled for 100 ticks and then steps once
- **THEN** the stored word holds mode 8 and countdown 100

### Requirement: Lever shape
A lever's links SHALL decide its swing. The last link of at most 1 cm of slack whose body point is farther than 1 cm SHALL give the axle, negated. The last longer link SHALL be the arm: radius r and half swing α, twice the engine's fast arcsine of half the slack over r. The frame's zero SHALL be the arm's anchor and the two detents SHALL be at −α and +α.

#### Scenario: Two detents around the anchor
- **WHEN** a lever's arm link is tied at its anchor with a slack that gives a half swing α
- **THEN** its first detent is −α and its second is +α

### Requirement: Operative load setup
At level load an operative record with flag 0x40 SHALL switch its node's effects off and SHALL still run its kind's setup. A light job whose value is odd SHALL start an electric effect. A fire record of class 8 SHALL light a fire whose size is the low half and whose lift (cm) is the high half of its first target word. A load fire's size SHALL be hundredths of the stored size, at least 0.25 and at most 10.

#### Scenario: A flagged fire
- **WHEN** a fire record with flag 0x40 and a first target word holding 250 and 40 is set up at daylight 0
- **THEN** the effects are switched off and a fire of size 250 with a lift of 40 cm is lit

### Requirement: Daylit fire
A fire record set up when the daylight exceeds 0.12 SHALL light no fire; the first light node under it SHALL be put out, or that node's parent when the parent is lit, or the node itself when there is no light node.

#### Scenario: The light or its lit parent
- **WHEN** a fire record is set up at daylight 0.12 and at 0.13, under a lit node that holds a light node, and again when that parent is not lit
- **THEN** at 0.12 a fire is lit, at 0.13 the lit parent is put out, and without a lit parent the light node is

### Requirement: Signalled fire size
A signalled fire job SHALL take hundredths of its stored value as its size, or 2 when the value is 0, without a clamp.

#### Scenario: A zero size and a large one
- **WHEN** fire jobs of stored size 0, 20 and 2000 are signalled
- **THEN** their sizes are 2, 0.2 and 20

### Requirement: Node word latch
Latching the doors under a node SHALL write the value into bit 0 of the node's word even when there is no door. Closing and locking them SHALL set that bit for every leaf that is already shut.

#### Scenario: A latch with no door, and a shut leaf
- **WHEN** the doors under a node with no door are latched, and two shut sliders under a node whose word is 16 are closed and locked
- **THEN** the first word is 1 and the second is 17

### Requirement: Placed mechanism records
A placed object with a door or an operative section SHALL keep both records under its placed id, tag bits included, and a signal to it SHALL be dispatched by those records. Its body SHALL stay static level geometry while the solver cannot hold a body hung from the world.

#### Scenario: An inverted door and a fire job
- **WHEN** a placed inverted, motorised door is signalled 1, 0 and −1, and a placed fire job of size 200 is signalled 1
- **THEN** the door closes and locks, opens, and toggles, and the fire job starts a fire of size 200

## MODIFIED Requirements

### Requirement: Door partner
A single shut door SHALL stay shut while its angle is no more than 0.05 rad short of closed, compared in double precision, and SHALL leave once it is farther and not latched, while still being held. A paired door whose partner is not within 0.1 rad SHALL not be held, and SHALL leave when its own angle is short of closed. A near partner SHALL keep it shut and held. Exactly 0.1 rad short SHALL not count as near, and past closed SHALL. A paired door SHALL add the pair bits to its layer.

#### Scenario: One leaf open, the other not held
- **WHEN** a single door with closed 1 is at 0.95000005 and then at the float nearest 0.95, another is 0.06 rad short and unlatched, a paired door 0.01 rad short has a partner that is not near, a paired door has a near partner, an angle is exactly 0.1 rad short and another is past closed, and a paired layer starts at 0
- **THEN** the single door stays shut at 0.95000005 and leaves at the float nearest 0.95 (it is past the double 0.05 line) while still held, the open partner's leaf leaves the shut state and is not held, the near partner's leaf stays shut and held, exactly 0.1 rad short is not near, past closed is near, and the layer is the pair bits

### Requirement: Lever signal
Arriving more than 0.1 rad from the last detent SHALL report the rest flag at the first detent and its opposite at the second. A report of 0 SHALL signal 0 unless inverted, SHALL stop a return and SHALL stop the tick loop. A report of 1 SHALL signal 1, or -1 when forced and inverted, and SHALL arm a timed return. A linked light SHALL fade in on 0 and out on 1, as detent 0 starts it at full. A report of 0 with a light SHALL not signal.

#### Scenario: An arrival signals, and a light fades
- **WHEN** a lever arrives at the first detent from a last detent of 1 with the rest flag clear, another arrives at the second detent with a return time, a held lever is forced and inverted on the first detent with the rest flag set, and a lit lever arrives at the first detent
- **THEN** the first signals 0, the second signals 1 and arms the return, the held lever signals -1, and the lit lever fades its light in without a signal

### Requirement: Plate travel
Every tick SHALL set each of the plate's links' smoothing to 0.1. A pressed plate SHALL raise each travel by 0.02, at most 1, and a free plate SHALL lower each by 0.01, at least 0, each step taken in double precision and stored as a float. A one-shot SHALL stop being pressed once a travel passes 1, and later travels that tick SHALL fall. The mean SHALL be written back to every travel. Grind speed SHALL be 0.9 times the mean's change plus 0.1 times the old speed, in double precision, and SHALL stop at 0.001 or below, compared in double precision. Otherwise volume SHALL be 50 times the speed, squared, pitch SHALL be 20 times the speed plus 0.8, at most 1, and the body SHALL stay awake for 256 ticks.

#### Scenario: A rise, a grind, and a one-shot
- **WHEN** a pressed plate starts at travel 0, a still plate's old speed leaves it at the float nearest 0.001, and a one-shot plate with travels 0.99 and 0.5 is pressed
- **THEN** the first mean is 0.02 with grind volume 0.81 and pitch 1 and the body awake, the plate at the float nearest 0.001 still grinds, and the one-shot is no longer pressed with both travels equal

### Requirement: Plate click
A plate's state SHALL be the object word's low byte as stored. A mean above 0.7 SHALL press a plate whose state is 0, click, and signal 1, or -1 when inverted. A mean below 0.5 SHALL release a plate whose state is 1 and signal 0, unless inverted. Any other state SHALL neither press nor release. A travel of 0.68 that rises SHALL not press. A travel of 0.51 that falls SHALL not release.

#### Scenario: The click and the two edges
- **WHEN** a pressed travel of 0.68 steps, a pressed travel of 0.69 steps, a down plate falls from 0.51, a down plate falls from 0.5, and a plate whose state is 2 is pressed from 0.99 and then falls from 0
- **THEN** 0.68 stays up, 0.69 clicks and signals 1, 0.51 stays down, 0.5 releases and signals 0, and the plate in state 2 neither clicks nor signals and keeps state 2
