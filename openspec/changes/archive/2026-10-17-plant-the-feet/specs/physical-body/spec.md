## ADDED Requirements

### Requirement: Foot contacts
Each leg of a living body SHALL keep a foot contact at its ankle with the original's state bits (2 touching, 4 anchored, 8 reached, 0x10 lifted, 0x20 no ground, 0x100 stalled) and an anchor. Not walking, a foot that does not touch SHALL be planted where it is once it meets the level; walking, no foot touches. Standing still: lying, none touches; getting up (to 0.9 of its clip) or unsupported, each touches on its stance spot; else one at a time steps to a spot 5 cm off.

#### Scenario: Standing
- **WHEN** a body stands on a floor for 120 ticks
- **THEN** both feet touch (state 0x2), anchored where they landed at the floor plus the sole height, their grips full

#### Scenario: Walking and stopping
- **WHEN** a standing body walks, then stops
- **THEN** walking no foot touches; stopped, each foot is planted where it meets the floor and steps onto its stance spot, both standing again within 45 ticks in the arena

#### Scenario: Getting up
- **WHEN** a body gets up
- **THEN** its start clears every foot's state; in the first fifth of the clip no foot touches, then each touches on its stance spot until nine tenths

#### Scenario: Lying
- **WHEN** a body lies down
- **THEN** no foot touches and its lower legs and feet have no friction

#### Scenario: A step
- **WHEN** a standing foot's stance spot is 4.9 cm, then 5 cm from its anchor
- **THEN** it stays, then steps (anchored, its anchor the spot); the other foot does not step until it has landed, and a stalled step lands where it is

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
- **THEN** its chest sways 4 cm and its feet stay within 1 cm of their anchors

#### Scenario: A step's reach
- **WHEN** a stepping foot comes 4.9 cm from its anchor
- **THEN** it has reached it, counts a landing, and its anchor moves halfway to it

### Requirement: Foot friction
Each tick a living body's lower legs and their feet SHALL take their friction from their foot's state: 0 while it does not touch, 2.0 touching, 0 anchored.

#### Scenario: By state
- **WHEN** a standing body's feet touch, then it walks
- **THEN** its lower legs and feet have friction 2.0, then 0

### Requirement: Sole floor
In each iteration, after its foot's links, a body with support and on its feet SHALL push its toe (the sole's point 20 cm × scale ahead of the ankle along the foot) straight up toward its floor (its ground, never over the lower leg's origin) by the gap × S, at most 0.5 cm while the foot stands (touching, not anchored, not lifted), else 0.1 cm.

#### Scenario: Under the floor
- **WHEN** a standing foot's toe is 3 cm, 0.3 cm, then 0.4 cm at S = 0.5 under its floor
- **THEN** it is pushed 0.5, 0.3, then 0.2 cm (0.1 cm when the foot does not stand), and not at all off its feet or without support

### Requirement: Stance level and leg extension
With support, a body's stance level SHALL follow 1 − v² by a tenth a tick (v the share of its feet's ground difference in their squared distance), and its legs' extension SHALL be the level × (pelvis over the highest anchor ÷ the pose's pelvis over its mean ankle)², at most 1; the grip's gain g is scale × the support pins' factor × the extension.

#### Scenario: A step under one foot
- **WHEN** a body stands with one foot on a 15 cm block and the other on the floor, its feet 11.4 cm apart
- **THEN** its anchors sit 24.5 and 9.5 cm up and its level settles at 0.60, where on level ground it is 1

## MODIFIED Requirements

### Requirement: Living bodies in motion
The system SHALL give a living physical body the animated pose of each tick on its character frame. A standing body SHALL be carried with its root's motion since the last tick, less its controller's follow, across the ground and about the vertical only, with where the last tick left it, its level contacts, its feet's anchors, last tick's targets and its frame, and its pelvis held over the frame's by 0.168 × tone² × energy⁴ a tick, critically damped, only damped while it moves (Exarust's own).

#### Scenario: Turning far from the origin
- **WHEN** a living body 80 m from the world's origin turns with its pose
- **THEN** its particles move no faster than the pose's own motion

#### Scenario: Carried by the controller
- **WHEN** the root of a body standing on a floor moves across it and down
- **THEN** its particles, where the last tick left each part and its level contacts move with the root across the floor but not down, while where it is drawn from stays

#### Scenario: Held on the controller
- **WHEN** a standing body's pelvis is off where its frame puts the pose's pelvis
- **THEN** it is pulled back across the ground, and the hold never moves it up or down; while the body moves it is only braked

#### Scenario: A standing turn
- **WHEN** the player turns a quarter standing in the arena
- **THEN** its planted feet are carried round with it, within 0.1 cm of their anchors

### Requirement: Level friction
The system SHALL apply friction at level contacts from the part's own coefficient μ (limbs 2.5, feet 0.5 as built; a living body's lower legs and feet by their foot's state) and the contact normal n, with the limit 0.5·μ·(0.01 + 0.99·n_y²) in cm² per tick². A slide this tick whose square is under the limit is cancelled; a larger one is reduced by 0.01 times the limit of itself. Friction SHALL run in 8 passes after detection and in each of the next tick's 20 link iterations.

#### Scenario: Floor and wall
- **WHEN** a limb's contact point slides 1.1 cm in a tick on a floor, or 0.1 cm on a vertical wall
- **THEN** the slide is cancelled, while a slide of 1.2 cm on the floor or 0.2 cm on the wall is reduced by only 1.25 % or 0.0125 %

#### Scenario: Limp body at rest
- **WHEN** a limp body has dropped onto a floor
- **THEN** it does not creep, every sphere rests within 3 mm of touching the floor, and it falls asleep

### Requirement: Body trace
`exarust --trace <file.csv>` SHALL write one row per traced body (the player's, or else every simulated character body, never a prop's) after each tick, in the retail body probe's columns: pelvis particles; heights, tone, energy, support, balance, counter; every part's target; the foot contacts (state hex, anchor); motion and clock; then sleep countdown and state, frame origin and body root, clip blend and hold, ankles, condition word (hex). Lengths in cm, clock in frames; empty if unsimulated.

#### Scenario: Standing in the arena
- **WHEN** `exarust --trace t.csv play arena` runs for a minute
- **THEN** `t.csv` has one row per tick for the player's body, its clock advancing 0.6 of a frame per row, its blend and hold 0, its condition `0x0`, and its pelvis 1.7 cm over its target pelvis height

#### Scenario: Planted feet
- **WHEN** the player stands in the arena
- **THEN** both foot contacts read `0x2`, their anchors 9.7 cm up, and its ankles within 0.5 cm of them

### Requirement: Support
Each tick each leg of a living body SHALL query the ground 5 cm ahead of its ankle from 70 cm above it (from it down or getting up) with a 10 cm sphere walking, else 5 cm + 10 cm × an anchored foot's share. Ground over half the leg under the pelvis is out of reach, but getting up. A foot is grounded within 20 cm (30 moving) of its ground plus its ground offset. S SHALL be grounded × balance; a standing body without a grounded foot loses its balance unless its controller stands on the ground.

#### Scenario: Standing in the arena
- **WHEN** `exarust --trace t.csv play arena` runs for a minute
- **THEN** every row has the player's support and balance at 1 and its counter at 80

#### Scenario: No ground
- **WHEN** a standing body has no ground under its feet
- **THEN** its balance and support are 0 and it counts the ticks it is airborne; a moving body keeps its balance

#### Scenario: Perched
- **WHEN** a standing body's controller stands on a stone its feet hang beside
- **THEN** it keeps its balance, with support 0

#### Scenario: Out of reach
- **WHEN** a 65 cm block stands under one foot of a standing human
- **THEN** that foot has no ground, the other is grounded, and its support is 1

#### Scenario: Getting up near the ground
- **WHEN** a body getting up has no foot grounded and its pelvis 20 cm over the ground
- **THEN** its support is half its balance, which rises 0.05 a tick

#### Scenario: Down
- **WHEN** a body lying or getting up has a shelf 40 cm over its ankle, and a 5 cm step's edge 7 cm beside its probe
- **THEN** its leg finds the floor under both, where a standing body finds the shelf and the floor, and a walking body the shelf and the step's edge (2.14 cm)

### Requirement: Getting up from the floor
A body lying still (balance under 0.3, limb motion under 25 cm a tick) SHALL get up once a foot gives support or 100 ticks after support was lost: Get up when its pelvis's front axis height is at most 0 (0.2 while Get up plays, −0.2 while Get Up 2 does), else Get Up 2; past a quarter of the clip an unchanged pick does nothing. It SHALL turn to pelvis → chest (reversed for Get Up 2) when over 18.2° off, clear its feet's states and start with a blend of 20.

#### Scenario: Waiting
- **WHEN** a body lies still without support for 99 ticks, then 100
- **THEN** it waits, then gets up

#### Scenario: Face up
- **WHEN** a supported body lies still on its back
- **THEN** Get Up 2 starts with a blend of 20 from its whole pose, its condition getting up, on its controller

#### Scenario: Rolling over
- **WHEN** a body 30 % through Get up rolls onto its back, or one 40 % through Get Up 2 rolls onto its front
- **THEN** the first changes to Get Up 2 and re-anchors its frame; the second keeps its clip

### Requirement: Steadying on the feet
A standing body at full tone playing a plain clip (no attack, combat idle, walk, run, kick, weapon switch or power) SHALL take the Defence stance (no blend) under balance 0.8 (double) with a fast chest speed² of at most 0.3, else Rebalance (blend 20); weak (tone under 0.85), the Defence stance. A Rebalance SHALL hold the posture switch off for 41 ticks off balance, 27 unsteady.

#### Scenario: Unsteady
- **WHEN** a standing body has balance 0.79 and a fast chest speed² of 0.29, then 0.31, then balance the float 0.8
- **THEN** it takes the Defence stance, then Rebalance, then neither

#### Scenario: Shoved
- **WHEN** `exarust --fixed-frames --shove 60 play arena` dazes the player
- **THEN** it Rebalances at balance 0.64 and sinks onto its planted feet before it is down
