## ADDED Requirements

### Requirement: Held cap
Mask bits 1 and 2 together SHALL then draw a cap under that strip. At unit 5 it SHALL run from 0, 10 to 10, 30. An edge of 5, 5 SHALL draw none. A clear hold bit SHALL draw none. The flag test after that cap is not in.

#### Scenario: An edge of 4 draws the cap
- **WHEN** unit 5 dispatches a held form that is not wide, with style bits 0x80 and an edge of 4, 4
- **THEN** the cap runs from 0, 10 to 10, 30

