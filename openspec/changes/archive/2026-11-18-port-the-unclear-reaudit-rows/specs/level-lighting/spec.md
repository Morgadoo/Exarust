## ADDED Requirements

### Requirement: Light flood box clear
A light's re-flood SHALL first take it out of the cells of a box around its new position: the cell holding it and the ceiling of its radius over 600 cm plus one more cells each way, clamped to the grid; entries in cells outside the box SHALL stay.

#### Scenario: A light that moved
- **WHEN** a light with a one-cell reach moves six cells and floods again
- **THEN** its old cell still lists it
