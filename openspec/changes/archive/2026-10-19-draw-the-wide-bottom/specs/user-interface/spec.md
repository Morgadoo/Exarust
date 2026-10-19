## ADDED Requirements

### Requirement: Wide bottom
The same wide form SHALL then draw a bottom. At unit 5, width 100 and height 80 the bar SHALL run from 10, 75 to 90, 85. A cap SHALL run from 0, 75 to 10, 85, and a cap from 90, 75 to 100, 85. Style 0x10 without those bits, and a form that is not wide, SHALL draw none. The right side is not in.

#### Scenario: The bottom runs from 10 to 90
- **WHEN** unit 5 dispatches a wide form of width 100 and height 80 whose style word is 0x80
- **THEN** the bottom bar runs from 10, 75 to 90, 85 and the two caps meet it
