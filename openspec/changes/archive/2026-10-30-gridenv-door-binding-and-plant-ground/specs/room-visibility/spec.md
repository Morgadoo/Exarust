# Spec Delta

## MODIFIED Requirements

### Requirement: Direction step
The system SHALL step one cell opposite to a direction vector using the engine's quick yaw of `(x, z)` widened to double: `|a|` below the double π/4 steps −Z; strictly between the f32 π/2 widened less and plus the double π/4 steps −X; `|a|` above the double 3π/4 steps +Z; strictly between −π/2 plus and less the double π/4 steps +X; anything else (a NaN) does not step.

#### Scenario: Four directions
- **WHEN** the vector is (0, 1)
- **THEN** the step is (0, −1)
- **WHEN** the vector is (1, 0)
- **THEN** the step is (−1, 0)

#### Scenario: Diagonals
- **WHEN** the vector is (1, 1)
- **THEN** the quick yaw is 0.78517902 and the step is (0, −1)
- **WHEN** the vector is (1, −1)
- **THEN** the step is (0, 1)

## ADDED Requirements

### Requirement: Quick yaw
The engine's yaw of `(x, z)` SHALL be 0 when `z² + x²` is below the f32 1e-8; otherwise the cosine `z / √(z² + x²)` SHALL give π at or below −1, 0 above 1 and the engine's quick arccosine between (a NaN through the arccosine), negated when `x` is below zero.

#### Scenario: Axes and NaN
- **WHEN** the direction is (−1, 0), (0, −1) or (NaN, 1)
- **THEN** the yaw is −π/2, π or NaN

### Requirement: World-point room query
The room at a world point SHALL be the room of the cell index `row × width + column`, with the cell from the floored 1/600 conversion plus half the grid, accepted when it is at least 0 and below the cell count, so a column off the grid's side reads a cell of the row before or after.

#### Scenario: Column off the side
- **WHEN** a 4 × 4 open grid is asked for the point in column 4 of row 1
- **THEN** it answers the room of cell (0, 2)

### Requirement: Cell active query
A grid cell SHALL be active when it belongs to a room whose active-room flag is set; a cell in no room is not active.

#### Scenario: Entering
- **WHEN** the viewer has just entered a room
- **THEN** its cells are not active until its fade-in ends, and the cells of the room behind its door are not active

### Requirement: Door body binding
A placed door with a body SHALL bind to the link from its cell to the cell one step opposite its third matrix row's `(x, z)` in its cell's room, and the body SHALL be readable from that link and its reverse; with no such link nothing is bound.

#### Scenario: Both facings
- **WHEN** a door in a door cell faces away from the room behind it
- **THEN** it is bound to the link and both sides carry its body
- **WHEN** the same door faces the other way
- **THEN** no link is found and nothing changes
