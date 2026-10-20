## ADDED Requirements

### Requirement: Clip rate by posture
The posture rules SHALL set a clip's rate each body tick from the race's clip-rate base: × 1.25 off balance (falling, lying, a get-up's first tick), × (1.5 − 0.5 × encumbrance) getting up, × 2 kicking at the tone a standing body keeps (0.85), else × 1; then × 1 while the clip playing is the body's stance, whatever its balance. The walking clock SHALL ignore the rate.

#### Scenario: Dazed in the Defence stance
- **WHEN** a human off balance plays its Defence stance, or any clip that is not its stance
- **THEN** its clock gains 0.6 frames a tick, or 0.75

#### Scenario: Kicking
- **WHEN** a human kicks at full tone, or weak (tone 0.8)
- **THEN** its clock gains 1.2 frames a tick, or 0.6

#### Scenario: An attack off balance
- **WHEN** a human off balance plays an attack's clip
- **THEN** its clock gains 1.25 frames a tick, the rate as it is

### Requirement: Sampler pipeline
A living body's sampler SHALL take each sample in the original's steps, in order: the keys blended, the gait overlay, the hold (normalised, kept as the targets), the femur posture, the clip chain (each part placed by its rest translation and turn under its parent), the IK chain, the torso twist, the head look and the foot pitch; then the drive's anchors and the feet. A step whose rules are not ported SHALL be the identity.

#### Scenario: Identity steps
- **WHEN** a standing human samples Idle
- **THEN** its parts' placements are the keys blended on the chain, and its trace's clip columns are as before the pipeline: the clock 0.6 a tick, no blend, no hold

### Requirement: Clocks without a body tick
A body without a body tick (the character view's figure, a non-physical character) SHALL tick its clip clock by the same body ticks (83 a second), counted from the frames' time with what is under a tick owed to the next frame, each tick gaining 0.6 × the race's base as a standing body's. No clip clock SHALL run per frame.

#### Scenario: Two 50 ms frames
- **WHEN** a human figure without a body tick is drawn for two 50 ms frames
- **THEN** its clock ticks 8 times, gaining 4.8 frames, and 0.3 of a tick is owed; a base of 1.5 gains 7.2

### Requirement: Clock quantities exposed
Each tick SHALL keep the frames its clock gained and whether its end test fired, and each sample SHALL write the progress, (clock − blend) / the last key's time (0 at a start), first; the swing clock, the posture rules and the body trace read them.

#### Scenario: The walk's wrap
- **WHEN** the walk (last key 80) at 79 gains 0.6 and is sampled, then gains 0.6 again
- **THEN** the sample's progress is 0.995, and the second tick's end test fires with the clock at 0.2

## MODIFIED Requirements

### Requirement: Clip clocks
A clip's clock SHALL gain frames each body tick from its rate (the race's clip-rate base × its posture's factor): 0.6 × rate standing; tone × rate × 0.6 getting up above half tone; the swing rate attacking (the rate itself off balance); the gait's clip rate ÷ 83 × (0.8 × speed + 1) walking; none while shocked. A clip SHALL end when its clock less its blend reaches its last key; a looping one SHALL then wrap by that key's time, frames before the first key blending from the last.

#### Scenario: Standing human
- **WHEN** a human stands in Idle
- **THEN** its clock gains 0.6 frames a tick and wraps from 80 to 0 every 133 ticks

#### Scenario: Getting up and walking
- **WHEN** a human at full tone gets up, or walks 1.687 cm a tick
- **THEN** its clock gains 0.9, or 0.764 frames a tick

#### Scenario: Walk loop
- **WHEN** the walk wraps from its last key (80) to frame 0
- **THEN** its pose blends from the last key back to the first over 10 frames, its targets turning no joint more than 4° a tick

#### Scenario: Getting up in plate
- **WHEN** a human at full tone in armour of encumbrance 0.625 gets up
- **THEN** its clock gains 0.7125 frames a tick

#### Scenario: A get-up's first tick
- **WHEN** a human at full tone starts getting up, from the floor or staggering
- **THEN** its clock gains 0.75 frames on that tick and 0.9 a tick after

#### Scenario: Shocked
- **WHEN** a body's condition is shocked
- **THEN** its clock gains nothing, unless its clip is an attack's

### Requirement: Pose sampling
The system SHALL sample a clip at any time by interpolating rotations spherically and the root position linearly between the keys around it; before its first key and past its last a looping clip SHALL blend from its last key back to its first (by time / first key, or (time − last key) / first key, no further than the first key) and one played once SHALL hold its first and last keys.

#### Scenario: Between keys
- **WHEN** a clip is sampled halfway between two keys
- **THEN** each bone's rotation is halfway along the arc between the keys' rotations

#### Scenario: Outside the keys
- **WHEN** a looping clip with keys at 10 and 20 is sampled at 5, 25 and 99 frames, and one played once at 3 and 99
- **THEN** the looping one is halfway from its last key to its first twice, then at its first key; the other is at its first key, then its last
