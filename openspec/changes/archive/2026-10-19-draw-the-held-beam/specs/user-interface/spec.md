## ADDED Requirements

### Requirement: Held beam
With the tab bit set and held-edge mask bit 4 clear, a clear bit 0x200 of the plain word SHALL draw one bar. At unit 5, width 100 and height 80 it SHALL run from 96, 20 to 106, 60. That plain bit SHALL draw none, and the right strip SHALL remain. Mask bit 4 SHALL draw none. A clear tab bit and a clear hold bit SHALL draw none. The drawing after that bar is not in.

#### Scenario: Width 100 sits at 96, 20
- **WHEN** unit 5 dispatches a held form of width 100 and height 80 that is not wide, with style bits 0x80, the tab bit, an edge of 5, 5, and the plain bit 0x200 clear
- **THEN** the bar runs from 96, 20 to 106, 60
