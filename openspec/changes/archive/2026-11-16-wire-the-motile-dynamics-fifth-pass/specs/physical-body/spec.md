## ADDED Requirements

### Requirement: Step radius base
The step trigger's radius base SHALL be 65 cm while the body is off its feet, 30 cm while it locomotes, and otherwise narrow by 0.5 cm a tick while above 30 cm (not floored) and return to 30 cm below it; a new body SHALL start at 60 cm.

#### Scenario: A spawned body walks
- **WHEN** a new body stands one tick, loses its support for one tick, then locomotes one tick
- **THEN** its radius base is 59.5, then 59, then 30 cm

### Requirement: Walk stop jolt seed
When a body in the walk's condition stops, the jolt's decaying term SHALL become the chest's slow velocity × −0.6 after that tick's jolt test, then decay by 0.85 a tick as before.

#### Scenario: The tick after a stop
- **WHEN** a body stops walking with a slow chest velocity of 2 cm a tick
- **THEN** the next tick's jolt adds 0.85 × 1.2 cm a tick back along it

### Requirement: Plane grip link
A held item's plain world link flagged as a plane link SHALL take 0.8 of its offset's part along the link's axis off before the plain law reads it.

#### Scenario: An offset along and across the axis
- **WHEN** a plane link's anchor is 1 cm off its point along its axis and 1 cm across it
- **THEN** the pull moves the point 0.2 cm along the axis and 1 cm across it
