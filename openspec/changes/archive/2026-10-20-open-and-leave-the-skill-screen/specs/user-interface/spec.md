## ADDED Requirements

### Requirement: Skill screen show
Showing SHALL run the relayout when the open flag is set, set bit 1 of the state and store the clock plus the delay as the deadline.

#### Scenario: The opening
- **WHEN** the open flag is set, the state is 0, the clock 100 and the delay 250
- **THEN** the relayout runs and the state becomes 2
- **AND** the deadline is 350

### Requirement: Skill screen leave
Leaving SHALL call the base when the closing flag is set. Otherwise a non-zero page SHALL do nothing. A set bit 0 with a deadline more than 400 above the clock SHALL do nothing, and a set bit 1 SHALL do nothing; without either, the clear call SHALL run.

#### Scenario: The leave
- **WHEN** the closing flag is set
- **THEN** the base call runs and the clear call does not
- **WHEN** the page is 2
- **THEN** nothing runs
- **WHEN** the state is 1, the deadline 900 and the clock 500
- **THEN** the clear call runs
- **WHEN** the state is 1, the deadline 1000 and the clock 500
- **THEN** nothing runs
- **WHEN** the state is 2
- **THEN** nothing runs
