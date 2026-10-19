## ADDED Requirements

### Requirement: Menu clock scale
The main menu SHALL convert a clock word by multiplying it by 1/135, rounding to nearest even, then multiplying by 1/8. A clock of 0 SHALL give 0, 135 SHALL give 1/8, 270 SHALL give 1/4, 67 SHALL give 0 and 68 SHALL give 1/8.

#### Scenario: Whole steps
- **WHEN** the clock is 135
- **THEN** the result is 0.125
- **WHEN** the clock is 270
- **THEN** the result is 0.25

#### Scenario: The rounding boundary
- **WHEN** the clock is 67
- **THEN** the result is 0
- **WHEN** the clock is 68
- **THEN** the result is 0.125
