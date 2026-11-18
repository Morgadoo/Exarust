## ADDED Requirements

### Requirement: Light flood cleanup box
Before a light floods again, the light flood SHALL take it out only of the cells in the box of `ceil(radius / 6 m) + 1` cells round the cell of its new position, clamped to the grid, in the original's 32-bit arithmetic (so an unranged light clears nothing); a cell outside the box SHALL keep its entry for the light.

#### Scenario: A light moved four cells
- **WHEN** a light with a one-cell radius floods again four cells from where it last flooded
- **THEN** its old cell still lists it, and the cells its new flood recorded do not include the old cell

### Requirement: Moving node cell bytes
The live scene SHALL give a moving root or a moved light the room of its cell clamped one cell in from the grid's edge, and SHALL move it to another sector only when that cell changes.

#### Scenario: Far off the grid
- **WHEN** a moving node stands far beyond the grid's low corner
- **THEN** its cell is (1, 1), where the load-time sort would take (0, 0)
