## ADDED Requirements

### Requirement: Skill tooltip
At tooltip 100, 50 and unit 5 the icon SHALL draw at 10, 25 and the label SHALL occupy 80, 50 of size 30 by 10. The draw SHALL count one begin, one bind, two setup calls and two draws.

#### Scenario: The icon and label
- **WHEN** the tooltip is at 100, 50 and the unit is 5
- **THEN** the icon is at 10, 25
- **AND** the label is at 80, 50 of size 30 by 10
- **AND** the counts are one begin, one bind, two setup calls and two draws
