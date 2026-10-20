## ADDED Requirements

### Requirement: Path window
The system SHALL plan from a walker's standing point toward a goal only when both cells (round(x/30 + ½) from the grid's middle) lie at least 10 cells inside the grid's half size and within 253 cells of each other along each axis, searching a 256 × 256 window centred between them. The start's layer SHALL be the upper one when its height lies above that slab's underside less 30 cm and below its floor plus 150 cm.

#### Scenario: Too far apart
- **WHEN** a plan is asked between cells 254 apart on a flat floor
- **THEN** there is no plan, while cells 253 apart get a straight path

#### Scenario: Near the edge
- **WHEN** a 64-cell grid is asked for a path to cell 23 from its middle
- **THEN** there is no plan, while cell 22 gets one

### Requirement: Path steps
The system SHALL step from a cell's floor to each of its eight neighbours by the line walk's reach (a floor over 40 cm up is out of reach and blocks unless 100 cm lie under its slab; the upper floor when in reach and higher), rising at most 40 cm and dropping at most 2 m, into a cell whose clearance is at most 180/(size class + 1).

#### Scenario: A step up
- **WHEN** a corridor rises 40 cm for one cell
- **THEN** the path walks it, and at 50 cm it ends in front of the step

#### Scenario: A drop
- **WHEN** a corridor drops 2 m
- **THEN** the path takes it, and at 2.1 m it does not

#### Scenario: Room under a slab
- **WHEN** a slab hangs 90 cm over a corridor cell, or 100 cm
- **THEN** the first shuts the corridor and the second does not

#### Scenario: Size classes
- **WHEN** a corridor cell's clearance is 45, 46, 180 or 181
- **THEN** class 3 passes 45 but not 46, class 0 passes 180 but not 181

### Requirement: Path cost and order
Each step SHALL cost 10·Δh² (voxels), 5 along an axis or 7 diagonally, twice the clearance and the penalty shifted left by the goal's shift; a shifted penalty over 600 SHALL refuse the cell. The search SHALL expand the cell of least cost plus 10 × its 3-D distance to the goal (cells, voxels), of equals the one met last; a cell met again more cheaply SHALL take that way.

#### Scenario: Straight ways
- **WHEN** a path runs 10 cells along X or 5 diagonally over open floor
- **THEN** it costs 50 or 35 and is its two ends

#### Scenario: One rough step
- **WHEN** a 4-cell corridor's middle cell is 10 cm up with clearance 10 and penalty 20
- **THEN** the path costs 100 with the shift 1

#### Scenario: Penalties
- **WHEN** a corridor cell's penalty is 75 or 76 with the shift 3
- **THEN** 600 is paid and 608 refuses the cell

### Requirement: Path end
The search SHALL succeed once a cell met lies within the stop distance of the goal (10 a cell) and give up when nothing waits or its 4096th cell would be met. The path SHALL lead to the cell met nearest the goal; when the search did not succeed and that cell's centre lies over 100 cm from the goal (or the goal's own test rejects it), the path SHALL stop before its first cell rougher than 64/(size class + 1).

#### Scenario: Stop distance
- **WHEN** a path 10 cells long is asked with the stop distance 50
- **THEN** it ends 4 cells short of the goal, its goal reached

#### Scenario: A goal without a floor
- **WHEN** the goal cell of an open floor has no floor
- **THEN** the search meets 4095 cells and the path ends 30 cm from the goal, near enough

#### Scenario: A goal behind a hole
- **WHEN** the goal lies in the middle of a 9 × 9 hole
- **THEN** the path ends 150 cm from it, not near enough

#### Scenario: Rough cells before a failed end
- **WHEN** a dead-end corridor's last three cells read clearance 17
- **THEN** a class-3 path stops before them

### Requirement: Path corners
The path SHALL keep the cells where its direction or layer changes, then from the start skip each next-but-one corner whose straight line back to the last kept steps only on lower floors in reach of its own floor and costs (heights from its floor, penalties shifted once more) at most what the search paid between them plus 10. Its points SHALL be cell centres on their floors: the start's cell, the corners kept, the end's.

#### Scenario: A greedy bend pulled straight
- **WHEN** a path runs 10 cells along X and 3 along Z over open floor
- **THEN** it costs 56 and is its two ends

#### Scenario: An L-shaped corridor
- **WHEN** a corridor runs 6 cells along X then 6 along Z
- **THEN** the path cuts the inside corner diagonally and keeps both ends of the cut: 4 points

#### Scenario: Shipped levels
- **WHEN** exanima01 is asked for a path from (52.05, 0, 39.15) to (63.15, 0, 27.15) m
- **THEN** it reaches the goal through 11 points over 22.75 m, 810 cells met

### Requirement: Character footprints
Each character SHALL add to the penalty bytes round its cell, on the layer whose underside less 30 cm and floor plus 2.5 m enclose its height: 63 under it, 32 on its eight neighbours, 16 on the twelve cells beyond along the axes, wrapping; none within 5 cells of the grid's edge. NPCs SHALL stamp every tick, the player while not walking; a footprint SHALL move with its character and be lifted while it plans.

#### Scenario: One footprint
- **WHEN** a character stands on a flat floor
- **THEN** its cell's penalty is 63, its neighbours' 32, the twelve beyond 16, and lifting it leaves none

#### Scenario: A footprint on a corridor
- **WHEN** a path passes a footprint along a corridor with the shift 1, or the shift 4
- **THEN** it pays 318 more, and with the shift 4 its centre's 1008 refuses the way

#### Scenario: A walking player
- **WHEN** the player walks
- **THEN** it has no footprint, while a standing NPC has one where it stands

### Requirement: One plan a tick
The system SHALL let only one walker plan each tick: whoever holds the slot. Every other walker asking SHALL wait, and the one that has waited longest since its last plan, of equals the first to ask, SHALL hold the slot through the next tick; a wait of no ticks SHALL never be queued.

#### Scenario: Two waiters
- **WHEN** two walkers ask in one tick, having waited 10 and 5 ticks
- **THEN** the first holds the slot next tick, and the second the tick after

### Requirement: Moving things on the grid
The system SHALL take rectangles of cells from movers (their box across the ground plus 2.5 cells, 3 cells inside the grid), queued on a ring of 16 or merged into a list where a union, sides 6 longer, is smaller than the two apart. Each tick each rectangle grown 3 cells SHALL get its floors as built back, raised by the movers' points, and where a floor changed its costs SHALL be worked out again.

#### Scenario: A prop comes and goes
- **WHEN** a prop's point raises a floor cell 40 cm and the prop then leaves
- **THEN** the cell reads roughness 16 while it stands there, and clearance 0 again once refreshed after it left

#### Scenario: Merging
- **WHEN** rectangles 10–14 and 12–16 are marked, then 26–30
- **THEN** the first two merge into 10–16 and the third stays apart

## MODIFIED Requirements

### Requirement: Grid straight walk
The system SHALL walk a line for a walker without walking feet from the floor under the start: a floor more than 50 cm above the floor last stood on, with less than 150 cm under its slab, SHALL block; every floor in reach SHALL be stood on, down as well as up; the end SHALL be clamped as the line walk's. It SHALL read the floors as built, before movers raise them.

#### Scenario: Down a pit and up again
- **WHEN** a line crosses one cell 4 m lower and goes on over the ground
- **THEN** the straight walk is blocked at the ground after the pit and the line walk is not

### Requirement: Queries without a grid
Until a level's grid is built, and in a level without one, the queries SHALL answer as the original's fallback does: a line walk ends where asked, a line meets no obstacle, a point costs nothing and is its own nearest walkable point, and there is no path.

#### Scenario: Before the grid is built
- **WHEN** a line walk is asked with no grid
- **THEN** it ends where asked, unblocked

#### Scenario: No path without a grid
- **WHEN** a path is asked with no grid
- **THEN** there is none
