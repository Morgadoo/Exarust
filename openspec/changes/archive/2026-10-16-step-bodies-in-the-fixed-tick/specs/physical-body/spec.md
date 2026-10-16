## ADDED Requirements

### Requirement: World tick order
The system SHALL run each 83 Hz tick of the bodies as one world, in the original's order: every living body's character tick, the player's last; the queued blows; the integration of every awake body; 20 iterations over all bodies; the bodies' level detection; 8 friction passes over all bodies; and the post-step of every body.

#### Scenario: Character ticks first
- **WHEN** a tick runs with the player and other living bodies
- **THEN** every living body's targets are set before any body is integrated, the player's after every other body's

#### Scenario: One body alone
- **WHEN** a single body ticks in the world
- **THEN** it ends the tick exactly where ticking it alone leaves it

### Requirement: Clips sampled every tick
The system SHALL advance a living physical body's clip clock by one tick, at 60 frames per second, and sample its clip inside the body's character tick, not once a frame. A clip that ends into its follow-up SHALL give the follow-up's first pose in that tick. Bodies that are not physical SHALL keep playing their clips once a frame.

#### Scenario: Clip clock at any frame rate
- **WHEN** a physical body plays a clip for 60 ticks at 30, 60, 144 or 240 FPS
- **THEN** its clock advances 60/83 of a frame per tick, and its targets after each tick are the body root's world transform times the clip's pose at that tick

#### Scenario: Frames leave the clip alone
- **WHEN** frames pass on a physical body's clip
- **THEN** only its ticks advance the clock

### Requirement: Body trace
`exarust --trace <file.csv>` SHALL write one row per traced body (the player's, or without a player every simulated body) after every tick, with the retail body probe's columns: the pelvis particles; the body's heights, tone, energy, support and balance; every part's target; the foot contacts; the motion and its clock. Lengths SHALL be in cm and the clock in clip frames; columns not simulated yet SHALL stay empty.

#### Scenario: Standing in the arena
- **WHEN** `exarust --trace t.csv play arena` runs for a minute
- **THEN** `t.csv` has one row per tick for the player's body, its clock advancing 60/83 of a frame per row, and its pelvis at the clip root's height above the floor

## MODIFIED Requirements

### Requirement: Living bodies in motion
The system SHALL give a living physical body the animated pose of each tick: its targets are the body root's world transform times the clip pose, and the pose's own motion is this tick's targets against last tick's. The body SHALL be carried with its root's motion since the last tick, together with where the last tick left it, its level contacts and last tick's targets. A living body's links and level contacts SHALL be solved from the hands and feet in to the pelvis.

#### Scenario: Turning far from the origin
- **WHEN** a living body 80 m from the world's origin turns with its pose
- **THEN** its particles move no faster than the pose's own motion

#### Scenario: Carried by the controller
- **WHEN** the root of a body standing on a floor moves
- **THEN** its particles, where the last tick left each part and its level contacts move with the root, while where it is drawn from stays

### Requirement: Fixed simulation rate
The system SHALL step the simulation, character controllers and bodies alike, at the original's 83 Hz in the fixed step, so that the same ticks give the same result at any frame rate. A slow frame SHALL catch up at most 4 steps; the rest of the backlog SHALL be dropped, as the original drops it.

#### Scenario: Slow frame
- **WHEN** a frame takes 200 ms
- **THEN** at most 4 fixed steps run, and game time advances 48 ms

#### Scenario: Any frame rate
- **WHEN** a carried, driven, shoved body and a limp one run for three seconds at 30, 60, 144 and 240 FPS
- **THEN** after every tick every particle is exactly where it is at any other frame rate

### Requirement: Smooth character motion
The system SHALL draw characters and their body parts between their last two fixed steps, by the fraction of the next step already elapsed. Each body part SHALL be drawn under its drawn parent, so that no part is interpolated twice, and where its ticks put it whatever the frame did to its character or body root.

#### Scenario: Walking at 37 FPS
- **WHEN** the player walks while the frame rate is not a divisor of 83
- **THEN** the drawn position advances evenly from frame to frame

#### Scenario: Parts between ticks
- **WHEN** a falling body is drawn at 144 FPS
- **THEN** each part lies the elapsed fraction of the way from where the tick before last left it to where the last tick did

#### Scenario: Under a walking character
- **WHEN** a character walks under its controller and its body is drawn at 30, 60, 144 or 240 FPS
- **THEN** each part is drawn where its last two ticks put it, under the drawn character

#### Scenario: Root moved by the frame
- **WHEN** a frame moves and turns a body's root after the ticks, as getting up or turning toward a foe does
- **THEN** each part is still drawn where its last two ticks put it, with no one-frame jump

### Requirement: Blows and daze
The system SHALL let a blow change the velocity of the struck part and take a share of the body's energy (at most 90 %, never below 0.05), which recovers by 0.005 per tick. Blows SHALL be queued and dealt by the next tick, whichever frame sends them.

#### Scenario: Stagger
- **WHEN** a body's chest is shoved at 3 m/s with daze 0.7
- **THEN** the chest is pushed off its pose, the body sags, and both recover within about a second

#### Scenario: Blow between ticks
- **WHEN** a blow is sent in a frame that runs no tick
- **THEN** it waits in the queue and the next tick deals it
