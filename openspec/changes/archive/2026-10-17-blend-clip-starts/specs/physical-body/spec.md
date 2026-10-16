## ADDED Requirements

### Requirement: Pose hold
A living body's sampled turns SHALL be pulled toward the pose it holds (its pelvis's and legs' targets, its other parts' physical turns, taken while it is dazed, its blow sag on or its support under 1) by w = 1 − (1 − S)²·(1 − s)·(E/Emax)⁴ while dazed (energy more than 0.05 under its most), else (1 − S)·s, when w is over 1.19e-7. The sag SHALL fade 0.01 a tick and be 0 once the body falls.

#### Scenario: Dazed on its feet
- **WHEN** a body standing with support 1 is dazed to half its energy
- **THEN** its targets are its physical pose until its energy is back over 0.95

#### Scenario: Dazed lying
- **WHEN** a lying body (support 0) has half its energy
- **THEN** its turns are pulled 0.9375 of the way to its physical pose

#### Scenario: Alert
- **WHEN** an alert body has no blow sag
- **THEN** its targets are its clip's pose

## MODIFIED Requirements

### Requirement: Clips sampled every tick
The system SHALL advance a living physical body's clip clock by the original's rate for what the body does, take a started clip's capture and sample its clip inside the body's character tick, not once a frame. A clip that ends into its follow-up SHALL start it, captured, in that tick. Bodies that are not physical SHALL keep playing their clips once a frame, at 60 frames a second.

#### Scenario: Clip clock at any frame rate
- **WHEN** a physical body stands in a clip for 60 ticks at 30, 60, 144 or 240 FPS
- **THEN** its clock advances 0.6 of a frame per tick, and its targets after each tick are the clip's pose at that tick on the body's frame

#### Scenario: Frames leave the clip alone
- **WHEN** frames pass on a physical body's clip
- **THEN** only its ticks advance the clock

### Requirement: Body trace
`exarust --trace <file.csv>` SHALL write one row per traced body (the player's, or else every simulated body) after every tick, with the retail body probe's columns: the pelvis particles; its heights, tone, energy, support, balance, counter; every part's target; the foot contacts; the motion and clock; then its sleep countdown and state, its frame's origin and body root, its clip's blend and hold weight. Lengths in cm, the clock in frames; columns not simulated stay empty.

#### Scenario: Standing in the arena
- **WHEN** `exarust --trace t.csv play arena` runs for a minute
- **THEN** `t.csv` has one row per tick for the player's body, its clock advancing 0.6 of a frame per row, its blend and hold 0, and its pelvis 1.7 cm over its target pelvis height

### Requirement: Falling and getting up
The system SHALL make a standing body whose balance was below 0.6 last tick fall: it loses its controller's support and hold, keeps its tone, plays Fall Forward (lean ahead) or Fall Backward, with no blend and the fast ease, unsteered. Once it has lain 2.5 s and its energy is back to 0.9, it SHALL get up with the get-up clip whose first pose lies the way it does, turned that way, blending 20 frames from its whole physical pose; its frame re-anchors to the pelvis and its controller follows.

#### Scenario: Knocked over
- **WHEN** `exarust --shove 60 play arena` deals the player a body blow
- **THEN** the player falls onto its back, lies a moment, gets up with `Get Up 2` without a hop, and stands where it fell

#### Scenario: From the lying body
- **WHEN** a get-up starts with a part turned 0.4 rad off its clip's pose
- **THEN** after its first tick that part's target turns as the part does
