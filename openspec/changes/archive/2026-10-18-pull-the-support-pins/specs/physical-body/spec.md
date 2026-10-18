# Spec Delta

## MODIFIED Requirements

### Requirement: Living bodies in motion
The system SHALL give a living physical body the animated pose of each tick on its character frame. A standing body SHALL be carried with its root's motion since last tick, less its controller's follow, across the ground and about the vertical only, with where the last tick left it, its level contacts, its feet's anchors, last tick's targets and its frame. No spring of 0.168 × tone² × energy⁴ SHALL pull or brake its pelvis toward that frame.

#### Scenario: Turning far from the origin
- **WHEN** a living body 80 m from the world's origin turns with its pose
- **THEN** its particles move no faster than the pose's own motion

#### Scenario: Carried by the controller
- **WHEN** the root of a body standing on a floor moves across it and down
- **THEN** its particles, where the last tick left each part and its level contacts move with the root across the floor but not down, while where it is drawn from stays

#### Scenario: Held on the controller
- **WHEN** a standing body's pelvis, still or moving, sits off where its frame puts the pose's pelvis
- **THEN** the step does not pull it across the ground by 0.168 × tone² × energy⁴, and does not brake its horizontal velocity by twice the square root of that gain

#### Scenario: A standing turn
- **WHEN** the player turns a quarter standing in the arena
- **THEN** its planted feet are carried round with it, within 0.1 cm of their anchors

### Requirement: Character frame
A living body's targets SHALL be its clip pose on a frame that turns with the body, its origin the pelvis less the pose's root offset across the ground, at the pelvis's height. Each character tick it SHALL follow the pelvis: moving, onto the pelvis; without support, exactly; standing, past 5 cm while support is under 0.95, else not at all; down (balance under 0.3), past 5 cm; getting up, past 10 cm in the clip's first quarter, then not at all.

#### Scenario: Steady stance
- **WHEN** a standing body at support 1 is pushed 9 cm across the floor
- **THEN** its frame stays put and it comes to rest within 1 cm of its spot

#### Scenario: Unsteady body
- **WHEN** a standing body at support 0.95 is pushed 14 cm across the floor
- **THEN** its frame follows it, and it comes to rest no more than 5 cm back from the farthest it went, within 2 cm of where its frame puts its pelvis

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

### Requirement: Foot grip
In each iteration, after its lower leg's links, a foot of a body over half its tone with support, more than 10 cm under the pelvis, SHALL be driven to its anchor: touching and not lifted, by the PD grip d·(0.1p + 0.004g(1 − p)) less its motion across the ground × (p + 0.03g(1 − p)), or walking by the velocity servo (never down); an anchored foot reaches it within 5 cm; a foot not touching or lifted only up. Its anchor SHALL sit on its ground plus the sole height.

#### Scenario: A quarter turn in place
- **WHEN** a standing body's pose turns 90° about the vertical over 60 ticks
- **THEN** its feet step to their new spots one at a time, and a foot planted with its grip full stays within 1 cm of where it stands

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

### Requirement: Climbing with the controller
Until the feet step and the legs bend (Exarust's own), while a living body that has not fallen has its character controller on the ground, standing or moving, the target pelvis height SHALL read each foot's ground no lower than the controller's floor less 6 cm. A controller in the air or under a fallen body SHALL change nothing.

#### Scenario: Level ground
- **WHEN** a body walks or stands with its controller floating 0, 2 or 5 cm over a level floor
- **THEN** its target is the original's every tick: in `--autowalk play arena` the same as without the rule, the pelvis 99.4–102.2 cm walking and 101.5 cm standing

#### Scenario: Stairs
- **WHEN** a body walks up four 18 cm steps with its controller, stopping on the third
- **THEN** it keeps its balance and ends on the landing, its pelvis 72 cm higher; with the original's target alone the support pins keep its balance and its pelvis ends short of the landing

#### Scenario: Stair room
- **WHEN** `exarust --autowalk --at 7.5,-0.95,64.4 play exanima01` or `--at 7.6,-1.3,66.2` walks the player up the stairs to the door
- **THEN** it climbs to the landing without falling on the stairs and stands there

#### Scenario: Perched
- **WHEN** a body's grounded controller stands 12 cm over the floor under its feet, or is in the air
- **THEN** its target is 6 cm higher than the original's, or the original's

#### Scenario: Fallen
- **WHEN** a body has just fallen, its feet on the floor and its grounded controller 12 cm over it
- **THEN** its target is the original's every tick, as with its controller on the floor

## ADDED Requirements

### Requirement: Support pin pull
In each iteration a supported living body SHALL pull its two thorax pins and its pelvis pin horizontally, in centimetres: raw is 0.2·e·|e|/60 when |e| < 60 and 12 along e otherwise, less this tick's horizontal displacement, capped at 8, times S²·pin-gain·P·pin factor, and times 0.3 while getting up. P is √E·T·E⁴. Pin-gain starts at 0.08.

#### Scenario: One application inside 60 cm
- **WHEN** a supported pin is 10 cm from its clamped target, its horizontal displacement this tick is 1 cm along that error, and the body is an alert human (S = 1, pin-gain 0.08, T = 0.5, E = 1, pin factor 1.25) that is not getting up
- **THEN** one application is (0.2·10·10/60 − 1) cm along the error, times 0.05, which is −0.0333 cm

#### Scenario: One application past 60 cm
- **WHEN** the same pin is 80 cm from its target and has not moved this tick
- **THEN** one application is 12 cm along the error, capped at 8 cm, times 0.05, which is 0.4 cm

#### Scenario: Getting up
- **WHEN** the same 10 cm pin with no displacement is on a body that is getting up
- **THEN** one application is 0.3 of the standing pull

#### Scenario: Chest returns
- **WHEN** a supported standing body's chest is held about 10 cm off the feet centre
- **THEN** later ticks of the real step bring the chest back toward the feet centre

### Requirement: Support pin targets
With support, each thorax pin's target SHALL stay within (60 − 60·crouch)·0.6·scale cm of the feet centre, or within (60 − 30·crouch)·0.6·scale cm while a foot is stepping, and the pelvis pin's target within 60·scale cm. Crouch is 0 until a crouch exists, so a standing thorax reach is 36·scale cm.

#### Scenario: Standing reach
- **WHEN** a supported body of scale 1 is not crouching and no foot is stepping, and a thorax target lies 40 cm from the feet centre
- **THEN** the target used by the pull is 36 cm from the feet centre, and a pelvis target 80 cm out is brought in to 60 cm

#### Scenario: Stepping reach
- **WHEN** the same body crouches fully and a foot is stepping
- **THEN** a thorax target is kept within 18 cm of the feet centre

### Requirement: Unsupported pin targets
With no support the pins SHALL apply no restoring pull, and their targets SHALL only shift 5·scale cm forward.

#### Scenario: No pull
- **WHEN** a body has no support and a pin is 10 cm off its target
- **THEN** one application is zero, and the target is the frame's point shifted 5·scale cm forward

### Requirement: Pins on a moving body
While a living body moves, its frame SHALL follow its pelvis, and the pins SHALL not hold the body on one world spot.

#### Scenario: Not nailed down
- **WHEN** a supported body is moving and is given a horizontal velocity
- **THEN** over later ticks its pelvis leaves the world spot it started on, and its frame's origin leaves that spot with it
