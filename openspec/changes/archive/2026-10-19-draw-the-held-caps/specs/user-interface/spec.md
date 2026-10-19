## ADDED Requirements

### Requirement: Held caps
The same clear plain bit SHALL then draw two caps. At unit 5, width 100 and height 80 the upper cap SHALL run from 96, 0 to 106, 20 and the lower cap SHALL run from 96, 60 to 106, 80. That plain bit, mask bit 4, a clear tab bit, and a clear hold bit SHALL draw none. The drawing on the clear tab path is not in.

#### Scenario: Width 100 sits at 96, 0
- **WHEN** unit 5 dispatches a held form of width 100 and height 80 that is not wide, with style bits 0x80, the tab bit, an edge of 5, 5, and the plain bit 0x200 clear
- **THEN** the upper cap runs from 96, 0 to 106, 20 and the lower cap runs from 96, 60 to 106, 80
