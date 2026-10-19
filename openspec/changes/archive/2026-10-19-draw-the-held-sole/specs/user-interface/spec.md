## ADDED Requirements

### Requirement: Held sole
The same bits SHALL then draw one ledge. At unit 5 and height 80 it SHALL run from 10, 70 to 30, 80. A missing bit SHALL draw none. A set tab bit SHALL still draw it. A clear hold bit SHALL draw the wider ledge, from 20, 80 to 40, 90 at height 100. The other arm is not in.

#### Scenario: Height 80 sits at 10, 70
- **WHEN** unit 5 dispatches a held form of height 80 that is not wide, with style bits 0x80, an edge of 4, 4, and a bottom room of 80
- **THEN** the ledge runs from 10, 70 to 30, 80
