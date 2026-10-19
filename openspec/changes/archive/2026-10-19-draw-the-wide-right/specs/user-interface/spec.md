## ADDED Requirements

### Requirement: Wide right
The same wide form SHALL then draw a right side. At unit 5, width 100 and height 80 the bar SHALL run from 95, 10 to 105, 70. A cap SHALL run from y 0 to 10, and a cap from y 70 to 80. Style 0x10 without those bits, and a form that is not wide, SHALL draw none. The mark after that side is not in.

#### Scenario: The right side runs from 10 to 70
- **WHEN** unit 5 dispatches a wide form of width 100 and height 80 whose style word is 0x80
- **THEN** the right bar runs from 95, 10 to 105, 70 and the two caps meet it
