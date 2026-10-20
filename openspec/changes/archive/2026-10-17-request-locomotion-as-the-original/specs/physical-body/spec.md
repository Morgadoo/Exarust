## MODIFIED Requirements

### Requirement: Body trace
`exarust --trace <file.csv>` SHALL write a row per traced body (the player's, else every character body, never a prop's) each tick: the retail probe's columns (pelvis particles; heights, tone, energy, support, balance, counter; targets; feet; motion, clock), then sleep, frame, root, blend, hold, condition (hex), swing rate, strength and word (hex), and the locomotion: flags (hex), moving, free walk, yaw, heading, turn rate, alignment, speed, reach, stop reason. Lengths in cm.

#### Scenario: Standing in the arena
- **WHEN** `exarust --trace t.csv play arena` runs for a minute
- **THEN** `t.csv` has one row per tick for the player's body, its clock advancing 0.6 of a frame per row, its blend and hold 0, its condition `0x0`, and its pelvis 1.7 cm over its target pelvis height

#### Scenario: A released punch
- **WHEN** `exarust --opponent --trace t.csv --press LMB@200 play arenao1` runs
- **THEN** the punch's rows show the rate 1.62, then P falling by 0.3 a row with the state word's stalling and released bits, and the stance from the punch's sixth row

#### Scenario: Walking and stopping
- **WHEN** `exarust --trace t.csv --press W@100-250 play arena` runs
- **THEN** the rows show `moving` with flags `0x40` at 1.687 cm a tick, then the soft stop's `0x70` and the stop reason `braked`, then `standing`

### Requirement: Character tick stages
Each living body's character tick SHALL run nine stages in the original's order (upkeep, readiness, anchor, behaviour, posture, attack, movement, sampler, carry) and finish them before the next body's tick starts. Its work SHALL sit in them: what the body does (posture); the movement decision and turn, the gait floor, the controller's step, balance and ground (movement); the clip's capture, clock and sample (sampler); the targets, lift and follow (carry).

#### Scenario: One character after another
- **WHEN** two characters tick
- **THEN** the first runs all nine stages, in order, before the second runs its first

#### Scenario: Who ticks
- **WHEN** a tick runs with the player, a walking character with a controller, a standing body, a limp body and a body whose clips play by frames
- **THEN** the walking character and the standing body tick with their controllers (none for the standing body), the player last, and the limp body and the frame-played body do not tick
