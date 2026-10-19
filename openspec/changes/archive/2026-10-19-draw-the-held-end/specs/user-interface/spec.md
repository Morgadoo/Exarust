## ADDED Requirements

### Requirement: Held end
Mask bits 1 and 4 together, with bit 0x200, SHALL draw one cap. At unit 5 and width 100 it SHALL run from 70, 10 to 100, 30. A clear bit 0x200 SHALL draw none. An edge that misses either bit, and a clear hold bit, SHALL draw none. The quad after a clear tab bit is not in.

#### Scenario: Width 100 sits at 70, 10
- **WHEN** unit 5 dispatches a held form of width 100 that is not wide, with style bits 0x80, bit 0x200, an edge of 4, 4, and a right room of 100
- **THEN** the cap runs from 70, 10 to 100, 30
