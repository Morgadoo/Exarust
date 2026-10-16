## ADDED Requirements

### Requirement: Carry
After its inertia each particle of a living part over half its tone t SHALL move by g(1 − a) + carry·a, any other by g: a = t × its support share (pelvis and thighs 1, lower legs 0.5, feet 0.25) or t × E (the upper body). Each character tick SHALL set the carry to gravity, then, with support, take the lift g·f·S off it.

#### Scenario: Shares
- **WHEN** a part of tone 0.9 takes the carry at energy 0.6
- **THEN** a is 0.9 on the pelvis and thighs, 0.45 on the lower legs, 0.225 on the feet and 0.54 above the legs, and nothing at a tone of 0.5

#### Scenario: No support
- **WHEN** a body at full tone and energy has no ground under its feet
- **THEN** its carry is gravity and every particle falls at g

#### Scenario: Fallen
- **WHEN** a fallen body lies on the floor
- **THEN** its feet have no ground within reach and nothing lifts it

#### Scenario: Dazed
- **WHEN** a body hovering on its lift, its feet off the floor, is held at energy 1, 0.75 and 0.5
- **THEN** its pelvis settles 0.3 cm over its target, 1.2 cm and 3.0 cm under it, where its weight balances the lift

### Requirement: Lift
With support the lift SHALL be f = 0.075(h + 15), at most 1.5, standing (a negative f lifts nothing, unless crouched past 0.9: down to −10), or f = (0.4·run + 0.2)(h + 5), at most 2, moving (none when negative); h = target − pelvis height, in cm. Then f −= (1 − |f|/(|f| + 2))·E²·v, v the pelvis's vertical displacement over the last tick, and f ×= 2(t − 0.5) over a tone of a half.

#### Scenario: Balance point
- **WHEN** an alert body stands on its legs, its pelvis 1.7 cm over its target
- **THEN** f is 1 and it stays there; set down 1 cm into the floor it rises less than 2 cm over where it comes to rest

#### Scenario: Gains
- **WHEN** the pelvis is 5 cm under its target standing, at the target walking, 3.3 cm over it running
- **THEN** f is 1.5, 1 and 1; falling 1 cm a tick at f = 1 adds 2/3 × E²

#### Scenario: Tone
- **WHEN** a standing body's tone is 0.75, then 0.5
- **THEN** its lift is half, then nothing

#### Scenario: Get-up
- **WHEN** `exarust --trace t.csv --shove 60 play arena` gets the player up with `Get Up 2` into Idle
- **THEN** its pelvis rises to its standing height, overshoots it by less than 6 cm (10.7 cm under the old vertical hold) and is back within a centimetre in under 100 ticks

### Requirement: Target pelvis height
Each leg's height SHALL be its ground + crouch·60 + (1 − crouch)·(the clip root's height over its animated ankle + its ground offset: the sole height, moving also the ankle's lift over the walk's lowest). The target SHALL be the lowest foot's, blended by half toward the other's while that one's contact height over its ground plus the lowest's is under 40 cm (less up to 60); standing, capped at ground + sole + leg length, plus 5 cm.

#### Scenario: Idle human
- **WHEN** a human's clip root is 91.4 cm over its ankles on flat ground, its sole 9.68 cm and its legs 85.15 cm
- **THEN** its target is capped at 99.83 cm and it stands with its pelvis 101.5 cm over the ground

#### Scenario: Step
- **WHEN** both feet stand on a 30 cm step, or the platform under a standing body is taken away
- **THEN** its target rises by 30 cm, or drops by 30 cm and its pelvis settles 30 cm lower without losing its balance

#### Scenario: One foot up
- **WHEN** one foot stands on a 30 cm step
- **THEN** standing the lower foot caps the target, and moving it is the mean of the two feet's, uncapped

#### Scenario: Blend
- **WHEN** the other foot's spread is 35, 55 or 60 cm, or it has no ground
- **THEN** it is blended in by 0.5, 0.125 or not at all, or not at all

#### Scenario: No ground
- **WHEN** no foot has ground
- **THEN** the target is −1000 m and nothing lifts the body

## MODIFIED Requirements

### Requirement: Muscles
The system SHALL drive a living physical body toward its animated pose: its link muscles turn every joint, and a pose drive turns the pelvis, lumbar, thorax and feet (gain 0.03·K, K = 19200/83²) and the head (0.01·K) toward their animated orientation about where each part's origin is, with no position target. Thighs, shins, arms and hands SHALL have no pose drive. The carry holds the body up in place of gravity.

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
The system SHALL give a living physical body the animated pose of each tick: its targets are the body root's world transform times the clip pose. A standing body SHALL be carried with its root's motion since the last tick across the ground and about the vertical only, together with where the last tick left it, its level contacts and last tick's targets, and its pelvis's origin held over the pose's pelvis by 0.168 × tone² × energy⁴ a tick, critically damped (Exarust's own).

#### Scenario: Turning far from the origin
- **WHEN** a living body 80 m from the world's origin turns with its pose
- **THEN** its particles move no faster than the pose's own motion

#### Scenario: Carried by the controller
- **WHEN** the root of a body standing on a floor moves across it and down
- **THEN** its particles, where the last tick left each part and its level contacts move with the root across the floor but not down, while where it is drawn from stays

#### Scenario: Held on the controller
- **WHEN** a standing body's pelvis is off where the controller carries the pose's pelvis
- **THEN** it is pulled back across the ground, and the hold never moves it up or down

### Requirement: Falling and getting up
The system SHALL make a standing body whose balance was below 0.6 last tick fall: it loses its controller's support and hold, keeps its tone, plays Fall Forward when its lean points ahead of it and Fall Backward otherwise, and it is not steered. Once it has lain for 2.5 s and its energy is back to 0.9, it SHALL get up with the get-up clip whose first pose lies the way the body does, placed under the body, its controller settling on the ground there.

#### Scenario: Knocked over
- **WHEN** `exarust --shove 60 play arena` deals the player a body blow
- **THEN** the player falls onto its back, lies a moment, gets up with `Get Up 2` without a hop, and stands where it fell

### Requirement: Support
Each tick each leg of a living body SHALL query the ground 5 cm ahead of its ankle, a 10 cm sphere from 70 cm above it (a 5 cm one from it while down or getting up). Ground over half the leg under the pelvis is out of reach, except getting up. A foot is grounded within 20 cm (30 moving) of its ground plus its ground offset. S SHALL be grounded × balance; a standing body without a grounded foot loses its balance unless its controller stands on the ground.

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
- **THEN** its leg finds the floor under both, where a standing body finds the shelf and the step's edge

### Requirement: Body trace
`exarust --trace <file.csv>` SHALL write one row per traced body (the player's, or else every simulated body) after every tick, with the retail body probe's columns: the pelvis particles; its heights (the lift's target included), tone, energy, support, balance, counter; every part's target; the foot contacts; the motion and clock; then its sleep countdown and state. Lengths in cm, the clock in clip frames; columns not simulated yet stay empty.

#### Scenario: Standing in the arena
- **WHEN** `exarust --trace t.csv play arena` runs for a minute
- **THEN** `t.csv` has one row per tick for the player's body, its clock advancing 60/83 of a frame per row, and its pelvis 1.7 cm over its target pelvis height
