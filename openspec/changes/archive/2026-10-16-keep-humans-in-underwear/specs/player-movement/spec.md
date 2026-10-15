## ADDED Requirements

### Requirement: Start outfit
The system SHALL start the player in play mode as the original's new character without a background kit: wearing only the underwear, with any apparel given on the command line put on over it. Apparel that covers the groin SHALL take the underwear's place.

#### Scenario: New game
- **WHEN** `exarust play exanima01` runs without `--outfit`
- **THEN** the player wears pale grey braies and nothing else

#### Scenario: Shirt given
- **WHEN** `exarust play exanima01 --outfit shirt_b` runs
- **THEN** the player wears the shirt with the braies beneath it

#### Scenario: Trousers given
- **WHEN** `exarust play exanima01 --outfit trouser_a,shirt_b` runs
- **THEN** the player wears the trousers and the shirt and no braies
