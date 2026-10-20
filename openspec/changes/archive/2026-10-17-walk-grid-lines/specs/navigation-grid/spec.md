## ADDED Requirements

### Requirement: Grid line walk
The system SHALL walk a line on the grid as the original does: a cell a step along its major axis from the start's height; a floor more than 40 cm above the running height with less than 100 cm under its slab SHALL block (an upper slab with nothing measured under it passes over a lower floor); a floor in reach SHALL raise the running height; drops never block. Blocked, the end SHALL move back to where the line passes the last free cell's centre, or to the start.

#### Scenario: A wall ahead
- **WHEN** a line is walked from 0 to 300 cm toward a wall in the cells spanning 150–180 cm
- **THEN** it is blocked and ends at 135 cm, keeping the asked end's height

#### Scenario: A wall in the first cell
- **WHEN** the first cell a line enters is a wall
- **THEN** the walk ends at its start

#### Scenario: The arena's walls
- **WHEN** lines 30 m long are walked from the arena's start along +X, −X, +Z and −Z
- **THEN** they end at x = 4.95 m, x = −4.95 m, z = 10.95 m and z = −4.95 m

### Requirement: Grid straight walk
The system SHALL walk a line for a walker without walking feet from the floor under the start: a floor more than 50 cm above the floor last stood on, with less than 150 cm under its slab, SHALL block; every floor in reach SHALL be stood on, down as well as up; the end SHALL be clamped as the line walk's.

#### Scenario: Down a pit and up again
- **WHEN** a line crosses one cell 4 m lower and goes on over the ground
- **THEN** the straight walk is blocked at the ground after the pit and the line walk is not

### Requirement: Obstacle on a line
The system SHALL find an obstacle on a line longer than 40 cm when, from the start's height and following the floors, a floor more than 70 cm up has less than 150 cm under its slab, or a cell before the line's last two has a penalty over 60. A line of 40 cm or less SHALL meet none.

#### Scenario: A short line over a block
- **WHEN** a line of 40 cm and one of 120 cm cross a floor 80 cm up
- **THEN** the short line meets no obstacle and the long one does

### Requirement: Cost at a point
The system SHALL give a point's cost as its cell's clearance and penalty over 255, on the upper layer when the point's height lies above that slab's underside less 30 cm and below its floor plus 150 cm, else on the lower.

#### Scenario: The edge of a drop
- **WHEN** the cost of a point over a cell whose clearance is 132 is asked
- **THEN** its clearance share is 132/255

### Requirement: Nearest walkable point
The system SHALL keep a point whose cell's clearance plus penalty is under 80, on its floor's height; otherwise it SHALL flood best-first by distance over steps under 50 cm up and 2.1 m down, at most 220 cells, stopping at one costing under 80, and give the centre of the cheapest cell met. Off the floors it SHALL first take the nearest cell across the ground.

#### Scenario: The edge of a drop
- **WHEN** the nearest walkable point to a cell on a drop's edge (clearance 132) is asked
- **THEN** it is the centre of its first neighbour inward in the original's order, the diagonal one, costing 60

#### Scenario: Shipped starts
- **WHEN** the nearest walkable points to the starts of arena, arenao1 and exanima01 are asked
- **THEN** each start stays, exanima01's on its floor 10 cm up

### Requirement: Queries without a grid
Until a level's grid is built, and in a level without one, the queries SHALL answer as the original's fallback does: a line walk ends where asked, a line meets no obstacle, a point costs nothing and is its own nearest walkable point.

#### Scenario: Before the grid is built
- **WHEN** a line walk is asked with no grid
- **THEN** it ends where asked, unblocked
