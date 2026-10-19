## ADDED Requirements

### Requirement: Ink mark
On a wide form whose style word has bits 0xc0 equal to 0x80, bit 0x40 of the first flag word SHALL count one ink copy. A clear bit SHALL count none. A form that is not wide, and style 0x10 without those bits, SHALL count none. The join after that copy is not in.

#### Scenario: Bit 0x40 counts one copy
- **WHEN** unit 5 dispatches a wide form whose style word is 0x80 and whose first flag word has bit 0x40 set
- **THEN** the ink copy is counted once, and a clear bit counts none
