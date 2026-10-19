## ADDED Requirements

### Requirement: Sign flip
When the edge bits are not both set, a held form SHALL draw one quad. At unit 5 it SHALL run from -10, -10 to 30, 30. An edge of 4, 4 SHALL draw none, and the cap SHALL remain. A clear hold bit SHALL draw none. The next mask test is not in.

#### Scenario: An edge of 5 sits at -10, -10
- **WHEN** unit 5 dispatches a held form that is not wide, with style bits 0x80 and an edge of 5, 5
- **THEN** the quad runs from -10, -10 to 30, 30
