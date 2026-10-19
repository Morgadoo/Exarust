## ADDED Requirements

### Requirement: Form dispatch
A dispatch SHALL store twice the unit, three times the unit, and four times the unit. Bit 0x40 counts a shade. Bit 0x20000 with bit 4 clear draws a quad from the origin to the size. Bit 0x800 leaves the shift at 0. Otherwise the shift is the spare times the value, ties rounded even, and the origin subtracts it. Bit 0x100 skips the shift call. Shown children are visited up to the count. The rest of the dispatcher is not in.

#### Scenario: Unit 5 shifts by the spare
- **WHEN** unit 5 dispatches a form of size 100 by 80 with bit 0x40 and bit 0x20000, a spare of 30 at value 1, and children marked 0, 1, 0, 1
- **THEN** the spans are 10, 15 and 20, the quad reaches 100, 80, the shift is 30, and the shown children are 1 and 3
