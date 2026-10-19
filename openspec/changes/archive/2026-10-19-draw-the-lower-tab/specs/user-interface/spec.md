## ADDED Requirements

### Requirement: Lower tab
The same bit SHALL then draw a lower tab from the seat. A seat of 80 at unit 5 SHALL sit from 16, 60 to 26, 70. Mask bits 4 and 8 together SHALL raise it so the top is 65. A clear bit 0x200 and a clear hold bit SHALL draw none. The anchor after that lower tab is not in.

#### Scenario: A seat of 80 sits at 16, 60
- **WHEN** unit 5 dispatches a held form that is not wide, with style bits 0x80, bit 0x200, a grip of 10, and a seat of 80
- **THEN** the lower tab runs from 16, 60 to 26, 70
