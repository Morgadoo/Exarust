## MODIFIED Requirements

### Requirement: Muscles
The system SHALL drive a living physical body toward its animated pose: its link muscles turn every joint, and a pose drive turns the pelvis, lumbar, thorax and feet (gain 0.03·K, K = 19200/83²) and the head (0.01·K times its character's head gain) toward their animated orientation about where each part's origin is, with no position target. Thighs, shins, arms and hands SHALL have no pose drive. The carry holds the body up in place of gravity.

#### Scenario: Driven walk
- **WHEN** `exarust character` plays the walk on a physical body
- **THEN** the body walks with every joint within 15° of the clip's angle, but for the tick its loop wraps, and no limb flails

#### Scenario: Limbs by their muscles
- **WHEN** a living body's arms and legs are aimed turned away from how they are turned
- **THEN** the pose drive leaves their particles where they are

#### Scenario: Gains
- **WHEN** an alert human stands, walks and runs
- **THEN** its pose drive's gain is 0.0314 on the pelvis, spine and feet and 0.0105 on the head standing, 0.0497 walking and 0.0836 running

#### Scenario: Rigid translation
- **WHEN** a body is moved and moves as a whole, each part turned as its pose turns it
- **THEN** the pose drive pushes no particle

#### Scenario: Turned part
- **WHEN** a part at rest is turned 0.3 rad about the vertical off its pose
- **THEN** one tick turns it back by G of the angle, about its origin, which stays where it is

#### Scenario: Feral heads
- **WHEN** a half-feral human and a fully feral one stand alert
- **THEN** their heads' pose gain is 0.0052 and none, their trunks' still 0.0314

### Requirement: Blows and daze
The system SHALL let a blow change the velocity of the struck part and take a share of the body's energy (at most 90 %, never below 0.05), which recovers by 0.005 per tick up to the body's most energy: its character's, 1 for a human that is not feral. Blows SHALL be queued and dealt by the next tick, whichever frame sends them.

#### Scenario: Stagger
- **WHEN** a body's chest is shoved at 3 m/s with daze 0.7
- **THEN** the chest is pushed off its pose, the body sags, and both recover within about a second

#### Scenario: Blow between ticks
- **WHEN** a blow is sent in a frame that runs no tick
- **THEN** it waits in the queue and the next tick deals it

#### Scenario: Zombie energy
- **WHEN** a fully feral body (most energy 0.9) spawns at energy 1, and later is dazed to 0.5
- **THEN** its first tick leaves it at 0.9, and it then recovers 0.005 a tick back to 0.9

### Requirement: Muscle gains
Each tick the character tick SHALL set: the stance factor s, the race's standing (human 0.5), rising 0.01 a tick toward 0.7 + 0.3·run while moving; T = tone·s; part factors 0.5, a foot half its lower leg's, the head a quarter of the thorax's times its character's head factor, a thigh 0.25 × scale standing and 0.2 moving, shoulders, arms and hands 1.25 × scale × arm stiffness; each leg limit link's strength s ← 0.9s + 0.1t, t = 0.5·S² standing, 0.4 moving.

#### Scenario: Walking firms the body
- **WHEN** a standing human starts walking
- **THEN** its stance factor rises from 0.5 to 0.7 in 20 ticks, reaches 1 running, keeps it while it moves on, and is 0.5 again when it stops

#### Scenario: Leg limit
- **WHEN** a living human stands for one tick
- **THEN** its leg limit links pull at 0.23, settling at 0.5, at 0.4 while it moves and toward 0 without support

#### Scenario: Feral heads and short limbs
- **WHEN** a fully feral and a half-feral human are built, and a short human (scale 0.95) falls
- **THEN** their head factors are 0.025 and 0.075, and the short human's arms pull with σ = 0.297 off balance

### Requirement: Arm stiffness
The arms' stiffness SHALL be 0.5 off balance (balance under 0.6, falling, lying or getting up), 0.2 at tone under 0.85, the character's walking arms (0.2 unless feral) in the walk and run cycles, and otherwise x ← 0.8x + 0.08·(0.5 + 0.5·race arm relaxation), never under min(0.4·|turn left in rad|, 0.4). An arm part's anchor relaxation SHALL be capped at 0.6 − (x − 0.1)·0.8.

#### Scenario: Idle human arms
- **WHEN** an alert human stands still
- **THEN** its arms settle at stiffness 0.4 and pull with σ = 0.25, their relaxation capped at 0.36

#### Scenario: Walking and off balance
- **WHEN** the human walks, and later falls
- **THEN** its arms pull with σ = 0.175 walking and 0.3125 off balance

#### Scenario: Zombie arms
- **WHEN** a fully feral human walks, and is then dazed to tone 0.8
- **THEN** its arms' stiffness is 0.05 walking and 0.2 dazed

### Requirement: Pose hold
A living body's sampled turns SHALL be pulled toward the pose it holds (its pelvis's and legs' targets, its other parts' physical turns, taken while it is dazed, its blow sag on or its support under 1) by w = 1 − (1 − S)²·(1 − s)·(E/Emax)⁴ while dazed (energy more than 0.05 under its most, in double precision), else (1 − S)·s, when w is over 1.19e-7. The sag SHALL fade 0.01 a tick and be 0 once the body falls.

#### Scenario: Dazed on its feet
- **WHEN** a body standing with support 1 is dazed to half its energy
- **THEN** its targets are its physical pose until its energy is back over 0.95

#### Scenario: Dazed lying
- **WHEN** a lying body (support 0) has half its energy
- **THEN** its turns are pulled 0.9375 of the way to its physical pose

#### Scenario: Alert
- **WHEN** an alert body has no blow sag
- **THEN** its targets are its clip's pose

#### Scenario: Threshold
- **WHEN** a body's energy is 0.95 as a float, a hair under the double 0.95, or the next float up
- **THEN** it is dazed, or it is not

#### Scenario: Feral threshold
- **WHEN** a fully feral body (most energy 0.9) has energy 0.85 as a float, or the float under it, and lies with energy 0.45
- **THEN** it is not dazed, or it is, and lying its turns are pulled 0.9375 of the way to its physical pose
