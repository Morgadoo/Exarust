## MODIFIED Requirements

### Requirement: Clip rate by posture
The posture rules SHALL set a clip's rate each body tick from the race's clip-rate base: × 1.25 off balance (falling, lying, a get-up's first tick), × (1.5 − 0.5 × encumbrance) getting up, the swing's rate attacking, × 2 kicking or parrying at tone 0.85 or more, else × 1; under that tone, and in the walk or the run without a parry, the rate SHALL stay as last set. The walking clock SHALL ignore the rate.

#### Scenario: Dazed in the Defence stance
- **WHEN** a human off balance plays its Defence stance, or any clip that is not its stance
- **THEN** its clock gains 0.6 frames a tick, or 0.75

#### Scenario: Kicking
- **WHEN** a human kicks at full tone, or weak (tone 0.8)
- **THEN** its clock gains 1.2 frames a tick, or 0.6

#### Scenario: A parry doubles the rate
- **WHEN** a human parries at full tone, including while its stance plays
- **THEN** its clock gains 1.2 frames a tick

#### Scenario: Off balance a parry keeps the fall rate
- **WHEN** a parrying human off balance plays its stance
- **THEN** its clock gains 0.75 frames a tick

#### Scenario: An attack off balance
- **WHEN** a human off balance plays an attack's clip
- **THEN** its clock gains 1.25 frames a tick, the rate as it is

### Requirement: Foot pitch
A foot SHALL pitch about its side axis so a point 20 cm times the body scale ahead of the ankle meets the ground. Off its feet the foot SHALL keep the clip; lying down and not getting up, the stored pitch SHALL ease toward 0 by 0.02 rad a tick plus its rate. A correction of 0.1 cm or under SHALL leave the pitch, and the angle SHALL ease by at most 0.01 rad a tick plus its stored rate. A placement SHALL store that angle at once and clear the rate.

#### Scenario: Shin tipped up
- **WHEN** a foot's shin points straight up, its contact is on its ground, and its pitch starts at 0
- **THEN** the pitch is 0.01 rad on the first tick, toward the angle that levels that toe

#### Scenario: A placement
- **WHEN** a placed shin points straight up and its foot is placed with it
- **THEN** the pitch is the whole angle that levels the toe and the rate is 0

## ADDED Requirements

### Requirement: Clip clock gains
A clip's clock SHALL gain frames each body tick from its rate (the race's clip-rate base × its posture's factor): 0.6 × rate standing; tone × rate × 0.6 getting up above half tone; the swing rate attacking (the rate itself off balance); the locomotion's step whenever it locomotes, whatever the clip; none while shocked.

#### Scenario: Standing human
- **WHEN** a human stands in Idle
- **THEN** its clock gains 0.6 frames a tick and wraps from 80 to 0 every 133 ticks

#### Scenario: Getting up and walking
- **WHEN** a human at full tone gets up, or walks aligned 1.687 cm a tick
- **THEN** its clock gains 0.9, or 0.764 frames a tick

#### Scenario: Getting up in plate
- **WHEN** a human at full tone in armour of encumbrance 0.625 gets up
- **THEN** its clock gains 0.7125 frames a tick

#### Scenario: A get-up's first tick
- **WHEN** a human at full tone starts getting up, from the floor or staggering
- **THEN** its clock gains 0.75 frames on that tick and 0.9 a tick after

#### Scenario: Shocked
- **WHEN** a body's condition is shocked
- **THEN** its clock gains nothing, unless its clip is an attack's

#### Scenario: A walk clip without locomotion
- **WHEN** the walk still plays on a body that no longer locomotes
- **THEN** its clock gains 0.6 frames a tick

### Requirement: Feral thorax hunch
A pinned part SHALL pitch its local forward axis down by the thorax hunch before the clip chain when the character's feral level, in double, is over 0.1, then rebuild the basis from that forward, the up axis and their cross. The hunch rises 0.01 a tick in double precision to 0.3 on its feet and falls 0.01 to 0 off them. A balanced attack tick (an attack's clip playing, not off balance, shocked or getting up) SHALL take a further 0.02 off.

#### Scenario: Standing thorax
- **WHEN** a pinned part's turn is identity and its hunch is 0.3, with feral level 1
- **THEN** its forward axis drops below −0.25 and the basis stays orthonormal

#### Scenario: Not feral
- **WHEN** the same part's feral level is 0, or 0.05
- **THEN** its turn stays the clip's

#### Scenario: The attack's ease
- **WHEN** a body with hunch 0.3 attacks on balance, and then attacks off balance, shocked or getting up, or stands normally
- **THEN** the hunch is 0.28 after the balanced attack tick and stays 0.3 otherwise

### Requirement: Clock precision
The standing and getting-up gains SHALL each be a double product added to the clock widened to double, the sum narrowed once; an attack's rate and a locomotion step SHALL add in single precision.

#### Scenario: Added in double
- **WHEN** a clock at 0.0274 gains 0.6 × 0.3, or a get-up clock at 0.0548 gains 0.63
- **THEN** it reads 0.20739999 (a single-precision add gives 0.20740001), or 0.68480003

### Requirement: Clip end and loop
A clip SHALL end when its clock less its blend reaches its last key; a looping one SHALL then wrap by that key's time, frames before the first key blending from the last.

#### Scenario: Walk loop
- **WHEN** the walk wraps from its last key (80) to frame 0
- **THEN** its pose blends from the last key back to the first over 10 frames, its targets turning no joint more than 4° a tick

### Requirement: Stance rate reset
While the clip playing is the body's stance and it does not parry (the previous tick's parry flag), the rate SHALL be the race's base, whatever its balance and even while shocked. While it parries, unless off balance, getting up, attacking or weak, the rate SHALL be × 2. A shocked tick otherwise SHALL keep the last rate unless an attack's clip plays.

#### Scenario: Shocked rates
- **WHEN** a shocked body whose rate was 2 plays its stance, or another clip
- **THEN** its rate is the base 1, or stays 2, and its clock gains nothing

## REMOVED Requirements

### Requirement: Clip clocks
**Reason**: Over 500 characters, and its double-precision sentence and scenario stated the PR's narrowed product where the original adds the product to the clock in double and narrows the sum. Split by behaviour.
**Migration**: Clip clock gains (the gains), Clock precision (the double add), Clip end and loop (the end and the wrap).

### Requirement: Thorax hunch
**Reason**: The hunch was gated on the race's weapon recovery, which the original does not read: it tests the character record's feral level, so a non-feral human is not hunched; its 0.02 ease runs only in the balanced attack branch.
**Migration**: Feral thorax hunch.
