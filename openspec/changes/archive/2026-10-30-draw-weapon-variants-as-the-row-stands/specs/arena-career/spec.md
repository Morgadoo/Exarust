## MODIFIED Requirements

### Requirement: Variant selection
The system SHALL draw a row's variant with the engine generator by counting the row's non-zero slots and taking the slot at `(count · roll) >> 32` from the row as it stands, empty slots included, a drawn zero meaning no weapon.

#### Scenario: A three-variant axe
- **WHEN** the generator draws index 1 from the Axe row
- **THEN** the chosen variant is 419

#### Scenario: An interior empty slot
- **WHEN** the row `[7, 0, 9]` is drawn with the highest roll
- **THEN** the count is two and the drawn slot is the zero, no weapon

#### Scenario: A word with the top bit set
- **WHEN** the row `[7, 0x80000000, 9]` is drawn with the highest roll
- **THEN** the count is three and the drawn slot is 9

### Requirement: Opponent weapon
The system SHALL draw a tier's weapon row with the LCG and then its variant by the variant selection rule, with the row 0-based.

#### Scenario: Seed 12345
- **WHEN** tier 1 draws a row from seed 12345
- **THEN** the row is 5
- **WHEN** tier 2's row 1 draws a variant from seed 12345
- **THEN** the variant is 419
