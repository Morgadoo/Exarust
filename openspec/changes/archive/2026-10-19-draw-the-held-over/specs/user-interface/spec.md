## ADDED Requirements

### Requirement: Held over
When mask bits 1 and 4 are not both set, bit 0x200 SHALL draw one quad. At unit 5 and width 100 it SHALL run from 70, -10 to 110, 30. A clear bit 0x200 SHALL draw none. An edge of 4, 4 with a right room of 100 SHALL draw the wider cap instead. A clear hold bit SHALL draw none. The quad after a clear tab bit on that path is not in.

#### Scenario: An edge of 5 sits at 70, -10
- **WHEN** unit 5 dispatches a held form of width 100 that is not wide, with style bits 0x80, bit 0x200, and an edge of 5, 5
- **THEN** the quad runs from 70, -10 to 110, 30
