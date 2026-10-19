## ADDED Requirements

### Requirement: Skin measures
The method SHALL return the first, second and third measured words followed by four units plus four. With measures 2, 2 and 24 at unit 4 it SHALL return 2, 2, 24, 20, and at unit 5 it SHALL return 2, 2, 24, 24.

#### Scenario: The four measures
- **WHEN** the measures are 2, 2, 24 and the unit is 4
- **THEN** the result is 2, 2, 24, 20
- **WHEN** the unit is 5
- **THEN** the fourth word is 24
