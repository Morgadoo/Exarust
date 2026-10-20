## ADDED Requirements

### Requirement: Arena loadout tiers
The system SHALL provide the six loadout tiers with 8, 11, 10, 14, 4 and 4 rows, including the multi-variant Axe, War Hammer and Spiked Mace rows, and the point limits 5, 10, 15, 20, 30 and 40.

#### Scenario: Tiers and variants
- **WHEN** tier 2 is read
- **THEN** it has 10 rows and the Axe row has variants 19, 419 and 213
- **WHEN** tier 5 is read
- **THEN** it repeats tier 4's four polearms
- **WHEN** the point limit of tier 5 is read
- **THEN** it is 40

### Requirement: Variant selection
The system SHALL draw one of a row's non-zero variants with the engine generator.

#### Scenario: A three-variant axe
- **WHEN** the generator draws index 1 from the Axe row
- **THEN** the chosen variant is 419
