## MODIFIED Requirements

### Requirement: Body trace
`exarust --trace <file.csv>` SHALL write one row per traced body (the player's, or else every simulated body) after every tick, with the retail body probe's columns: the pelvis particles; its heights, tone, energy, support, balance, counter; every part's target; the foot contacts; the motion and clock; then its sleep countdown and state, frame origin and body root, clip blend and hold weight, condition word (hex). Lengths in cm, the clock in frames; columns not simulated stay empty.

#### Scenario: Standing in the arena
- **WHEN** `exarust --trace t.csv play arena` runs for a minute
- **THEN** `t.csv` has one row per tick for the player's body, its clock advancing 0.6 of a frame per row, its blend and hold 0, its condition `0x0`, and its pelvis 1.7 cm over its target pelvis height
