## ADDED Requirements

### Requirement: Held left
A held form that is not wide SHALL then draw a left strip. At unit 5 and height 80 it SHALL run from 0, 30 to 10, 50. A clear bit 0x200 SHALL still draw it. A clear hold bit and a wide bit SHALL draw none. The mask compare after that strip is not in.

#### Scenario: Height 80 sits at 0, 30
- **WHEN** unit 5 dispatches a held form of height 80 that is not wide, with style bits 0x80 and bit 0x200 clear
- **THEN** the strip runs from 0, 30 to 10, 50
