## ADDED Requirements

### Requirement: Held ledge
Mask bits 1 and 2 together SHALL then draw a ledge. At unit 5 it SHALL run from 10, 0 to 30, 10. Bit 1 of the second flag word, or owner bit 1, SHALL draw none unless the owner also has bit 0x800000. An edge of 5, 5 and a clear hold bit SHALL draw none. The sign flip after that ledge is not in.

#### Scenario: An edge of 4 draws the ledge
- **WHEN** unit 5 dispatches a held form that is not wide, with style bits 0x80 and an edge of 4, 4
- **THEN** the ledge runs from 10, 0 to 30, 10
