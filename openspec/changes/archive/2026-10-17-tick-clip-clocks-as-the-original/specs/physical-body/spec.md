## MODIFIED Requirements

### Requirement: Body trace
`exarust --trace <file.csv>` SHALL write one row per traced body (the player's, else every character body, no prop's) a tick: the retail probe's columns (pelvis particles; heights, tone, energy, support, balance, counter; part targets; foot contacts; motion, clock), then Exarust's (sleep countdown and state, frame origin, body root, clip blend, hold, rate, progress, end flag, ankles, condition word; swing rate, strength, state word). Lengths cm, clock frames, words hex; empty if unsimulated.

#### Scenario: Standing in the arena
- **WHEN** `exarust --trace t.csv play arena` runs for a minute
- **THEN** `t.csv` has one row per tick for the player's body, its clock advancing 0.6 of a frame per row, its blend and hold 0, its condition `0x0`, and its pelvis 1.7 cm over its target pelvis height

#### Scenario: A released punch
- **WHEN** `exarust --opponent --trace t.csv --press LMB@200 play arenao1` runs
- **THEN** the punch's rows show the rate 1.62, then P falling by 0.3 a row with the state word's stalling and released bits, and the stance from the punch's sixth row

#### Scenario: Planted feet
- **WHEN** the player stands in the arena
- **THEN** both foot contacts read `0x2`, their anchors 9.7 cm up, and its ankles within 0.5 cm of them

#### Scenario: The idle clock's columns
- **WHEN** a standing body's trace runs through its Idle clip's wrap
- **THEN** every row's `rate` is 0.6, its `progress` the clock over the last key, and `clip_end` is 1 on the wrap's row alone
