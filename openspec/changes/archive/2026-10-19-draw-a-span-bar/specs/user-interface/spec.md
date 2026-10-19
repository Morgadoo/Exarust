## ADDED Requirements

### Requirement: Span bar
A span bar SHALL draw three quads and count one preparation. Stored word 4 SHALL double to thickness 8. At unit 5, origin 10, 20 and span 100, the middle SHALL be (15, 20), (15, 28), (105, 28), (105, 20). The left cap SHALL be (15, 20), (15, 28), (10, 28), (10, 20). The right cap SHALL be (105, 20), (105, 28), (110, 28), (110, 20). The middle sample SHALL bind once and the cap sample twice. Stored word -3 SHALL use thickness -6. The view scale and the colour word are not in.

#### Scenario: Unit 5 draws the three quads
- **WHEN** unit 5 draws a span from 10, 20 with span 100 and stored word 4
- **THEN** one preparation is counted, the middle sample is bound once, the cap sample is bound twice, and the three quads have the corners above

#### Scenario: A negative stored word doubles
- **WHEN** unit 5 draws a span from 10, 20 with span 100 and stored word -3
- **THEN** the thickness is -6
