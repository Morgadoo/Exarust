# navigation-grid Specification

## Purpose
Gives every level the original's navigation grid: 30 cm cells of 10 cm
voxel columns built from its terrain, its meshes' GI sample points and its
tile-grid floors, reduced to two walkable layers with the cost bytes the
planner, the player's look-ahead and the NPCs read.

## Requirements

### Requirement: Grid size and height
The system SHALL give each level a navigation grid of 30 cm cells, 20 per 6 m tile-grid room (320 × 320 when the grid gives no width), centred on the level origin, each a column of 256 voxels of 10 cm. Voxel 0 SHALL top out 25.5 m below the level's grid top: the environment record's `0x90`, or its highest point `0x98` when `0x90` is 0, or 3.4 m without the record.

#### Scenario: exanima01
- **WHEN** exanima01's grid is built (32 × 40 rooms, grid top 340 cm)
- **THEN** it has 640 × 800 cells and a floor at 0 m reads voxel 221

#### Scenario: Arena without a grid top
- **WHEN** a level's record stores 0 at `0x90` and 340 cm at `0x98`
- **THEN** its grid top is 3.4 m

### Requirement: Grid columns
The system SHALL fill a column from voxel 0 to the terrain's height at the cell centre (bilinear between the four vertices as the height pass moved them; none next to a hole, the hole mask read a byte at a time), then mark the voxel each static mesh's GI sample point rounds to (nearest, ties to even; not on border cells or voxel 255), then the points generated for tile-grid floors, half a voxel higher.

#### Scenario: A point on a voxel boundary
- **WHEN** a sample point sits at height 0 in a grid whose voxel 221 tops out at 0
- **THEN** it marks voxel 222, the even one of the tie

#### Scenario: Ground next to a hole's byte edge
- **WHEN** vertex 8 of a terrain row is a hole
- **THEN** a cell centre between vertices 7 and 8 still has ground and one between 8 and 9 has none


### Requirement: Generated floor points
The system SHALL generate tile-grid floors' points as the original does: the highest upward face over each 10 cm column marks the 10 cm cell whose top is the nearest 10 cm boundary to it, and every 20 cm cube on the world's 20 cm lattice whose four columns all mark it gets a point at its centre.

#### Scenario: A flat cell floor
- **WHEN** a 6 m square floor lies at height 0
- **THEN** it gets 900 points, all 10 cm under it

### Requirement: Spans and layers
The system SHALL scan each interior column from the top a byte at a time as the original does: a slab with over 12 free voxels above (counted from the scan's last change) starts a span, the first always; closer slabs join the span above. A span keeps its floor (top voxel, 255 above 240) and the underside the scan leaves; at most three, a non-last one dropped if its underside is 235 or more. The top span SHALL be the upper layer, the next the lower; a single span the lower.

#### Scenario: Ceiling over a floor
- **WHEN** a column has a floor slab to voxel 219 and a slab from 235 to 237
- **THEN** its upper layer has floor 237 and underside 232 and its lower layer floor 219

#### Scenario: Ceiling counted from its byte's bottom
- **WHEN** the floor slab reaches voxel 221 instead
- **THEN** the scan counts 11 free voxels (from 232), and the column has one span, floor 237

#### Scenario: Wall top
- **WHEN** a wall's points reach 2.9 m in a level whose grid top is 3.4 m
- **THEN** its cell's floor is open sky (255)

### Requirement: Cost bytes
Per layer the system SHALL derive roughness (no floor 120, open sky 60, else the hardest step onto a neighbour, 0–15, × 256 orthogonal or 181 diagonal / 64), a blur ((own + Σ neighbour × weight / 512) / 2) and the clearance (no floor 240, else the largest of own blurred value and each neighbour's, capped at 120, × weight / 512), each pass one cell further from the border. Size class s SHALL pass a clearance ≤ 180/(s + 1).

#### Scenario: Edge of a drop
- **WHEN** a flat floor ends at a column with no ground
- **THEN** the floor cells from the edge read clearance 132, 60, 18, then 0

#### Scenario: A 40 cm step
- **WHEN** a floor rises by 40 cm between two cells
- **THEN** both read roughness 16

### Requirement: Built at load
The system SHALL build the grid when a level finishes loading, from what is under the level root once it has world transforms, off the main thread, and keep it until the next level; until then the level has none (the original's fallback: no obstacle, no cost).

#### Scenario: Shipped levels
- **WHEN** the grids of arena, arenao1 and exanima01 are built from the installation
- **THEN** arena has 1956 lower and 10 upper floor cells, arenao1 14677 lower and 3678 upper, exanima01 220850 lower and 2279 upper; exanima01's start reaches 46113 cells, its closed doors keeping it from the stair room, and on the floors as built it reaches 195049 cells including the stair room's foot a metre lower

### Requirement: Terrain blocks in the grid
The system SHALL fill the grid's terrain a 6 m block at a time over the blocks the level's terrain builds: in a level with a tile grid only those whose grid cell carries the terrain flag, otherwise every block of every tile. A block's 20 × 20 cells SHALL take the ground's height where a whole-tile fill takes it, and an absent block SHALL leave its cells without terrain.

#### Scenario: Rock between rooms
- **WHEN** exanima01's grid is built from the installation
- **THEN** terrain fills its 682 flagged blocks only, and of the cells over its 477 closed grid cells only 1120, at the walls' edges, have a floor (186 721 when whole tiles were filled)

#### Scenario: A tile with two blocks
- **WHEN** a tile whose blocks (0, 0) and (5, 2) alone exist is filled
- **THEN** 800 cells get a floor, each at the voxel a whole-tile fill gives it, and the cells of block (1, 0) get none

#### Scenario: No tile grid
- **WHEN** a level without a tile grid has a terrain tile
- **THEN** all its 64 blocks fill, 25 600 cells

### Requirement: Look-ahead
The look-ahead SHALL be 75 cm + 6·v² forward, 45 cm + 6·v² in combat, and 70 cm backward, where v is the speed along the heading in centimetres a tick.

#### Scenario: Standing and walking
- **WHEN** the speed is 0, then 2 cm a tick
- **THEN** the forward reach is 75 cm, then 99 cm, and the combat reach at 2 cm a tick is 69 cm

#### Scenario: Backing
- **WHEN** the character backs up
- **THEN** the look-ahead is 70 cm behind the facing

### Requirement: Path around a wall
A search on the grid SHALL return cell-centre points from the start cell to the goal cell that do not enter a blocked cell. When a wall sits between them, the path SHALL be longer than the straight line and keep a corner.

#### Scenario: A wall across the straight line
- **WHEN** a flat grid has a wall between two walkable cells and a gap at the wall's end
- **THEN** the search reaches the goal, the path has more than two points, no point of the path is on the wall, and the path is longer than the straight line

### Requirement: Line walk
A line walk SHALL step from the cell after the start along the longer axis. A floor more than four above the current height, whose underside is within ten, SHALL block. The first blocked step SHALL keep the start. A later one SHALL stop before that cell and keep the asked height. A clear line SHALL keep the asked end.

#### Scenario: An open floor, then a floor too high
- **WHEN** a line crosses an open floor from cell 1 to cell 6, the same line meets a floor of 40 at cell 4, and a line starts on the cell before that floor
- **THEN** the open line keeps its end, the walled line blocks before cell 4 and keeps the asked height, and the line that starts against the floor keeps its start

### Requirement: Straight walk
A straight walk SHALL allow a floor five above the current height and SHALL block one six above when the underside is within fifteen. After a lower floor the height SHALL become that floor, so a later rise is measured from it.

#### Scenario: Five is allowed, and a drop is remembered
- **WHEN** a floor rises by five, the same floor rises by six, and a floor drops by four before rising by six
- **THEN** the rise of five is open while the line walk blocks it, the rise of six blocks, and the drop makes the later rise block

### Requirement: Obstacle
A line of 40 cm or less SHALL not be an obstacle. A longer line SHALL be one when a floor is more than seven too high, or when a cell before the last two has a penalty above 60. A penalty of 60, and a penalty above 60 on a last cell, SHALL not.

#### Scenario: Forty centimetres is clear, sixty-one is not
- **WHEN** a high floor is one cell ahead, the same floor is two cells ahead, a penalty of 61 is on an early cell, and a penalty of 60 is early while 61 is on the last cell
- **THEN** one cell ahead is not an obstacle, two cells ahead is, the early 61 is, and the 60 with a late 61 is not

### Requirement: Nearest walkable
A cell whose clearance and penalty sum to less than 80 SHALL keep the point and take the floor height. Otherwise the system SHALL flood best-first by distance from the point over steps under 50 cm up and 2.1 m down, in the original's neighbour order, meeting at most 220 cells, stopping at one costing under 80, and give the centre of the cheapest cell met on its floor. Off the floors it SHALL first take the nearest cell with a floor across the ground.

#### Scenario: Under 80 stays, 80 steps aside
- **WHEN** a point sits off the centre of a cheap cell, and a point sits on a cell whose penalty is 80
- **THEN** the cheap point keeps its place at the floor height, and the costly point moves to the centre of the neighbour one cell toward the origin

#### Scenario: The edge of a drop
- **WHEN** the nearest walkable point to a cell on a drop's edge (clearance 132) is asked
- **THEN** it is the centre of its first neighbour inward in the original's order, the diagonal one, costing 60

#### Scenario: Past 220 cells
- **WHEN** every cell costs 100 but one walkable cell 9 cells from the point, then also one 5 cells away
- **THEN** the point first lands on its own cell's centre, then on the nearer walkable cell

#### Scenario: A step too high
- **WHEN** the only walkable neighbour of a costly cell lies 50 cm up, then 40 cm up
- **THEN** it is not stepped onto, then it is

### Requirement: Cost at a point
The system SHALL give a point's cost as its cell's clearance and penalty over 255, on the upper layer when the point's height lies above that slab's underside less 30 cm and below its floor plus 150 cm, else on the lower.

#### Scenario: Two layers
- **WHEN** the cost of a point on the ground under a cell whose clearance is 132 is asked, and of a point on an upper floor 1.5 m up whose clearance is 60
- **THEN** the shares are 132/255 and 60/255

### Requirement: Footprint
A footprint SHALL add 63 to its centre, 32 to the eight neighbours, and 16 to the twelve cells beyond, on the layer whose underside is within three below the height and whose floor is within twenty-five above. The outer five cells SHALL not be stamped. Removing it SHALL take the same amounts back. That centre SHALL make the obstacle test fire, and removing it SHALL clear the test.

#### Scenario: Sixty-three at the centre, then gone
- **WHEN** a footprint is stamped at a floor of height 1, removed, tried in the outer five cells, and stamped again on an upper floor at height 5
- **THEN** the centre is 63, a neighbour is 32, a cell beyond is 16, the obstacle test fires and then clears, the outer cell stays 0, and the upper floor's centre is 63 while the lower stays 0

### Requirement: Footprints kept by the characters
Each fixed tick, in the behaviour stage, every character under its controller SHALL move its footprint to where its feet stand: an NPC always, whatever its condition; the player unless its condition is the walk's, its footprint lifted while it walks. A character that leaves, or loses its controller, SHALL take its footprint off; a new level's grid SHALL start with none.

#### Scenario: A moving NPC and a walking player
- **WHEN** an NPC stands in cell 20 and the walking player in cell 23, then the NPC moves to cell 18, then leaves
- **THEN** cell 20 reads 63 and cell 23 nothing, then cell 18 reads 63 and cell 20 the 16 of two cells along the axis, then no cell has a penalty

### Requirement: Plan around a footprint
A plan SHALL lift the footprint at the standing point, search from there, and stamp that footprint again. That search SHALL cost less than the same search with the footprint still down, and the centre SHALL be 63 again afterwards.

#### Scenario: The lifted search costs less, then the footprint is back
- **WHEN** a footprint is stamped and a search runs four cells away, and a plan then runs that same search
- **THEN** the plan costs less than the search with the footprint still down, the centre is 63, and a neighbour is 32

### Requirement: Dirty rectangle
A moved box SHALL mark the cells `W/2 + round((c ∓ e) · (1/30) ∓ 2.5)` on each axis, clamped to three from the edges. A rectangle SHALL merge when its padded union is strictly smaller than the two padded rectangles, else be appended. The scene-wide countdown SHALL hold the merge but not the sixteen-slot ring. A refresh SHALL raise floors from samples as the mover refresh does.

#### Scenario: Close rectangles join, and a sample raises the floor
- **WHEN** a zero-extent box is taken around cell 12, close and far rectangles are queued, and a refresh is given voxels 20, 30 and 12, plus 255, 0 and a voxel outside
- **THEN** the box covers cells 10 to 15, the close pair is one rectangle and the far pair two, voxel 20 raises the lower floor to 20, voxel 30 the upper floor to 30, voxel 12 an empty upper floor under a slab at 15, and the others change nothing

### Requirement: Pathfinder metrics
The constructor SHALL store cell size 30, depth 10, inverse 1/30, a zero counter and the seed byte, then run the base once.

#### Scenario: The opening
- **WHEN** the seed byte is 7
- **THEN** the cell is 30, the depth 10 and the inverse 1/30
- **AND** the counter is 0 and the seed is 7
- **AND** one base call runs

### Requirement: Footprint extents
With axes 10 and 10, inverse cell 1, extents 1, 1 and 1, and local values 2, 0, 4 elsewhere zero, the extents SHALL give low bounds 5 and 5 and high bounds 6 and 6. The slot SHALL advance by one modulo 16.

#### Scenario: The extents
- **WHEN** the axes are 10 and 10, the inverse is 1 and the local values are 2 and 4 at 0 and 2
- **THEN** the low bounds are 5 and 5 and the high bounds are 6 and 6
- **WHEN** the slot is 15
- **THEN** it becomes 0

### Requirement: Footprint rectangles
The planner SHALL merge a footprint into the first rectangle whose padded union is smaller than the two padded areas. Otherwise it SHALL append the footprint, adding eight to the capacity when the length has reached it. A busy planner SHALL leave the list alone. The padded area SHALL be `(x1 − x0 + 6) × (y1 − y0 + 6)`.

#### Scenario: Merge, append and grow
- **WHEN** `(0, 0, 9, 9)` is planned into an empty list, then `(5, 5, 14, 14)`, then `(100, 100, 109, 109)`
- **THEN** the first is appended at 0 and the capacity is 8
- **AND** the second merges into 0, leaving `(0, 0, 14, 14)`
- **AND** the third is appended at 1
- **WHEN** the planner is busy
- **THEN** nothing changes
- **WHEN** the length has reached a capacity of 1
- **THEN** appending raises the capacity to 9

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

### Requirement: Line stamp
Stamping a line SHALL weigh each line point's cell 255, the eight cells around each line cell 128 (255 when reached a second time), and the cells beside those that neither pass weighed 64 more each time they are reached, at most 255. Each weighed cell SHALL add a quarter of its weight to its layer's penalty byte, wrapping, and taking the stamp back SHALL subtract the same.

#### Scenario: A two-cell line
- **WHEN** a line runs through two neighbouring cells on the lower layer
- **THEN** the line cells gain 63, a cell beside both gains 63, a corner of one gains 32, and taking the stamp back leaves every penalty as it was

### Requirement: Line stamp layer
A line cell SHALL be weighed on the layer whose height band holds its point, the lower layer first, and the cells around the line on the layer that holds the first point's height. A height band SHALL hold a height, in 10 cm steps over the grid's base, more than three over the cell's underside and under its floor plus 25.

#### Scenario: An upper floor
- **WHEN** a point stands 96 steps up over a cell whose lower floor is at 10 and whose upper floor spans 90 to 100
- **THEN** its cell is weighed on the upper layer

### Requirement: Barricade stamp in play
A standing barricade's line SHALL be stamped on the navigation grid and taken back when its fall ends.

#### Scenario: A barricade on a grid
- **WHEN** a barricade stands on a grid and then falls
- **THEN** its line raises the grid's penalty while it stands and the penalty returns to zero after the fall

### Requirement: Installed pathfinder
The system SHALL answer pathfinder queries through the level's grid once it is built, and through a 64 x 64 abstract pathfinder while the grid is being built or when the level has none, the abstract pathfinder answering as if there were nothing there: no obstacle, no clamped line, no snapping, no blocked point, no reach, the point unchanged.

#### Scenario: Level without a grid
- **WHEN** a level has no navigation grid
- **THEN** the look-ahead is never blocked and a snap leaves the point as it was

### Requirement: Meshes with bodies in the stamp
A mesh whose node carries a rigid body SHALL mark no voxel unless its node is level geometry (a prop placed static, or the root of a placed object with the static bits).

#### Scenario: A loose sack
- **WHEN** a sack placed loose stands on a corridor floor
- **THEN** its points mark no voxel, and the same sack placed static marks its voxels

### Requirement: Moving props raise floors
Once the layers are written and before the cost passes, every mesh with GI points whose node carries a rigid body SHALL raise the floors under it, level geometry or not: each point, in world space, rounded to its cell and voxel as the mover refresh rounds a sample, inside cells 1 to W − 3 and D − 3 and voxels 0 to 254, raises its cell's current floors one point after another by the refresh's rule. The floors as built SHALL stay.

#### Scenario: A closed door
- **WHEN** a door leaf's points stand one voxel apart up its doorway, lowest first
- **THEN** the doorway's lower floor rises to the leaf's top and its floor as built stays

#### Scenario: A point high over the floor
- **WHEN** a point lies fifteen voxels or more over a cell's lower floor
- **THEN** it raises the cell's upper floor when that is missing
