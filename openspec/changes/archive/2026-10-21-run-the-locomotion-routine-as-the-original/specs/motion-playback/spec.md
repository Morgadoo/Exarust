## MODIFIED Requirements

### Requirement: Clip clocks
A clip's clock SHALL gain frames each body tick from its rate (the race's clip-rate base × its posture's factor): 0.6 × rate standing; tone × rate × 0.6 getting up above half tone; the swing rate attacking (the rate itself off balance); the locomotion's step whenever it locomotes, whatever the clip; none while shocked. A clip SHALL end when its clock less its blend reaches its last key; a looping one SHALL then wrap by that key's time, frames before the first key blending from the last.

#### Scenario: Standing human
- **WHEN** a human stands in Idle
- **THEN** its clock gains 0.6 frames a tick and wraps from 80 to 0 every 133 ticks

#### Scenario: Getting up and walking
- **WHEN** a human at full tone gets up, or walks aligned 1.687 cm a tick
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

#### Scenario: A walk clip without locomotion
- **WHEN** the walk still plays on a body that no longer locomotes
- **THEN** its clock gains 0.6 frames a tick

### Requirement: Backwards walk
Backing up SHALL play Walk and advance its clock by the locomotion clock's step on the backpedal row's negated rate. A looping clock below 0 SHALL wrap by the last key's time, and the locomotion SHALL keep the wrapped clock.

#### Scenario: Backing up
- **WHEN** a human backs up aligned at 1.476 cm a tick playing Walk
- **THEN** its clock loses 0.736 frames a tick

#### Scenario: Negative wrap
- **WHEN** a looping clip's clock is 0.3 and the tick subtracts 0.8, and the last key is at 80
- **THEN** the clock is 79.5

### Requirement: Sprint clock
With the sprint latch kept, a forward gait's clock rate SHALL be × (1 − 0.1 × the encumbrance), on the jog row too. While sprinting (the latch, a stance level above 0.9, over 3.5 cm a tick) the forward gait SHALL blend from the walk toward the feral sprint row, linearly by the run blend while running and by its square root otherwise. The feral sprint row SHALL be the sprint row blended to the jog row by the feral level.

#### Scenario: Unencumbered sprint
- **WHEN** a human sprints at feral level 0, run blend 1 and 4.337 cm a tick
- **THEN** its clock gains 0.727 frames a tick

#### Scenario: Feral sprint
- **WHEN** a human sprints at feral level 1, with no encumbrance, run blend 1 and 4.337 cm a tick
- **THEN** its clock gains 0.894 frames a tick

#### Scenario: Sprint in armour
- **WHEN** a human sprints at feral level 0, encumbrance 0.5, run blend 1 and 4.337 cm a tick
- **THEN** its clock gains 0.691 frames a tick

#### Scenario: Level at 0.9
- **WHEN** the latch is kept, the stance level is 0.9, the run blend is 1 and the speed is 4.337 cm a tick
- **THEN** its clock gains 0.894 frames a tick

#### Scenario: Speed at 3.5 cm
- **WHEN** the latch is kept, the stance level is above 0.9, the run blend is 1 and the speed is 3.5 cm a tick
- **THEN** its clock gains 0.76 frames a tick

## ADDED Requirements

### Requirement: Locomotion clock
The locomotion clock's step SHALL be, with support, K × (0.8·speed + 1) × the gait's clock rate, K = (1 − k)·(1 − 1.8·|0.5 − frac(2·(progress + 1.5))|)·4 + k, k = 1 − (1 − alignment)² (1 aligned); stopping at no more than 5 cm a tick, rate × speed × K. Without support it SHALL keep the walk's phase from when the body lost it, 0.85 − 0.5·round(frac(progress + 1)), and step (2 − u)·u·(0.8·speed + 0.9)·rate, u = frac(1 + phase − progress). frac SHALL keep the sign of its argument.

#### Scenario: Misaligned
- **WHEN** a human walks 1.687 cm a tick at an alignment of 0.8, at the cycle's start and a quarter in
- **THEN** its step is 0.764 × 0.976, then × 1.12

#### Scenario: Stopping
- **WHEN** a human stops softly at 3 cm a tick, then at 6
- **THEN** its step is 0.3253 × 3, then 0.3253 × 5.8

#### Scenario: In the air
- **WHEN** a human without support walks 1.687 cm a tick at progress 0.3
- **THEN** its phase is 0.85 and its step 0.5836; back on its feet the phase is unset
