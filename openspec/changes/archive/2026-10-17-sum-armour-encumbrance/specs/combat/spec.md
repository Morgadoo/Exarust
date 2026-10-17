## MODIFIED Requirements

### Requirement: Armour
The system SHALL give each body part two armour layers: the race's natural layer, then the worn items in order, folding the older outer layer inwards by effective coverage. Each item's protection SHALL be read as the item stores it: an item made from a template stores it scaled by its condition. With Deflection learnt, an item covering c of a part SHALL cover c + 0.1 up to half, else c + 0.1 × (1 − c) + 0.05, at most all of it.

#### Scenario: Two shirts
- **WHEN** a character wears two mail shirts covering 80 % of a part
- **THEN** the inner layer has full coverage and the first shirt's slash resistance at its effective coverage, and the second shirt is the outer layer

#### Scenario: Deflection
- **WHEN** a character with Deflection wears items covering 30 %, 75 % and 95 % of a part
- **THEN** they cover 0.4, 0.825 and all of it

#### Scenario: Stored protection
- **WHEN** a placed character wears trousers made at quality 160 and damage 160, which store padding 17 (their template's 20 scaled by that condition)
- **THEN** their layer's padding is 17 × 0.005, not scaled again

### Requirement: Shield record
The system SHALL read a shield item's record from its section `0x8000` (weight, block and bulk, bytes 4 to 6 × 0.01 in double precision) and derive what it gives its holder: parry effectiveness 0.5 + 1.5·(1 − weight)², block leak 1 − block/2, stamina cost equal to the leak, and an encumbrance share of bulk/2, as changed by the Mobility and Endure techniques. It SHALL hold for the 22 shield sections of the installed `objdb.rdb`.

#### Scenario: Round shield
- **WHEN** "RoundShield R01" (bytes 50, 58, 60) is read
- **THEN** its parry effectiveness is 0.875 and its leak and stamina cost 0.71; with Endure the stamina cost is 0.473

#### Scenario: Buckler
- **WHEN** "Buckler A" (bytes 15, 50, 8) is read
- **THEN** its weight is 0.15 (not the 0.14999999 of single-precision scaling), its block 0.5 and its bulk 0.08
