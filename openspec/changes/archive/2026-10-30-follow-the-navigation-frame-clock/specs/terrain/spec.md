## ADDED Requirements

### Requirement: Vegetation scatter grid
The grass point set's draws SHALL be the 32-bit step widened to double times 2⁻³², narrowed once. Its neighbour grid's cell side SHALL be the radius times the original's double for 1/√2, narrowed once; the grid side SHALL be the tile side over the cell rounded to nearest even plus one, and a point's cell each coordinate over the cell rounded to nearest even. A cell SHALL list at most four points, and retiring an active point SHALL move the list's last point into its slot.

#### Scenario: The cell side
- **WHEN** the radius is 1.3
- **THEN** the cell side's bits are 0x3F6B533C and the grid is 262 cells a side

#### Scenario: A half-cell coordinate
- **WHEN** a point lies at 2.5 and 3.5 cells
- **THEN** its cell is (2, 4)
