## ADDED Requirements

### Requirement: Pose drive
After inertia each particle of a pose-driven part SHALL be pushed by G·o·(1 − c), only when c < 1, its vertical cut to a tenth. o runs from the particle to R·r + O: its rest vertex under the part's animated rotation, at the part's origin on its particles. c = o·v/(s|o|²), v its last displacement, s the stance factor. G = gain·t²·T·E⁴·(0.5T + 0.5)·S², t the tone, T = tone·s. Only with support.

#### Scenario: Closing particle
- **WHEN** a particle closes on its target at s|o| a tick or faster, and another moves away from it
- **THEN** the first is not pushed and the second is pushed harder than G·o

#### Scenario: Moving part
- **WHEN** a part turned a little off its pose moves at 1.6 cm a tick
- **THEN** its particles whose offsets point along the motion are not pushed, and those whose offsets point against it are pushed harder than G·o

#### Scenario: No support
- **WHEN** a body lies on its back on the floor, or stands on its controller with no ground under its feet
- **THEN** its pose drive moves no particle

#### Scenario: Chest shove
- **WHEN** a standing body is shoved at the chest at 1.5 m/s
- **THEN** its torso sways and turns back to its pose, its balance stays at 0.6 or more, and it comes to rest

## MODIFIED Requirements

### Requirement: Muscles
The system SHALL drive a living physical body toward its animated pose: its link muscles turn every joint, and a pose drive turns the pelvis, lumbar, thorax and feet (gain 0.03·K, K = 19200/83²) and the head (0.01·K) toward their animated orientation about where each part's origin is, with no position target. Thighs, shins, arms and hands SHALL have no pose drive. Gravity is compensated by tone times energy.

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

### Requirement: Living bodies in motion
The system SHALL give a living physical body the animated pose of each tick: its targets are the body root's world transform times the clip pose. A standing body SHALL be carried with its root's motion since the last tick, together with where the last tick left it, its level contacts and last tick's targets, and its pelvis's origin held toward the pose's pelvis by 0.168 × tone² × energy⁴ a tick, critically damped (Exarust's own). Links and contacts SHALL be solved from the hands and feet in.

#### Scenario: Turning far from the origin
- **WHEN** a living body 80 m from the world's origin turns with its pose
- **THEN** its particles move no faster than the pose's own motion

#### Scenario: Carried by the controller
- **WHEN** the root of a body standing on a floor moves
- **THEN** its particles, where the last tick left each part and its level contacts move with the root, while where it is drawn from stays

#### Scenario: Held on the controller
- **WHEN** the floor pushes a standing body's feet up by 1 cm
- **THEN** its pelvis rises less than 2 cm and comes to rest within 1 cm of its pose

### Requirement: Blows and daze
The system SHALL let a blow change the velocity of the struck part and take a share of the body's energy (at most 90 %, never below 0.05), which recovers by 0.005 per tick up to the body's most energy, 1 for a human. Blows SHALL be queued and dealt by the next tick, whichever frame sends them.

#### Scenario: Stagger
- **WHEN** a body's chest is shoved at 3 m/s with daze 0.7
- **THEN** the chest is pushed off its pose, the body sags, and both recover within about a second

#### Scenario: Blow between ticks
- **WHEN** a blow is sent in a frame that runs no tick
- **THEN** it waits in the queue and the next tick deals it

### Requirement: Falling and getting up
The system SHALL make a standing body whose balance was below 0.6 last tick fall: it loses its controller's support and hold, keeps its tone, plays Fall Forward when its lean points ahead of it and Fall Backward otherwise, and it is not steered. Once it has lain for 2.5 s and its energy is back to 0.9, it SHALL get up with the get-up clip whose first pose lies the way the body does, placed under the body, its controller settling on the ground there.

#### Scenario: Knocked over
- **WHEN** `exarust --shove 60 play arena` deals the player a body blow
- **THEN** the player falls onto its back, lies a moment, gets up with `Get Up 2`, and stands where it fell

### Requirement: Ragdoll stability
The system SHALL keep bodies stable under blows: a particle SHALL carry at most 25 m/s (Exarust's guard; the original has none).

#### Scenario: Fallen far from the origin
- **WHEN** a living body 72 m from the world's origin falls and lies still while its pose keeps being aimed
- **THEN** no particle moves faster than 5 m/s

#### Scenario: Deep contact
- **WHEN** a body's sphere would end a tick deep behind a floor it crossed
- **THEN** it is taken back to where it first touched the floor, and no particle gains more than 25 m/s

### Requirement: Fallen muscles act inside the body
A fallen body SHALL keep its tone and, once its support is 0, have no pose drive: only its link muscles act, pulling its parts against each other, so nothing damps its bulk motion and it falls as fast as a limp body.

#### Scenario: A fallen body kicking its legs
- **WHEN** a fallen body lies on its back on a floor while its pose kicks its legs and swings its arms for six seconds
- **THEN** its centre of mass stays within 30 cm of where it lay

#### Scenario: Falling
- **WHEN** a fallen body and a limp one drop for half a second
- **THEN** the fallen one falls at least 0.8 times as fast
