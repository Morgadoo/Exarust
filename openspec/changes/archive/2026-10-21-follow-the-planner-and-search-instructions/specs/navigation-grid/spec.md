## ADDED Requirements

### Requirement: Planner cells
The planner SHALL turn the start and goal into cells by rounding `cm · (1/30) + ½` ties to even in single precision. It SHALL answer no path when either lies more than the grid's half size less ten from its middle, or they lie more than 253 cells apart on an axis. It SHALL search a 256-cell window centred between them and never enter the window's border.

#### Scenario: Rounding and the margin
- **WHEN** points at 0, 15, 30, −15 and −45 cm are turned into cells, and a plan starts at the eleventh cell from a 40-cell grid's middle
- **THEN** the cells are 0, 1, 2, 0 and −1, and the plan has no points

### Requirement: Planner heights
Heights SHALL be `(y − base) · 0.1` in centimetres, rounded ties to even. The start SHALL take the upper layer when its height lies above that layer's underside less three and below its floor plus fifteen, else the lower layer.

#### Scenario: Ties
- **WHEN** heights of 5, 15, 25 and −15 cm are turned into voxels
- **THEN** they are 0, 2, 2 and −2

### Requirement: Planner steps
A step SHALL go from the floor of the node it leaves, on that node's layer, to the eight neighbours in the order north-west, north, north-east, west, east, south-west, south, south-east. A cell without a lower floor SHALL be refused. A lower floor more than four above SHALL be refused unless ten lie under its slab, and then only the upper floor SHALL take the step. An upper floor within four and above the lower floor SHALL be entered.

#### Scenario: Floors in reach
- **WHEN** a step from floor 10 meets a cell with no lower floor, a lower floor of 12, a lower floor of 15 with its slab at 15 or 25 and no upper floor, and a lower floor of 2 under an upper floor of 13
- **THEN** it is refused, takes 12, is refused twice, and takes the upper 13

### Requirement: Planner upper slab
An upper floor more than four above SHALL refuse the cell when less than ten lie under its slab, unless nothing is measured there and the floor is not below the lower floor; otherwise the lower floor SHALL be kept.

#### Scenario: Slabs over the floor
- **WHEN** a step from floor 10 onto a lower floor of 10 meets an upper floor of 20 with its slab at 15, at 25, and with nothing measured under it
- **THEN** it is refused, then takes the lower floor twice

### Requirement: Planner step cost
A step's rise SHALL be −20 to +4 and the entered layer's clearance at most `180/(size + 1)`. It SHALL cost `10·Δ² + 5 or 7 + 2·clearance + (penalty << shift)`, the shift masked to 31, and SHALL be refused when the shifted penalty is over 600.

#### Scenario: A corridor and its penalty
- **WHEN** a plan walks five cells along a corridor, then with a penalty byte of 4 on one cell, unshifted, shifted by 1, by 8 and by 33
- **THEN** it costs 25, then 29, 33, ends at the cell before the penalised one, and 33

### Requirement: Planner order
New nodes SHALL go in front of a LIFO list; the node of least total, compared unsigned, SHALL be taken next, of equals the newest. A node met again more cheaply SHALL take the new cost and parent and keep its place; a taken node SHALL not be put back. A new node's heuristic SHALL be `round(10·√(Δx² + Δz² + Δh²))`, `Δh` its floor less the goal's height.

#### Scenario: The height leg runs to the goal
- **WHEN** a goal stands 50 cm over the corridor's floor and the accept distance is 55
- **THEN** the plan ends two cells short of the goal at cost 15

### Requirement: Planner end
The search SHALL stop when the closest node's heuristic is under the accept distance, when nothing waits, or at the 4096th node, the accept test still running. A search that did not reach the goal SHALL count as reached when the closest node's centre lies within 100 cm of the goal; otherwise its path SHALL end at the parent of the rough node (clearance over `64/(size + 1)`) nearest the start.

#### Scenario: 100 cm and rough ground
- **WHEN** a gap leaves the closest node 120 cm from the goal, with and without clearance 70 on it and on the start's neighbour
- **THEN** the plan is not reached and ends at the closest node, one cell back from it, or has no points

### Requirement: Planner corners
The path SHALL keep the end, the last node of each run of one direction on one layer, and the start. From the start, each corner after the next SHALL be tested with the line walk back to the last point kept, and the corner before it SHALL be kept when the line is blocked. Points SHALL be cell centres on their layer's floor, none when fewer than two corners remain.

#### Scenario: A straight corridor
- **WHEN** a plan walks five cells straight along a corridor
- **THEN** it has two points, the start's and the goal's cell centres on the floor

### Requirement: Planner line walk
The line walk SHALL hold the far corner's floor, step every cell of the longer axis to the kept point, refuse a cell as a step does, and pay the step cost with the penalty shifted once more. It SHALL be blocked by a refused cell or a cost over what the search paid between the two plus ten.

#### Scenario: The line walk's budget
- **WHEN** a line walks five flat cells with budgets 15 and 14, then with a penalty of 3 on one cell and budgets 21 and 20, then onto floors two higher with budgets 221 and 220, then over a gap
- **THEN** it is clear, blocked, clear, blocked, clear, blocked, and blocked

### Requirement: Mover refresh
A refresh SHALL drain the ring, grow each side by three cells and start the working floors from the floors as built. A sample voxel `y·0.1 + ½ − base·0.1` from 1 to 254 SHALL raise a missing working floor; one less than fifteen above SHALL take it; one fifteen or more above, or at or under the lower slab's underside, SHALL raise the upper floor when it is missing or less than fifteen below. Changed floors SHALL be written and the costs run again.

#### Scenario: A mover gone
- **WHEN** a sample at 2 m raises a cell's floor of 10 and a later refresh of that cell has no sample, and samples at 2.0, 2.04 and 1.96 m are rounded
- **THEN** the floor is 20, then 10 again with no roughness, and the voxels are 20, 21 and 20

### Requirement: Plan timing on the shipped levels
A plan as an NPC's (size class 3, penalties doubled) from each installed level's start to the farthest reached cell within the window SHALL give a path in under 250 ms.

#### Scenario: Arena, arenao1, exanima01
- **WHEN** the installation is present and the three levels' grids are built
- **THEN** each plan gives at least two points (0.3 to 1.1 ms on the worker)

## MODIFIED Requirements

### Requirement: Path around a wall
A search on the grid SHALL return cell-centre points from the start cell to the goal cell that do not enter a blocked cell. When a wall sits between them, the path SHALL be longer than the straight line and keep a corner.

#### Scenario: A wall across the straight line
- **WHEN** a flat grid has a wall between two walkable cells and a gap at the wall's end
- **THEN** the search reaches the goal, the path has more than two points, no point of the path is on the wall, and the path is longer than the straight line

### Requirement: Dirty rectangle
A moved box SHALL mark the cells `W/2 + round((c ∓ e) · (1/30) ∓ 2.5)` on each axis, clamped to three from the edges. A rectangle SHALL merge when its padded union is strictly smaller than the two padded rectangles, else be appended. The scene-wide countdown SHALL hold the merge but not the sixteen-slot ring. A refresh SHALL raise floors from samples as the mover refresh does.

#### Scenario: Close rectangles join, and a sample raises the floor
- **WHEN** a zero-extent box is taken around cell 12, close and far rectangles are queued, and a refresh is given voxels 20, 30 and 12, plus 255, 0 and a voxel outside
- **THEN** the box covers cells 10 to 15, the close pair is one rectangle and the far pair two, voxel 20 raises the lower floor to 20, voxel 30 the upper floor to 30, voxel 12 an empty upper floor under a slab at 15, and the others change nothing

## REMOVED Requirements

### Requirement: Path coordinates
**Reason**: The rule (truncation, a half-cell shift back) described an unused helper that disagreed with the planner's rounding.
**Migration**: Planner cells and Planner heights.

### Requirement: Path step pricing
**Reason**: It described an unused one-layer step with a truncated heuristic.
**Migration**: Planner steps, Planner step cost and Planner order.

### Requirement: Path search
**Reason**: It described an unused one-layer search stopping at 4095 nodes.
**Migration**: Planner order and Planner end.

### Requirement: Walkable snap
**Reason**: It described an unused duplicate of the nearest-walkable search.
**Migration**: Nearest walkable.

### Requirement: Path shortcut
**Reason**: It described an unused shortcut that keeps a different corner than the original.
**Migration**: Planner corners and Planner line walk.

### Requirement: Path endpoint fix-up
**Reason**: It read the planner's dead clearance loop as a trim.
**Migration**: Planner end (the rough cut).

### Requirement: Budgeted path shortcut
**Reason**: It described an unused budgeted shortcut with the wrong corner.
**Migration**: Planner corners and Planner line walk.

### Requirement: Path layer choice
**Reason**: It described an unused one-sided reading of the step's layer choice.
**Migration**: Planner steps and Planner upper slab.

### Requirement: Path stamp clear
**Reason**: It described an unused stamp-table helper.
**Migration**: Planner cells (the window's border is never entered).
