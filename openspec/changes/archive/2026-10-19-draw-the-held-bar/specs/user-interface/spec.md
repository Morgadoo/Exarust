## ADDED Requirements

### Requirement: Held bar
The same held form SHALL then draw one bar. At unit 5 and width 100 the bar SHALL run from 30, 0 to 70, 10. A y below the unit with owner bit 1 and without bit 0x800000 SHALL draw none. Owner bit 0x800000 SHALL draw the bar. A clear hold bit and a wide bit SHALL draw none. The drawing after that bar is not in.

#### Scenario: The bar runs from 30 to 70
- **WHEN** unit 5 dispatches a held form of width 100 that is not wide, with style bits 0x80 and a y of 5
- **THEN** the bar runs from 30, 0 to 70, 10
