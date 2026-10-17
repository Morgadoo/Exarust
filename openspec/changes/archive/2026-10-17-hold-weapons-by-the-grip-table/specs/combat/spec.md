## MODIFIED Requirements

### Requirement: Wielded weapons
The system SHALL hold a weapon model in the hand its grip names (the right hand, the left for a polearm) so it follows the hand through every clip, placed by the original's grip table.

#### Scenario: Combat stance
- **WHEN** a character with `--weapon swordex01` plays the one-handed combat idle
- **THEN** the sword is held in the right fist with the blade forward and up

#### Scenario: Polearm
- **WHEN** a character with `--weapon "@Halberd A1"` plays the polearm combat idle
- **THEN** the halberd is held in the left hand at the chest, its haft running down past the right hand

## ADDED Requirements

### Requirement: Held item grips
The system SHALL place every held item by the original's grip table: the second held item takes the off-hand grip, else the weapon class picks the fists', one-handed (also two weapons'), two-handed, polearm or claws grip. The item sits at the grip's offset times the character's size (an ogre's own offset, unscaled) plus the race's held offset (x away from the middle), turned by the grip's angles about the hand's X, then Z, then Y.

#### Scenario: One-handed sword
- **WHEN** a human of average height holds a one-handed sword
- **THEN** it sits 12 cm along −X, 1.5 cm down and 6 cm along +Z of the right hand, its blade along (−0.36, 0.32, 0.88) of the hand

#### Scenario: Size and race
- **WHEN** a one-handed weapon is held by a human of size 1.05, by an ogre of any size, and by the demon (size 1.5)
- **THEN** it sits at (−12.6, −1.575, 6.3) cm, at (−27.5, −7, 9) cm, and at (−27, −2.25, 9) cm of the right hand

#### Scenario: Shipped weapons
- **WHEN** every weapon item of the installed object database is held as the first item
- **THEN** 136 take the one-handed grip (the torch among them), 40 the two-handed and 32 the polearm's, and none is left without one

#### Scenario: Two-hander
- **WHEN** a character with `--weapon "sword gsf 01"` plays the greatsword combat idle
- **THEN** the blade stands upright, tip up, before the chest, as in the live capture of the original's two-handed stance

#### Scenario: Torch
- **WHEN** a human of average height carries the torch in the right hand, or in the left beside a weapon
- **THEN** it sits at (−12, −1.5, 6) cm turned (70°, −15°, 20°), or at (11, −0.5, 6) cm turned (110°, 15°, 160°), as before
