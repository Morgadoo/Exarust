## ADDED Requirements

### Requirement: Joint measurements
The system SHALL measure each joint of a simulated body where the last tick left it: its turn from the model's rest pose relative to its parent, split into a twist about the part's bone axis and a swing of that axis (with the swing's axis); in a living body, the angle to the clip's turn; and each link's gap against its slack, the largest share of slack a slack link uses, and the pivot's gap. A joint in its rest pose SHALL measure 0.

#### Scenario: Rest pose
- **WHEN** a part sits at its rest turn relative to its parent
- **THEN** its total, swing and twist are 0

#### Scenario: Bend and twist
- **WHEN** a part's turn is a 30° twist about its bone followed by a 60° swing about an axis across it
- **THEN** its swing reads 60° about that axis and its twist 30°

#### Scenario: Link past its slack
- **WHEN** a thigh swings 60° at the hip with a link 30 cm down it and 5 cm of slack
- **THEN** that link reads 25 cm past its slack, 600 % of its range, and is the worst

### Requirement: Joint trace
`exarust --joint-trace <file.csv>` SHALL write, after every tick, one row per part of every simulated body not in deep sleep: the tick, body and bone; the joint's measures in degrees, cm and mm; the clip's turn split the same way; whether a blow landed on the part in the tick; the body's sleep state and countdown; its clip and clock.

#### Scenario: Duel
- **WHEN** `exarust --opponent --joint-trace j.csv play arena` runs for a minute
- **THEN** `j.csv` has 18 rows per tick per fighter, and the rows of the ticks a hit lands mark the struck part as shoved

## MODIFIED Requirements

### Requirement: Body trace
`exarust --trace <file.csv>` SHALL write one row per traced body (the player's, or without a player every simulated body) after every tick, with the retail body probe's columns: the pelvis particles; the body's heights, tone, energy, support and balance; every part's target; the foot contacts; the motion and its clock; then the body's sleep countdown and state. Lengths SHALL be in cm and the clock in clip frames; columns not simulated yet SHALL stay empty.

#### Scenario: Standing in the arena
- **WHEN** `exarust --trace t.csv play arena` runs for a minute
- **THEN** `t.csv` has one row per tick for the player's body, its clock advancing 60/83 of a frame per row, and its pelvis at the clip root's height above the floor
