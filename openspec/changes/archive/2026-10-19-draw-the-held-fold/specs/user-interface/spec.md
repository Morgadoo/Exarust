## ADDED Requirements

### Requirement: Held fold
When mask bits 4 and 8 are not both set, a clear tab bit SHALL draw one quad. At unit 5, width 100 and height 80 it SHALL run from 70, 50 to 110, 90. A set tab bit SHALL draw the other sample instead. Both bits set SHALL draw none. A clear hold bit SHALL draw none. The join after that quad is not in.

#### Scenario: A clear tab sits at 70, 50
- **WHEN** unit 5 dispatches a held form of width 100 and height 80 that is not wide, with style bits 0x80, the tab bit clear, an edge of 4, 4, a right room of 400, and a bottom room of 80
- **THEN** the quad runs from 70, 50 to 110, 90
