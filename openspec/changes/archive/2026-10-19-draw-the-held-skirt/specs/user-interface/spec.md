## ADDED Requirements

### Requirement: Held skirt
When mask bits 2 and 8 are not both set, a held form SHALL draw one quad. At unit 5 and height 80 it SHALL run from -10, 50 to 30, 90. Both bits set SHALL draw the ledge instead. A clear hold bit SHALL draw none. The next quad is not in.

#### Scenario: A tall room sits at -10, 50
- **WHEN** unit 5 dispatches a held form of height 80 that is not wide, with style bits 0x80, an edge of 4, 4, and a bottom room of 400
- **THEN** the quad runs from -10, 50 to 30, 90
