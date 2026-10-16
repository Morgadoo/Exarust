## ADDED Requirements

### Requirement: Debug weapon blood
`--weapon-blood a` SHALL give the held weapon of the player's body, or of the character view's body, blood a (0 to 1) from when it is dressed, so its stain shows without a fight. A value that is not a number from 0 to 1 SHALL print the expected form and exit with code 2.

#### Scenario: Soaked sword
- **WHEN** `exarust --weapon ub05ssworda --weapon-blood 1 character` runs
- **THEN** the short sword is stained from its tip to below its guard

#### Scenario: Bad value
- **WHEN** `--weapon-blood 1.5` is given
- **THEN** exarust prints the expected form and exits with code 2
