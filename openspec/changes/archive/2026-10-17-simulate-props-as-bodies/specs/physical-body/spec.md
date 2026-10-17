## ADDED Requirements

### Requirement: Drawing sleeping bodies
The system SHALL draw only awake bodies between their last two ticks: a body in deep sleep SHALL keep the transforms it was last drawn with, and a body registered asleep and never simulated SHALL keep its nodes' transforms.

#### Scenario: Registered asleep
- **WHEN** a prop body has slept since it was registered for 60 frames
- **THEN** its node's transform is the one the level gave it

#### Scenario: Falls asleep
- **WHEN** a limp body falls into deep sleep
- **THEN** it is drawn once where its last tick left it and not redrawn until it wakes

## MODIFIED Requirements

### Requirement: Body trace
`exarust --trace <file.csv>` SHALL write one row per traced body (the player's, or else every simulated character body, never a prop's) after each tick, with the retail body probe's columns: the pelvis particles; its heights, tone, energy, support, balance, counter; every part's target; the foot contacts; the motion and clock; then its sleep countdown and state, its frame's origin and body root, its clip's blend and hold weight. Lengths in cm, the clock in frames; unsimulated columns stay empty.

#### Scenario: Standing in the arena
- **WHEN** `exarust --trace t.csv play arena` runs for a minute
- **THEN** `t.csv` has one row per tick for the player's body, its clock advancing 0.6 of a frame per row, its blend and hold 0, and its pelvis 1.7 cm over its target pelvis height

### Requirement: Joint trace
`exarust --joint-trace <file.csv>` SHALL write, after every tick, one row per part of every simulated character body not in deep sleep (props' bodies never): the tick, body and bone; the joint's measures in degrees, cm and mm; the clip's turn split the same way; whether a blow landed on the part in the tick; the body's sleep state and countdown; its clip and clock.

#### Scenario: Duel
- **WHEN** `exarust --opponent --joint-trace j.csv play arena` runs for a minute
- **THEN** `j.csv` has 18 rows per tick per fighter, and the rows of the ticks a hit lands mark the struck part as shoved
