## ADDED Requirements

### Requirement: Opponent weapon
The system SHALL draw a tier's weapon row with the LCG and then one of its non-zero variants, with the row 0-based.

#### Scenario: Seed 12345
- **WHEN** tier 1 draws a row from seed 12345
- **THEN** the row is 5
- **WHEN** tier 2's row 1 draws a variant from seed 12345
- **THEN** the variant is 419

### Requirement: Opponent level
The system SHALL set a company opponent's level to 60000 + rand(9999) and a generic opponent's to 50000 + rand(9999).

#### Scenario: Bases
- **WHEN** a company opponent is spawned from seed 12345
- **THEN** the level is 60000 + the first rand(9999)
- **WHEN** a generic opponent is spawned
- **THEN** the level is 50000 + the first rand(9999)
