## ADDED Requirements

### Requirement: Plain cap
Mask bits 1 and 4 together, with bit 0x200 clear, SHALL draw one narrower cap. At unit 5 and width 100 it SHALL run from 90, 10 to 100, 30. A set bit 0x200 SHALL draw the wider cap instead. An edge that misses either bit, and a clear hold bit, SHALL draw none. The flag test after that cap is not in.

#### Scenario: A clear tab bit sits at 90, 10
- **WHEN** unit 5 dispatches a held form of width 100 that is not wide, with style bits 0x80, bit 0x200 clear, an edge of 4, 4, and a right room of 100
- **THEN** the cap runs from 90, 10 to 100, 30
