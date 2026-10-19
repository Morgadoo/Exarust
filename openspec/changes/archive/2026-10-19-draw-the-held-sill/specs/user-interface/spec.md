## ADDED Requirements

### Requirement: Held sill
Both tab arms then draw one quad. At unit 5, width 100 and height 80 it SHALL run from 70, 70 to 90, 80. A set tab bit still draws it, and the wider strip remains. A missing bit SHALL draw none. A clear hold bit SHALL draw the wider quad, from 60, 80 to 80, 90 at height 100. The other arm is not in.

#### Scenario: Width 100 sits at 70, 70
- **WHEN** unit 5 dispatches a held form of width 100 and height 80 that is not wide, with style bits 0x80, the tab bit clear, an edge of 4, 4, a right room of 100, and a bottom room of 80
- **THEN** the quad runs from 70, 70 to 90, 80
