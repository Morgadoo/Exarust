## ADDED Requirements

### Requirement: Path coordinates
A world coordinate SHALL map to a cell as `trunc(world × inverse unit + 0.5)`, and a cell back to the world as `(cell − 0.5) × unit`.

#### Scenario: Round trip
- **WHEN** the world 0.5 maps with inverse unit 1
- **THEN** the cell is 1 and its world is 0.5
- **WHEN** the world −0.5 maps
- **THEN** the cell is 0 and its world is −0.5

### Requirement: Path step pricing
The eight neighbours SHALL be NW, N, NE, W, E, SW, S, SE with costs 7, 5, 7, 5, 5, 7, 5, 7. A step SHALL enter only a set floor with a rise of −20 through +4, A at most 180/(quality+1) and shifted B at most 600. Its cost SHALL add 10·dz², the direction cost, twice A and the shifted B; the heuristic SHALL be ten times the 3D distance, truncated.

#### Scenario: Accepted and refused steps
- **WHEN** the current floor is 10, the next is 12, A is 50 and B is 0
- **THEN** the step is accepted and costs 10·dz² + direction + 2A + shifted B
- **WHEN** the next floor is 0 or 32, A is 181 or shifted B is above 600
- **THEN** the step is refused
