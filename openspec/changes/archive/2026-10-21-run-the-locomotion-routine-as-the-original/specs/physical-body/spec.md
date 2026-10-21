## MODIFIED Requirements

### Requirement: Muscle gains
Each tick the character tick SHALL set: the stance factor s, the race's standing (human 0.5; 1 dormant or in the shock's stagger), rising 0.01 a tick (double) toward 0.7 + 0.3·run (last tick's run blend) while the decision walks, runs or backpedals it, never past it, keeping what it reached above it; T = tone·s; part factors 0.5, a foot half its lower leg's, the head a quarter of the thorax's × its head factor, shoulders, arms and hands 1.25 × scale × arm stiffness, and the legs' gains.

#### Scenario: Walking firms the body
- **WHEN** a standing human starts walking
- **THEN** its stance factor rises from 0.5 to 0.7 in 20 ticks, reaches 1 running, keeps it while it moves on, and is 0.5 again when it stops

#### Scenario: The rise stops on its target
- **WHEN** a walking body's stance factor is 0.695, then 0.845, at a run blend of 0.5
- **THEN** it is 0.705, then 0.85, a tick later

#### Scenario: Dormant
- **WHEN** a standing body is dormant
- **THEN** its stance factor is 1

#### Scenario: Leg limit
- **WHEN** a living human stands for one tick
- **THEN** its leg limit links pull at 0.23, settling at 0.5, at 0.4 while it walks or runs and toward 0 without support

#### Scenario: Feral heads and short limbs
- **WHEN** a fully feral and a half-feral human are built, and a short human (scale 0.95) falls
- **THEN** their head factors are 0.025 and 0.075, and the short human's arms pull with σ = 0.297 off balance

## ADDED Requirements

### Requirement: Leg gains
Each tick the character tick SHALL set the legs' gains by the movement decision's leg mode: a thigh's factor 0.2 walking, 0.6 running, 0.25 × scale standing or backpedalling (in double precision); each leg limit link's strength s ← 0.9s + 0.1t (double precision), t = 0.4 walking or running, 0.5·S² standing or backpedalling. A collapsing body's thighs and leg limits SHALL keep their gains.

#### Scenario: Running thighs
- **WHEN** a human runs, then backpedals
- **THEN** its thighs pull at 0.6, then at 0.25

#### Scenario: Collapsing
- **WHEN** a collapsing body's leg mode is the run
- **THEN** its thighs and leg limits keep the gains they had
