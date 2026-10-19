## ADDED Requirements

### Requirement: Held right
Bit 0x200 SHALL then draw one strip. At unit 5, width 100 and height 80 it SHALL run from 80, 30 to 90, 50. A clear bit 0x200 SHALL draw none. A clear hold bit with that bit set SHALL draw the wider strip, from 70, 40 to 80, 60 at height 100. The drawing after that strip is not in.

#### Scenario: Width 100 sits at 80, 30
- **WHEN** unit 5 dispatches a held form of width 100 and height 80 that is not wide, with style bits 0x80 and bit 0x200
- **THEN** the strip runs from 80, 30 to 90, 50
