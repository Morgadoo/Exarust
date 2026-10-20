## MODIFIED Requirements

### Requirement: Body trace
`exarust --trace <file.csv>` SHALL write a row per traced body (the player's, else every character body, never a prop's) each tick: the retail probe's columns (pelvis particles; heights, tone, energy, support, balance, counter; targets; feet; motion, clock), then sleep, frame, root, blend, hold, condition (hex), swing rate, strength and word (hex), and the locomotion: flags (hex), moving, free walk, yaw, heading, turn rate, alignment, speed, reach, stop reason, goal x and z. Lengths in cm.

#### Scenario: Standing in the arena
- **WHEN** `exarust --trace t.csv play arena` runs for a minute
- **THEN** `t.csv` has one row per tick for the player's body, its clock advancing 0.6 of a frame per row, its blend and hold 0, its condition `0x0`, and its pelvis 1.7 cm over its target pelvis height

#### Scenario: A released punch
- **WHEN** `exarust --opponent --trace t.csv --press LMB@200 play arenao1` runs
- **THEN** the punch's rows show the rate 1.62, then P falling by 0.3 a row with the state word's stalling and released bits, and the stance from the punch's sixth row

#### Scenario: Walking and stopping
- **WHEN** `exarust --trace t.csv --press W@100-250 play arena` runs
- **THEN** the rows show `moving` with flags `0x40` at 1.687 cm a tick, then the soft stop's `0x70` and the stop reason `braked`, then `standing`

#### Scenario: The look-ahead's goal
- **WHEN** `exarust --trace t.csv --press W@100-850 play arena` walks the player into the −Z wall
- **THEN** `goal_z` follows the player 75 cm and more ahead, then holds at −495 once `free_walk` is 0
