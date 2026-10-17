## MODIFIED Requirements

### Requirement: Body trace
`exarust --trace <file.csv>` SHALL write one row per traced body (the player's, else every simulated character body, never a prop's) after each tick, with the retail probe's columns: pelvis particles; heights, tone, energy, support, balance, counter; part targets; foot contacts; motion and clock; then sleep countdown and state, frame origin, body root, clip blend, hold weight, condition word (hex); swing rate, strength and state word (hex). Lengths in cm, clock in frames; empty if not simulated.

#### Scenario: Standing in the arena
- **WHEN** `exarust --trace t.csv play arena` runs for a minute
- **THEN** `t.csv` has one row per tick for the player's body, its clock advancing 0.6 of a frame per row, its blend and hold 0, its condition `0x0`, and its pelvis 1.7 cm over its target pelvis height

#### Scenario: A released punch
- **WHEN** `exarust --opponent --trace t.csv --press LMB@200 play arenao1` runs
- **THEN** the punch's rows show the rate 1.62, then P falling by 0.3 a row with the state word's stalling and released bits, and the stance from the punch's sixth row
