## ADDED Requirements

### Requirement: Held rear
Mask bits 4 and 8 together, with the tab bit, SHALL draw one strip. At unit 5, width 100 and height 80 it SHALL run from 80, 50 to 100, 70. A missing bit SHALL draw none. A clear tab bit SHALL draw none. A clear hold bit with the tab bit SHALL draw the wider strip, from 70, 60 to 90, 80 at height 100. The clear-tab arm is not in.

#### Scenario: Width 100 sits at 80, 50
- **WHEN** unit 5 dispatches a held form of width 100 and height 80 that is not wide, with style bits 0x80, the tab bit, an edge of 4, 4, a right room of 100, and a bottom room of 80
- **THEN** the strip runs from 80, 50 to 100, 70
