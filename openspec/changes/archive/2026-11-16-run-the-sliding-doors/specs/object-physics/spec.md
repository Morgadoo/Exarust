## MODIFIED Requirements

### Requirement: Props registered as bodies
The system SHALL register every free or hung body tree (a top body and its body children) of a level's props, placed objects, and own model: asleep (countdown 0) at its nodes' world matrices, at rest, its mobility the stored over its divisor, gravity by shares 4/mᵢ scaled by 4 over their sum over all four masses, the stored spheres against the level with friction 0.75. Mechanisms SHALL register with their rules, sliding door leaves included, unless kept static. A blow SHALL wake a body.

#### Scenario: First level
- **WHEN** `exanima01` loads
- **THEN** 4 911 trees of 6 235 bodies register asleep, 320 of them hung from the world (109 hinged door leaves, 40 lever arms, 12 push surfaces and 37 sliding leaves among them), and no mechanism stays static

#### Scenario: Sliding leaves kept static
- **WHEN** `exanima01` loads with its sliding doors kept static
- **THEN** 4 874 trees register and its 37 sliding leaves stay level geometry

#### Scenario: Asleep where placed
- **WHEN** a registered prop 30 cm over the floor is ticked 500 times
- **THEN** its particles stay exactly where its node placed them and it stays asleep

### Requirement: Sliding leaf body
A door's sliding leaf SHALL register on its slider links with the door's sliding rules, its travel link the first whose body point is 30–160 cm up, reading shut at a fraction of 0.995 or more; each awake tick, right after the leaf's integration and before the link passes, it SHALL feed the link's fraction to the sliding step and write the step's fraction and smoothing back, a command's fraction written into the link first.

#### Scenario: Shut
- **WHEN** a leaf at the end of its rails is woken and left 600 ticks
- **THEN** it stays shut where it is and sleeps

#### Scenario: An open in the tick it is written
- **WHEN** a plain leaf's open jumps its fraction to 0.6 and it ticks once
- **THEN** the leaf has moved toward the jump in that tick

## ADDED Requirements

### Requirement: Sliding leaf commands
An open command SHALL wake the leaf's body to 256 each tick it runs. On level rails a close SHALL slide a leaf from the open end into its shut place, where it slams once, latches when it latches, and is held at fraction 1 with smoothing 0.5; a motorised open SHALL slide it back toward the open end.

#### Scenario: A level leaf closed and opened
- **WHEN** a latching leaf at the open end of 1.7 m level rails takes a close, and a motorised one shut there takes an open
- **THEN** the first travels 1.7 m, slams once and latches, and the second moves back more than 0.3 m with its body awake throughout

### Requirement: Sliding leaf hand drag
The hand's press SHALL add the door's drag link at the hit of a sliding leaf (a latched one rattles and still drags); each frame the drag target SHALL be the link's point moved by the cursor's offset along the leaf's own x axis, and each tick the link's anchor SHALL be that target kept within 60 cm of the link's point; out of reach the link SHALL be disabled, and the release SHALL remove it.

#### Scenario: Dragged along its rail
- **WHEN** a level leaf is held and the cursor stays 2 m along its rail and 1 m across for 300 ticks
- **THEN** the anchor stays on the rail's line within 60 cm of the link's point, the leaf moves along its rail, and the release removes the link

### Requirement: Shipped portcullises
exanima01's 37 sliding leaves SHALL lie at the open end of level rails at load and read open; woken and left alone, each SHALL creep less than 2 cm and sleep; the router's close of an unlatched fast leaf SHALL slide it 1.7 m into its frame, where it reads shut.

#### Scenario: Signalled shut
- **WHEN** the unlatched fast portcullises of `exanima01` are toggled by signal
- **THEN** each travels 1.7 m along its rail and reads shut

### Requirement: Door motor arrival when unordered
A toggle SHALL arrive unless its step is less than the absolute error, and an open command unless the absolute error is more than 0.1 rad, so an unordered (NaN) error arrives; a zero range SHALL give an infinite plain step.

#### Scenario: A NaN angle
- **WHEN** a toggle and an open command run with a NaN angle
- **THEN** both arrive
