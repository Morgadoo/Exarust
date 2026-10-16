## ADDED Requirements

### Requirement: Moveset from held items
The system SHALL pick a fighter's moveset from what it holds, in and out of combat mode, as the original: empty hands the race's own moveset (weapon class 0x80); one one-handed weapon "One Handed", two "2W Light"; a two-hander "Greatsword" for flags 0x521, "2H Light" with kind bit 0, else "Two Handed"; a polearm "Polearm"; a shield a shield moveset; an off-hand light "1H & Torch"; a main-hand light "Torch". A clip playing SHALL go on in the new moveset.

#### Scenario: Unarmed player
- **WHEN** `exarust --opponent play arena` runs without `--weapon` and Tab is pressed
- **THEN** the player stands in the "Default" moveset's combat idle, and the duellist with its sword in "One Handed"

#### Scenario: Two-handers
- **WHEN** a fighter holds a two-handed weapon whose flags are 0x521, then one with kind 1, then one with kind 2
- **THEN** it fights with "Greatsword", then "2H Light", then "Two Handed", stance class 2 and weapon class 0x20 each time

#### Scenario: Torch beside a sword
- **WHEN** the player carries the torch with `--weapon ub05ssworda` in a story level
- **THEN** the torch is in the left hand and the player moves with "1H & Torch", stance class 4

### Requirement: Player attacks
The system SHALL start an attack on a left click in combat mode from the player's moveset: Attack Mid, Down or Up at random, never the one picked last; Attack Combo 1, 2 or 3 the same way when the cursor lies more than 20° left of the body's facing (15° for weapon class 0); Overhead Swing on a click within 300 ms of the one before, unless a Stab or an Overhead plays.

#### Scenario: Punch
- **WHEN** the unarmed player clicks in combat mode with the cursor ahead
- **THEN** it throws the "Default" right straight, the right arm out and the body twisted into it

#### Scenario: Left straight
- **WHEN** the unarmed player clicks with the cursor 25° to the left of its facing
- **THEN** it throws one of the "Default" combos, the left straight

#### Scenario: Double click
- **WHEN** the player clicks twice within 300 ms
- **THEN** the second click starts the overhead swing
