## MODIFIED Requirements

### Requirement: Placement on the ground
A placement on the ground SHALL run the leg snap from its frame and take its frame height, put every part at rest at the current clip's pose-only sample lowered by the pelvis's posed height onto that frame, anchor each foot at its posed contact on the frame less the clip root at its ground plus the sole and clear its state, restart the chest history and the feet centre, and restart every slack link's smoothed length. With no ground under any foot the body SHALL be left as it was.

#### Scenario: A platform
- **WHEN** a biped is placed on the ground over a platform 15 cm up
- **THEN** its pelvis stands 15 cm higher than over the floor, at the snap's height, every part at rest at its posed offset from the pelvis

#### Scenario: No chest velocity from the move
- **WHEN** a body whose chest was moving is placed on the ground
- **THEN** its chest velocities are zero, its next chest step reads none, every foot state is clear and its feet centre is its anchors' mean

### Requirement: Character frame
A living body's targets SHALL be its clip pose on a frame that turns with the body, its origin the pelvis less the pose's root offset across the ground, at the pelvis's height. Each character tick it SHALL follow the pelvis: moving or stepping (any step episode), onto the pelvis; without support, exactly; standing, past 5 cm while support is under 0.95, else not at all; down (balance under 0.3), past 5 cm; getting up, past 10 cm in the clip's first quarter, then not at all.

#### Scenario: Steady stance
- **WHEN** a standing body at support 1 is pushed 9 cm across the floor
- **THEN** its frame stays put and it comes to rest within 1 cm of its spot

#### Scenario: Unsteady body
- **WHEN** a standing body at support 0.95 is pushed 14 cm across the floor
- **THEN** its frame follows it, and it comes to rest no more than 6 cm back from the farthest it went, within 2 cm of where its frame puts its pelvis

#### Scenario: Support threshold
- **WHEN** a standing body's support is 0.95 as a float, which is under the double 0.95, or the next float up
- **THEN** its frame follows past 5 cm, or stays put

#### Scenario: Moving
- **WHEN** a body moves
- **THEN** its frame's origin is its pelvis origin every tick

#### Scenario: Stepping
- **WHEN** a steady body at support 1 runs a step episode
- **THEN** its frame's origin is its pelvis origin every tick

#### Scenario: Getting up
- **WHEN** a get-up's pose puts the pelvis 30 cm off the frame's origin in the clip's first quarter
- **THEN** the frame moves under the body at once; later the pelvis 8 cm off leaves it, 12 cm moves it, and past the first quarter it stays

#### Scenario: Standing in the arena
- **WHEN** `exarust --trace t.csv play arena` stands the player still
- **THEN** its frame does not move while its support is 0.95 or more

#### Scenario: Far from the origin
- **WHEN** a steady standing body 90 m from the world's origin stands for 600 ticks
- **THEN** its frame does not move at all

## ADDED Requirements

### Requirement: Footwork step on the body
The player's combat footwork SHALL read and write the body's step (stepping, balance step, target, long kind, queue and queued vector); the body SHALL walk it on its feet and the character SHALL follow the body's frame. The movement decision's gate, the leg muscles' anchored-foot rule and the lift's standing lead SHALL read the body's own step episode.

#### Scenario: A footwork step refuses the walk
- **WHEN** the player's footwork starts the body's step and the movement decision runs
- **THEN** the decision refuses the move as stepping and the character takes no velocity of its own

### Requirement: Step episode flags
A body's step episode SHALL keep whether it is a balance step: set when the balance routine starts a step from rest, cleared by any other start, by the step's end and on any tick without a step; a tick without a step also clears the long kind. It SHALL keep a queue word: 1 while a queued step waits, the countdown once taken, 0 for none; a knockback SHALL clear it.

#### Scenario: A queue that is not waiting
- **WHEN** a foot lands while the queue word is 0 or a taken countdown
- **THEN** no queued step is taken
