## ADDED Requirements

### Requirement: Held trim
A clear tab bit SHALL then draw one strip. At unit 5, width 100 and height 80 it SHALL run from 90, 30 to 100, 50. A set tab bit SHALL draw none. A clear hold bit with the tab bit clear SHALL draw the wider strip, from 80, 40 to 90, 60 at height 100. The next mask test is not in.

#### Scenario: Width 100 sits at 90, 30
- **WHEN** unit 5 dispatches a held form of width 100 and height 80 that is not wide, with style bits 0x80 and the tab bit clear
- **THEN** the strip runs from 90, 30 to 100, 50
