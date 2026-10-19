## ADDED Requirements

### Requirement: Band close
On that band, a close mark of -4 SHALL draw one square. At unit 5 and width 100 it SHALL run from 80, 0 to 100, 20. The earlier square remains. A mark of 0 or -3 SHALL draw none of this square. A style that is not that band SHALL draw none of it. A wide bit SHALL draw none of it. The call after that square is not in.

#### Scenario: Width 100 sits at 80, 0
- **WHEN** unit 5 dispatches a held form of width 100 and height 80 that is not wide, with style bits 0x80, a close mark of -4, an edge of 4, 4, a right room of 400, and a bottom room of 80
- **THEN** the square runs from 80, 0 to 100, 20
