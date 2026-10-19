## ADDED Requirements

### Requirement: Held drape
When mask bits 4 and 8 are not both set, the tab bit SHALL draw one quad. At unit 5, width 100 and height 80 it SHALL run from 70, 50 to 110, 90. Both bits set SHALL draw the bottom quad instead. A clear tab bit SHALL draw none. A clear hold bit SHALL draw none. The clear-tab arm is not in.

#### Scenario: Width 100 sits at 70, 50
- **WHEN** unit 5 dispatches a held form of width 100 and height 80 that is not wide, with style bits 0x80, the tab bit set, an edge of 4, 4, a right room of 400, and a bottom room of 80
- **THEN** the quad runs from 70, 50 to 110, 90
