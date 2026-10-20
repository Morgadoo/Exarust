## ADDED Requirements

### Requirement: Tree charge
The charge SHALL add `tick / 120` in double precision, clamp to −9 through 5 and set the gain to `(5 + charge) × 0.2` at or above 0, else `(4 − √−charge) × 0.25`. A clear started byte SHALL be set and the clock stamped only on the first call.

#### Scenario: The rising branch
- **WHEN** the charge is 0 and the tick is 120
- **THEN** the charge is 1 and the gain is 1.2
- **WHEN** the charge is 0 and the tick is −120
- **THEN** the charge is −1 and the gain is 0.75

#### Scenario: The clamps
- **WHEN** the charge is −9 and the tick is −120
- **THEN** the charge stays −9 and the gain is 0.25
- **WHEN** the charge is 5 and the tick is 120
- **THEN** the charge stays 5 and the gain is 2

#### Scenario: The start stamp
- **WHEN** the started byte is clear
- **THEN** it is set and the clock is stored
- **WHEN** it is already set
- **THEN** the clock is unchanged
