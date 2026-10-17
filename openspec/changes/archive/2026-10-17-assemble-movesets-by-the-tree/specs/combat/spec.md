## MODIFIED Requirements

### Requirement: Moveset from held items
The system SHALL pick a fighter's moveset config from what it holds, in and out of combat mode, as the original: empty hands none (weapon class 0x80); one one-handed weapon "One Handed", two "2W Light"; a two-hander "Greatsword" for flags 0x521, "2H Light" with kind bit 0, else "Two Handed"; a polearm "Polearm"; a shield a shield moveset (plus Torch beside a light); an off-hand light "1H & Torch"; a main-hand light "One Handed" plus Torch. A clip playing SHALL go on in the new table.

#### Scenario: Unarmed player
- **WHEN** `exarust --opponent play arena` runs without `--weapon`
- **THEN** the player fights with the "Default" moveset's guard and punches, and the duellist with its sword in "One Handed"

#### Scenario: Two-handers
- **WHEN** a fighter holds a two-handed weapon whose flags are 0x521, then one with kind 1, then one with kind 2
- **THEN** it fights with "Greatsword", then "2H Light", then "Two Handed", stance class 2 and weapon class 0x20 each time

#### Scenario: Torch beside a sword
- **WHEN** the player carries the torch with `--weapon ub05ssworda` in a story level
- **THEN** the torch is in the left hand and the player moves with "1H & Torch", stance class 4

#### Scenario: Race kept
- **WHEN** an ogre's hands change from empty to a one-handed weapon
- **THEN** its base moveset stays "Ogre" and its config becomes One Handed (0x1)
