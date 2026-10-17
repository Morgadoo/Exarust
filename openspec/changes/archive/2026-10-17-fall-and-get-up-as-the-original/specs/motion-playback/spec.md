## MODIFIED Requirements

### Requirement: Clip clocks
A clip's clock SHALL gain frames each body tick, from the race's clip-rate base: 0.6 × base standing, × 1.25 off balance; tone × (1.5 − 0.5 × encumbrance) × base × 0.6 getting up above half tone (tone × 0.75 × base on the tick it starts); the swing rate attacking; the gait's clip rate ÷ 83 × (0.8 × speed + 1) walking. A clip SHALL end when its clock less its blend reaches its last key; a looping one SHALL then wrap by that key's time, its frames before the first key blending from the last.

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

## ADDED Requirements

### Requirement: Starting a few keys in
A clip started a few keys in SHALL have its capture meant for that key and its clock at the key before it. A sample aiming at any key other than the one its capture is meant for SHALL drop the blend (the clock loses it) and sample the keys; a clip played once then holds its first key until its clock reaches it.

#### Scenario: Get up three keys in
- **WHEN** Get up (keys 20, 40, 55, 70, 80) starts with a blend of 50 three keys in and ticks 0.75 frames, then 0.9 a tick
- **THEN** its clock starts at 55, reads 5.75 after its first sample, which is the first key exactly, and the first key holds for 16 more ticks

#### Scenario: Key 0
- **WHEN** a clip is started at key 0, or at a key past its last
- **THEN** its start is unchanged
