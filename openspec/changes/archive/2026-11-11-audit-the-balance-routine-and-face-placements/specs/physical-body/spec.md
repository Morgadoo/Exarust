## ADDED Requirements

### Requirement: Balance pass grip reset
The balance routine's feet-centre pass SHALL put every leg's grip back at 1 each tick, for a body with two legs or more, before the solver's iterations zero it on an anchored foot and raise it on a touching one.

#### Scenario: A grip left low
- **WHEN** a standing body's legs have grip 0.2 before its balance routine runs
- **THEN** every leg's grip is 1 after it

### Requirement: Balance floor clears the step
With the balance under 0.2, compared in double, the balance routine SHALL clear a running step (its stepping, knockback and out-of-balance state) without the step's end: no foot is landed and the standing position stays. A NaN balance SHALL not be under the floor.

#### Scenario: Balance falls during a step
- **WHEN** a step runs and the balance is 0.19
- **THEN** the step is cleared and not ended, and its out-of-balance bit is off

### Requirement: Placement facing
A placement SHALL take the heading as its matrix's forward flattened and normalised by the approximate reciprocal square root (a zero one kept zero), copy it to the stance facing, take the body's yaw as the engine's heading of it, and write the raw forward's heading as the cursor's smoothed and raw directions and the look.

#### Scenario: A tilted forward
- **WHEN** a body is placed with the forward (1, 0.5, 1)
- **THEN** its yaw and request heading are 0.785179 rad (the engine's arc cosine), and its cursor and look the raw forward's

### Requirement: Placement facing callers
The placement on the ground SHALL write the placement facing from its frame; a fighter's spawn SHALL write it from its model's forward, with the twist reset to the identity and the move vector's reach to 0.

#### Scenario: A ground placement
- **WHEN** a body with a twist and a reach is placed on the ground facing (−0.6, 0, 0.8)
- **THEN** its yaw is about −0.6435 rad and its twist and reach are kept

### Requirement: Whole capture keeps the thighs
A whole capture SHALL leave the thighs (the parents of the parts with foot contacts) at their last target, the clip turn before the femur posture, instead of their measured turn; a plain capture keeps its lower body's targets as before.

#### Scenario: Thighs in a whole capture
- **WHEN** a whole capture is taken with the thighs' physical turns measured
- **THEN** the thighs keep their targets and every other part takes its measured turn

### Requirement: Stab drive guide normalisation
The stab drive's guide direction SHALL be normalised by the approximate reciprocal square root.

#### Scenario: A guide
- **WHEN** the stab drive's guide runs from the frame's offset to its guide point
- **THEN** its direction is that offset scaled by RSQRTSS of its squared length

## MODIFIED Requirements

### Requirement: Foot grip
In each iteration, after its lower leg's links, a foot of a body over half its tone with support, more than 10 cm under the pelvis, SHALL be driven to its anchor: touching and not lifted, by the PD grip d·(0.1p + 0.004g(1 − p)) less its motion across the ground × (p + 0.03g(1 − p)), or walking by the velocity servo (never down); an anchored foot reaches it within 5 cm; a foot not touching or lifted only up. Its anchor SHALL sit on its ground plus the sole height.

#### Scenario: A quarter turn in place
- **WHEN** a standing body's pose turns 90° about the vertical over 60 ticks
- **THEN** its feet step to their new spots one at a time, and a foot planted for 50 ticks stays within 1 cm of where it stands

#### Scenario: A lifted foot
- **WHEN** a lifted foot, or one that does not touch, is 3 cm under its anchor and 5 cm aside
- **THEN** it moves only up, and an anchor under it moves it not at all

#### Scenario: The grip
- **WHEN** a foot touches for 250, then 1000 iterations
- **THEN** its grip is 0.25, then 1, and a full grip pulls 2 cm of gap by 0.2 cm and takes back its motion across the ground; 5 cm over its anchor the hold is halved, past 9 cm gone

#### Scenario: A light shove
- **WHEN** a standing body's torso takes a 2 m/s sideways blow
- **THEN** its chest sways about 2 cm and its feet stay within 1 cm of their anchors

#### Scenario: A step's reach
- **WHEN** a stepping foot comes 4.9 cm from its anchor
- **THEN** it has reached it, counts a landing, and its anchor moves halfway to it
