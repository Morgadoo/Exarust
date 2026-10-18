# Spec Delta

## MODIFIED Requirements

### Requirement: Clip clocks
A clip's clock SHALL gain frames each body tick from the race's clip-rate base: 0.6 × base standing, × 1.25 off balance; tone × (1.5 − 0.5 × encumbrance) × base × 0.6 getting up above half tone (tone × 0.75 × base on its first tick); the swing rate attacking; the gait's clip rate ÷ 83 × (0.8 × speed + 1) walking. Backing up SHALL play Walk and advance that clock by the negated backpedal rate. A clip SHALL end when its clock less its blend reaches its last key; a looping one SHALL then wrap by that key's time, and a looping clock below 0 SHALL wrap the same way. Frames before the first key blend from the last.

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

#### Scenario: Backing up
- **WHEN** a human backs up at 1.476 cm a tick playing Walk
- **THEN** its clock loses 0.736 frames a tick

#### Scenario: Negative wrap
- **WHEN** a looping clip's clock is 0.3 and the tick subtracts 0.8, and the last key is at 80
- **THEN** the clock is 79.5
