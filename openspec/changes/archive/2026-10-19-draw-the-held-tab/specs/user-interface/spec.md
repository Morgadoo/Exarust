## ADDED Requirements

### Requirement: Held tab
Bit 0x200 on a held form that is not wide SHALL draw one tab. A grip of 10, 20 at unit 5 SHALL sit from 16, 30 to 26, 40. Mask bits 1 and 4 together SHALL raise it to 35. A clear bit 0x200 and a clear hold bit SHALL draw none. The next anchor after that tab is not in.

#### Scenario: A grip of 10, 20 sits at 16, 30
- **WHEN** unit 5 dispatches a held form that is not wide, with style bits 0x80, bit 0x200, and a grip of 10, 20
- **THEN** the tab runs from 16, 30 to 26, 40
