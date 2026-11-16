## ADDED Requirements

### Requirement: The barricade's place score
The combat tick's place score for a barricade SHALL measure the nearest wall point along the wall from the fighter's chest, clamped to the half width, and the distance and bearing from the scored place to it across the ground.

#### Scenario: A chest beyond the wall's end
- **WHEN** the chest is 900 cm along a wall of half width 300 and the place is 100 cm off the wall's middle
- **THEN** the distance is measured to the wall's end, 300 cm along

### Requirement: The barricade's keep weight is the approach term
The barricade's keep weight SHALL be written to the approach term, with the reach the step length times 1.5 (1.4 in double above a body scale of 1.3) plus the reach word, the window doubled past a feral level of a half, and the lunge reach kept; the close and danger terms stay zero.

#### Scenario: A place near the wall
- **WHEN** a place is within the keep window
- **THEN** its approach term is positive and its danger term is zero
