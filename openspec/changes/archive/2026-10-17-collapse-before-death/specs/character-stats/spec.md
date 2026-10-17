## ADDED Requirements

### Requirement: Collapse before death
A killing blow on a living body SHALL start its collapse: condition collapsing, Fall Backward blending round((1.2 − E) × 300) frames, smooth, E the energy the blow leaves; not carried. Each tick, in place of the posture decision: its heading drifts by up to 0.25 per axis; under size 1.2 and balance 0.8 Rebalance starts (20 frames); tone −0.003, pins' gain −0.0003. At tone 0.25 it SHALL die: dead, limp, keeping its motion.

#### Scenario: Blend
- **WHEN** a killing blow leaves a fighter's energy at 1, 0.62 or 0.05
- **THEN** Fall Backward starts with 60, 174 or 345 frames of blend

#### Scenario: Three seconds
- **WHEN** a fighter at full tone starts its collapse
- **THEN** it is collapsing for 249 ticks and dead on the 250th, at a tone of 0.24999

#### Scenario: Big bodies
- **WHEN** a collapsing body of size 1.2 loses its balance
- **THEN** it does not Rebalance, where one of size 1 does once its balance is under 0.8

#### Scenario: Killed in the duel
- **WHEN** the duellist kills the player in `exarust --opponent --trace t.csv play arena`
- **THEN** the trace reads 0x800 from the blow, Fall Backward then Rebalance, and 0xA00 250 ticks later, and the body lies limp on its back

## MODIFIED Requirements

### Requirement: Condition events and readers
A get-up's start SHALL write getting up and the end of its clip normal; a killing blow SHALL write collapsing (dead without muscles). Hit detection SHALL skip the dead; the daze SHALL leave a helpless or attacking fighter's clip alone; the player SHALL not be steered nor engage an attack while helpless; the duellist SHALL idle for 0xA00 and leave a dying target; the world cursor SHALL read dead, collapsing and bit 0x200 from the word.

#### Scenario: Killing blow
- **WHEN** a cut takes a fighter's yellow health under 0.01
- **THEN** its condition is collapsing, a further cut leaves its collapse as it is, and it is dead 250 ticks later

#### Scenario: Dazed attacker
- **WHEN** a fighter is dazed during its attack
- **THEN** its condition stays attacking and its attack clip plays on

#### Scenario: Cursor
- **WHEN** the player's condition is getting up, then collapsing, then dead, in combat mode
- **THEN** the cursor is the ring while it gets up and collapses and the arrowhead once dead
