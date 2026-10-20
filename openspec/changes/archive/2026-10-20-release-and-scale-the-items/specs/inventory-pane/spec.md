## ADDED Requirements

### Requirement: Item release
The release SHALL clear bit 0 of the 0x14 flag word of every counted entry and run the base once.

#### Scenario: The release sweep
- **WHEN** the flag words are 3, 2 and 1
- **THEN** they become 2, 2 and 0
- **AND** one base call runs

### Requirement: Item scale
The scale SHALL divide each entry's x and y by the argument and multiply by the unit, rounding to nearest even, and run the base once.

#### Scenario: The scale
- **WHEN** the positions are 100, 200 and 50, 25 with divisor 4 and unit 10
- **THEN** they become 250, 500 and 125, 62
- **AND** one base call runs
