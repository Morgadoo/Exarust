## ADDED Requirements

### Requirement: Wide side
The same wide form SHALL then draw a left side. At unit 5 and height 80 the bar SHALL run from -5, 10 to 5, 70. A cap SHALL run from y 0 to 10, and a cap from y 70 to 80. Style 0x10 without those bits, and a form that is not wide, SHALL draw none. The bottom after that side is not in.

#### Scenario: The side runs from 10 to 70
- **WHEN** unit 5 dispatches a wide form of height 80 whose style word is 0x80
- **THEN** the side bar runs from -5, 10 to 5, 70 and the two caps meet it
