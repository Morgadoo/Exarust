## ADDED Requirements

### Requirement: Shield equip factors
A shield's equip SHALL give its holder the parry factor 0.5 + 1.5·(1 − w)², with w halved under the Mobility technique, the leak 1 − block/2 and the stamina cost equal to the leak, times 0.6666667 under Endure, all in single precision. The equip SHALL NOT turn every blow on the holder into a block.

#### Scenario: Round shield, no techniques
- **WHEN** a shield of weight 0.5 and block 0.58 is equipped by a holder with no shield techniques
- **THEN** the parry factor is 0.875 and the leak and cost are 0.71

#### Scenario: Blow on the arm
- **WHEN** a blow strikes a limb of a shield holder
- **THEN** it takes the flesh path, not the block path

### Requirement: Starting flip
A held weapon whose character starts flipped SHALL be placed half a turn about y and built from the flipped section. Every held item SHALL take the character's flip, which the main hand's item sets.

#### Scenario: Flipped second weapon
- **WHEN** the main hand's item starts flipped and a second weapon is attached
- **THEN** the second weapon is placed and built flipped too

### Requirement: Weapon switch swap
On the tick a weapon switch's condition passes from switching to switched, a holder with two held weapons in different slots SHALL trade their slots and move its primary striker with them, once.

#### Scenario: Next tick
- **WHEN** the tick after the swap is still switched
- **THEN** the slots stay as swapped

### Requirement: Held refresh
The held refresh SHALL count the held weapons only and take the first slot's weapon for the reach (start byte × 0.01 in double × the item height + 60 cm, or the race's reach without the hands bits), the aim height 0.9 × the item height and the 0x521 flag; empty hands SHALL clear the parry flag and give no measures.

#### Scenario: Weapon and shield
- **WHEN** a holder holds one sword and a shield
- **THEN** the refresh counts one held weapon
