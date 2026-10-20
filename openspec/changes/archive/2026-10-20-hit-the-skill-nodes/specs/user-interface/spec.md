## ADDED Requirements

### Requirement: Skill node hit
For each node within 23 units of its centre, five samples SHALL sit at 18.3 units around it, 0.2 of a turn apart, with `x + round(sin) × turn` and `y − round(cos) × turn`. A sample within 5 units SHALL OR its index into the result and add one. The result SHALL start at 0 on page 2 and −1 otherwise.

#### Scenario: A sample and a miss
- **WHEN** page is 2, the unit is 4, the turn is 1, the node is at 100, 100 and the mouse is at 100, 27
- **THEN** the result is 1
- **WHEN** the mouse is at 170, 77
- **THEN** the result is 2
- **WHEN** the page is 0 and the mouse is far
- **THEN** the result is −1
