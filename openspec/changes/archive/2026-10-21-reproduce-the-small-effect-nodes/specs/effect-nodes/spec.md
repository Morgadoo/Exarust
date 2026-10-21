## ADDED Requirements

### Requirement: Effect clock
The system SHALL step the small effects on the original's game clock in whole milliseconds and SHALL take a span between the clock and a stamp as a signed difference of the two widened words.

#### Scenario: Two ticks
- **WHEN** two 83 Hz ticks of game time have passed
- **THEN** the clock reads 24 ms

#### Scenario: A stamp ahead
- **WHEN** a shell is stamped 100 ms after the clock
- **THEN** its shown fraction is −0.25

### Requirement: Domain wheel
The system SHALL place the domain wheel's six icons clockwise from the top of a 76 px ring, Mind, Force, Body, Energy, Light and Displacement, and SHALL hit an icon when the squared whole-pixel distance to its rounded place is under 433.

#### Scenario: The first icon
- **WHEN** the wheel's centre is (100, 100)
- **THEN** icon 1 is at (100, 24) and icon 2 at (166, 62)

#### Scenario: The hit edge
- **WHEN** the mouse is 12 px right of and 17 px below icon 1
- **THEN** no icon is hit

### Requirement: Displacement shell fade
The system SHALL give the displacement shell's shader the fade (1 − shown²)².

#### Scenario: A quarter shown
- **WHEN** the shell is a quarter shown
- **THEN** the fade is 0.87890625

### Requirement: Teleport bubble fade
The system SHALL upload the bubble's own value as its fade while it shrinks, and max(1 − value, 0) before.

#### Scenario: Shrinking halfway
- **WHEN** a shrinking bubble's value is −0.5
- **THEN** its fade is −0.5

### Requirement: Flame row lag
The system SHALL move each lagging point of a flame row a fifth of the way to a point 20 cm above its end on the up axis each tick.

#### Scenario: From the origin
- **WHEN** a lagging point at the origin follows an end at (10, 0, 0)
- **THEN** it moves to (2, 4, 0)

### Requirement: Portal dissolve seeding
The system SHALL jitter each seeded dissolve particle's position by up to 1.5 cm per axis and push it 1.5 % further from the body's centre, keeping its previous position at the cell's point.

#### Scenario: The first cell
- **WHEN** seed 1 seeds a 12 cm cube at the origin around a body centred at the origin
- **THEN** the first particle's previous position is (−6, −6, −6) and its position x is about −6.343

### Requirement: Level portal light
The system SHALL give a loaded level portal marker a light of colour (0.49, 0.69, 1), intensity 0.75, radius 500 cm and softness 3, 50 cm above it.

#### Scenario: A loaded marker
- **WHEN** the game sees a new portal marker node
- **THEN** one portal light hangs under it 0.5 m up
