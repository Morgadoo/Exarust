## ADDED Requirements

### Requirement: Line shade
The shade SHALL draw six outer quads with the binds cap, bar, cap, cap, bar, cap, then a solid inner rectangle inset by the unit. The outer bar SHALL halve the unit with an integer shift. State band 0x10000 SHALL select the special ink with alpha 255, and the inner ink SHALL force alpha 255. Three mode calls, two begins, two ends, twenty-eight vertices and twenty-eight colour writes SHALL be counted.

#### Scenario: The outer quads and the inner rectangle draw
- **WHEN** the shade is drawn at unit 5, place 10, 20, size 100 by 80, side inset 15, with a clear state
- **THEN** the top quads span y 13 to 23 and the bottom quads y 97 to 107, the inset edges are 25 and 95, and the inner rectangle is 15, 25 to 105, 95
- **AND** the binds are cap, bar, cap, cap, bar, cap
- **AND** the counts are 3 modes, 2 begins, 2 ends, 28 vertices and 28 colour writes

#### Scenario: The state band selects the special ink
- **WHEN** the state band is 0x10000
- **THEN** the outer ink is the special ink with alpha 255
- **WHEN** it is not
- **THEN** the outer ink is the normal ink
- **AND** the inner ink forces alpha 255

#### Scenario: The unit is written back as a 16-bit place field
- **WHEN** the shade is drawn at unit 5
- **THEN** the place's unit slot holds 5 afterwards
