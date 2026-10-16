## ADDED Requirements

### Requirement: Clip clocks
A clip's clock SHALL count frames and gain each body tick, from the race's clip-rate base: 0.6 × base standing, × 1.25 off balance; tone × 1.5 × base × 0.6 getting up, above half tone; the weapon's swing rate attacking; the gait's clip rate ÷ 83 × (0.8 × speed + 1) walking in its walk clip. A clip SHALL end when its clock less its blend reaches its last key; a looping one SHALL then wrap by that key's time, its frames before the first key blending from the last key.

#### Scenario: Standing human
- **WHEN** a human stands in Idle
- **THEN** its clock gains 0.6 frames a tick and wraps from 80 to 0 every 133 ticks

#### Scenario: Getting up and walking
- **WHEN** a human at full tone gets up, or walks 1.687 cm a tick
- **THEN** its clock gains 0.9, or 0.764 frames a tick

#### Scenario: Walk loop
- **WHEN** the walk wraps from its last key (80) to frame 0
- **THEN** its pose blends from the last key back to the first over 10 frames, its targets turning no joint more than 4° a tick

### Requirement: Clip starts from the body
Starting a clip SHALL capture the root, the pelvis's and legs' last targets and the upper body's physical turns (a get-up from the floor: every part's, the pelvis's on the frame), which become the targets. The clip SHALL blend from the capture to its first key by clock / (first key + blend), linear, smooth f²(3 − 2f) or fast 1 + 0.11f − 1/(1 + 8f); once it aims past its first key the blend SHALL end and the clock lose it.

#### Scenario: First sample
- **WHEN** a clip starts
- **THEN** its first sample is the capture, and a get-up's next tick is 0.9/(first key + 20) of the way to the first key

#### Scenario: On time
- **WHEN** Get up (first key 20) and Get Up 2 (15) start with a blend of 20 at 0.9 a tick, or Idle with none at 0.6
- **THEN** they reach their first key after 44, 38 and 33 ticks

### Requirement: Follow-up blends
A clip played once SHALL end into its follow-up with a blend of twice its weapon's recovery after an attack (actions 5–12), 60 frames from action 0x21, 20 from 0x1F and none below, eased smoothly; with no follow-up it SHALL hold its last key.

#### Scenario: End of a get-up
- **WHEN** `Get Up 2` ends into Idle
- **THEN** Idle starts from the get-up's last root, (6.1, 81.7, −1.8) cm, and blends to its own over 20 frames, the pelvis target never moving 1 cm in a tick

#### Scenario: End of an attack
- **WHEN** a duellist's attack ends into Cmbt - Idle
- **THEN** no joint moves more than 2° a tick in the 15 ticks after

## MODIFIED Requirements

### Requirement: Clips in place
The system SHALL leave out the root's travel along z of the walk, the run and the backpedal (and of Exarust's interim `Combat Walk`), whose travel the body's own motion makes. Every other clip SHALL keep its root's travel.

#### Scenario: Walk cycle
- **WHEN** the walk loops
- **THEN** the pelvis stays over the character instead of advancing and snapping back

#### Scenario: Get-up travel
- **WHEN** `Get Up 2` is sampled at its last key
- **THEN** its root is the key's own, (6.1, 81.7, −1.8) cm, not its first key's place across the ground
