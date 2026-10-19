## ADDED Requirements

### Requirement: Wide top
A wide form whose style word has bits 0xc0 equal to 0x80 SHALL draw three top quads. At unit 5 and width 100 the left cap SHALL run from 0, -5 to 10, 5, the bar from 10, -5 to 90, 5, and the right cap from 90, 5 to 100, -5. Style 0x10 without those bits, and a form that is not wide, SHALL draw none. The side bars after that top are not in.

#### Scenario: The bar runs from 10 to 90
- **WHEN** unit 5 dispatches a wide form of width 100 whose style word is 0x80
- **THEN** the bar runs from 10, -5 to 90, 5 and the two caps meet it
