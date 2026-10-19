## ADDED Requirements

### Requirement: Held floor
A form on that band SHALL then draw one bar. At unit 5, width 100 and height 80 it SHALL run from 30, 70 to 70, 80. A set tab bit SHALL still draw it. A clear hold bit SHALL draw the wider bar, from 40, 80 to 60, 90 at height 100. A style that is not that band SHALL draw none. The next mask test is not in.

#### Scenario: Width 100 sits at 30, 70
- **WHEN** unit 5 dispatches a held form of width 100 and height 80 that is not wide, with style bits 0x80
- **THEN** the bar runs from 30, 70 to 70, 80
