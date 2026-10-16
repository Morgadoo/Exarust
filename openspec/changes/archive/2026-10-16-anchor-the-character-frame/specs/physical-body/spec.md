## ADDED Requirements

### Requirement: Character frame
A living body's targets SHALL be its clip pose on a frame that turns with the body, its origin the pelvis less the pose's root offset across the ground, at the pelvis's height. Each character tick it SHALL follow the pelvis: moving, onto the pelvis; without support, exactly; standing, past 5 cm while support is under 0.95, else not at all; down (balance under 0.3), past 5 cm; getting up, past 10 cm in the clip's first quarter, then not at all.

#### Scenario: Steady stance
- **WHEN** a standing body at support 1 is pushed 9 cm across the floor
- **THEN** its frame stays put and it comes to rest within 1 cm of its spot

#### Scenario: Unsteady body
- **WHEN** a standing body at support 0.95 is pushed 14 cm across the floor
- **THEN** its frame follows it, and it comes to rest no more than 5 cm back from the farthest it went, where its frame puts its pelvis

#### Scenario: Support threshold
- **WHEN** a standing body's support is 0.95 as a float, which is under the double 0.95, or the next float up
- **THEN** its frame follows past 5 cm, or stays put

#### Scenario: Moving
- **WHEN** a body moves
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

### Requirement: Controller follows the frame
A living character's controller SHALL follow its body's frame (Exarust's own): each fixed step, before its own walk, it moves its feet a tenth of the way across the ground to where the frame's origin puts the body root, at most 2 cm, through the controller. The body's carry SHALL leave that move out.

#### Scenario: Glide
- **WHEN** the frame's origin is 50 cm from the feet
- **THEN** the feet go 2 cm a step until it is 20 cm away, then a tenth of the rest, and stop within a millimetre of it

#### Scenario: Follow left out of the carry
- **WHEN** a standing body's root moves 30 cm across the ground, by the controller's follow or by its walk
- **THEN** the body and its frame stay where they are, or are carried 30 cm

#### Scenario: Shoved over
- **WHEN** `exarust --trace t.csv --shove 60 play arena` knocks the player over and it gets up
- **THEN** its controller follows the falling body, never moves more than 2 cm a tick, and comes to stand at the frame's origin

## MODIFIED Requirements

### Requirement: Living bodies in motion
The system SHALL give a living physical body the animated pose of each tick on its character frame. A standing body SHALL be carried with its root's motion since the last tick, less its controller's follow, across the ground and about the vertical only, with where the last tick left it, its level contacts, last tick's targets and its frame, and its pelvis held over the frame's by 0.168 × tone² × energy⁴ a tick, critically damped, only damped while it moves (Exarust's own).

#### Scenario: Turning far from the origin
- **WHEN** a living body 80 m from the world's origin turns with its pose
- **THEN** its particles move no faster than the pose's own motion

#### Scenario: Carried by the controller
- **WHEN** the root of a body standing on a floor moves across it and down
- **THEN** its particles, where the last tick left each part and its level contacts move with the root across the floor but not down, while where it is drawn from stays

#### Scenario: Held on the controller
- **WHEN** a standing body's pelvis is off where its frame puts the pose's pelvis
- **THEN** it is pulled back across the ground, and the hold never moves it up or down; while the body moves it is only braked

### Requirement: Falling and getting up
The system SHALL make a standing body whose balance was below 0.6 last tick fall: it loses its controller's support and hold, keeps its tone, plays Fall Forward when its lean points ahead of it and Fall Backward otherwise, and it is not steered. Once it has lain for 2.5 s and its energy is back to 0.9, it SHALL get up with the get-up clip whose first pose lies the way the body does, turned that way; its frame re-anchors to the pelvis, and its controller follows the frame.

#### Scenario: Knocked over
- **WHEN** `exarust --shove 60 play arena` deals the player a body blow
- **THEN** the player falls onto its back, lies a moment, gets up with `Get Up 2` without a hop, and stands where it fell

### Requirement: Clips sampled every tick
The system SHALL advance a living physical body's clip clock by one tick, at 60 frames per second, and sample its clip inside the body's character tick, not once a frame. A clip that ends into its follow-up SHALL give the follow-up's first pose in that tick. Bodies that are not physical SHALL keep playing their clips once a frame.

#### Scenario: Clip clock at any frame rate
- **WHEN** a physical body plays a clip for 60 ticks at 30, 60, 144 or 240 FPS
- **THEN** its clock advances 60/83 of a frame per tick, and its targets after each tick are the clip's pose at that tick on the body's frame

#### Scenario: Frames leave the clip alone
- **WHEN** frames pass on a physical body's clip
- **THEN** only its ticks advance the clock

### Requirement: Body trace
`exarust --trace <file.csv>` SHALL write one row per traced body (the player's, or else every simulated body) after every tick, with the retail body probe's columns: the pelvis particles; its heights (the lift's target included), tone, energy, support, balance, counter; every part's target; the foot contacts; the motion and clock; then its sleep countdown and state, its frame's origin and its body root. Lengths in cm, the clock in clip frames; columns not simulated yet stay empty.

#### Scenario: Standing in the arena
- **WHEN** `exarust --trace t.csv play arena` runs for a minute
- **THEN** `t.csv` has one row per tick for the player's body, its clock advancing 60/83 of a frame per row, and its pelvis 1.7 cm over its target pelvis height
