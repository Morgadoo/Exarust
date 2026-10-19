## ADDED Requirements

### Requirement: Capped span
At unit 5 with left 10, right 110 and centre 28 the capped span SHALL draw a left cap from 0, 18 to 30, 38, a right cap from 120, 18 to 90, 38, and a middle from 30, 28 to 90, 38. The cap sample SHALL bind twice and the middle sample once. Twelve colour writes SHALL be counted. The retail method's second positional float SHALL be dead and SHALL NOT be taken.

#### Scenario: The caps and the middle draw
- **WHEN** the capped span is drawn at unit 5 with left 10, right 110 and centre 28, colour 17 and a set view
- **THEN** the three quads are those above, the cap sample binds twice, the middle sample binds once, colour 17 is stored twelve times, and the placed corners follow the view

#### Scenario: A zero centre still splits at the extent
- **WHEN** the centre is 0 and the unit is 5
- **THEN** the top is -10 and the bottom is 10
