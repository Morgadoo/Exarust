## ADDED Requirements

### Requirement: Held lip
The same bits with a clear tab bit SHALL draw one narrower strip. At unit 5, width 100 and height 80 it SHALL run from 90, 50 to 100, 70. A set tab bit SHALL draw the wider strip instead. A missing bit SHALL draw none. A clear hold bit with a clear tab bit SHALL draw the wider clear strip, from 80, 60 to 90, 80 at height 100. The next quad is not in.

#### Scenario: Width 100 sits at 90, 50
- **WHEN** unit 5 dispatches a held form of width 100 and height 80 that is not wide, with style bits 0x80, the tab bit clear, an edge of 4, 4, a right room of 100, and a bottom room of 80
- **THEN** the strip runs from 90, 50 to 100, 70
