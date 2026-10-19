## ADDED Requirements

### Requirement: Held low
Mask bits 2 and 8 together SHALL then draw one strip. At unit 5 and height 80 it SHALL run from 0, 50 to 10, 70. A missing bit SHALL draw none. A set tab bit SHALL still draw it. A clear hold bit SHALL draw the wider strip, from 10, 60 to 20, 80 at height 100. The next quad is not in.

#### Scenario: Height 80 sits at 0, 50
- **WHEN** unit 5 dispatches a held form of height 80 that is not wide, with style bits 0x80, an edge of 4, 4, and a bottom room of 80
- **THEN** the strip runs from 0, 50 to 10, 70
