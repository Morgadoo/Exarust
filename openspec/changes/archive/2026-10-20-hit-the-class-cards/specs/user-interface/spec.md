## ADDED Requirements

### Requirement: Class card hit test
With screen width 1000, clock 2048, scale 1 and available mask 1, the band SHALL run from 652 to 1164 and the two cards SHALL occupy 212–468 and 500–756. A point at 300, 700 SHALL name card 1 and 501, 700 card 2. Points on an edge or outside the band SHALL name none.

#### Scenario: The two cards
- **WHEN** the point is 300, 700
- **THEN** the named card is 1
- **WHEN** the point is 501, 700
- **THEN** the named card is 2

#### Scenario: The edges
- **WHEN** the point is 468, 700 or 212, 700
- **THEN** no card is named
- **WHEN** the point is 300, 652 or 300, 1164
- **THEN** no card is named
- **WHEN** the point is 300, 653
- **THEN** card 1 is named
