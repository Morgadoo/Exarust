## ADDED Requirements

### Requirement: Held shelf
The same clear tab bit SHALL then draw a shelf. At unit 5 and width 100 it SHALL run from 70, 0 to 90, 10. Bit 1, or owner bit 1, SHALL draw none unless the owner also has bit 0x800000. A set bit 0x200, an edge that misses either mask bit, and a clear hold bit SHALL draw none. The quad after that shelf is not in.

#### Scenario: Width 100 sits at 70, 0
- **WHEN** unit 5 dispatches a held form of width 100 that is not wide, with style bits 0x80, bit 0x200 clear, an edge of 4, 4, and a right room of 100
- **THEN** the shelf runs from 70, 0 to 90, 10
