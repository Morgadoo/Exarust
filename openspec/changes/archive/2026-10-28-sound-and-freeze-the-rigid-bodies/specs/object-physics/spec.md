## ADDED Requirements

### Requirement: Frozen body step
A frozen body's step SHALL not integrate: each part SHALL be put back on the particles the last tick left it at, with no inertia, gravity or speed band, and the body's sleep countdown SHALL lose one tick, floored at 1. A thawed body SHALL integrate again. No shipped body is frozen.

#### Scenario: A frozen body moving sideways
- **WHEN** a frozen limp part moving sideways at 1 m/s is ticked twice, the second time after being given 10 m/s
- **THEN** it stays at its previous particles both times and its countdown falls by exactly one each tick

### Requirement: Link-iteration count
Each body root SHALL keep a link-iteration count: 2 for a plain body plus one for each articulated child, 20 for a character, 10 once the character dies in play. The count SHALL be reached at iteration pass p when count ≤ p. The solve SHALL still run its 20 passes over every root whatever the count. The whole-count iteration SHALL run that many link passes numbered from 0.

#### Scenario: A body with two articulated children
- **WHEN** a prop with a child and a grandchild registers
- **THEN** its count is 4, reached at pass 4 and not at pass 3
