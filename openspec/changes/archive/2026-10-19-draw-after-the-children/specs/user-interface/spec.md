## ADDED Requirements

### Requirement: Title and corners
After the children, a positive title measure SHALL be centred in the width, toward zero. A tint byte of 200 SHALL stay 200. Style bits equal to 0x10 SHALL draw four corners of six times the unit. Bit 2 SHALL add a four-unit square unless the mark is -4, and that top corner SHALL use 0, 0.4, 0, 0.7. Bit 0x400000 SHALL count a wide call and draw no corners. The later edge bars are not in.

#### Scenario: A title of 40 sits at 30
- **WHEN** unit 5 dispatches a form of width 100 and height 80 with style bits 0x10 and a title measure of 40
- **THEN** the title sits at 30 and the four corners have span 30
