## ADDED Requirements

### Requirement: Late close
A close mark of -4 SHALL draw one square after the other quads. At unit 5 and width 100 the square SHALL run from 100, 0 to 80, 20. A mark of 0 or -3 SHALL draw none. The hold-bit edge test is not in.

#### Scenario: A mark of -4 draws the square
- **WHEN** unit 5 dispatches a form of width 100 whose close mark is -4
- **THEN** one square runs from 100, 0 to 80, 20, and a mark of 0 draws none
