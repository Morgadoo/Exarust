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
- **THEN** arena has 1935 floor cells, arenao1 14677 lower and 3657 upper, exanima01 219292 lower and 792 upper, and exanima01's start reaches its stair room's foot a metre lower

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
A search on the grid SHALL return cell-centre points from the start cell to the goal cell that do not enter a blocked cell. When a wall sits between them, the path SHALL be longer than the straight line.

#### Scenario: A wall across the straight line
- **WHEN** a flat grid has a wall between two walkable cells and a gap at the wall's end
- **THEN** the search reaches the goal, no point of the path is on the wall, and the path is longer than the straight line

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
A cell whose clearance and penalty sum to less than 80 SHALL keep the point and take the floor height. A cell at 80 SHALL step to the first cheaper neighbour centre.

#### Scenario: Under 80 stays, 80 steps aside
- **WHEN** a point sits off the centre of a cheap cell, and a point sits on a cell whose penalty is 80
- **THEN** the cheap point keeps its place at the floor height, and the costly point moves to the centre of the neighbour one cell toward the origin

### Requirement: Footprint
A footprint SHALL add 63 to its centre, 32 to the eight neighbours, and 16 to the twelve cells beyond, on the layer whose underside is within three below the height and whose floor is within twenty-five above. The outer five cells SHALL not be stamped. Removing it SHALL take the same amounts back. That centre SHALL make the obstacle test fire, and removing it SHALL clear the test.

#### Scenario: Sixty-three at the centre, then gone
- **WHEN** a footprint is stamped at a floor of height 1, removed, tried in the outer five cells, and stamped again on an upper floor at height 5
- **THEN** the centre is 63, a neighbour is 32, a cell beyond is 16, the obstacle test fires and then clears, the outer cell stays 0, and the upper floor's centre is 63 while the lower stays 0

### Requirement: Plan around a footprint
A plan SHALL lift the footprint at the standing point, search from there, and stamp that footprint again. That search SHALL cost less than the same search with the footprint still down, and the centre SHALL be 63 again afterwards.

#### Scenario: The lifted search costs less, then the footprint is back
- **WHEN** a footprint is stamped and a search runs four cells away, and a plan then runs that same search
- **THEN** the plan costs less than the search with the footprint still down, the centre is 63, and a neighbour is 32

### Requirement: Dirty rectangle
A rectangle SHALL merge when its padded union is strictly smaller than the two padded rectangles, and the queue SHALL keep sixteen. A refresh SHALL grow each side by three cells. A sample from 1 to 254 SHALL raise the lower floor when it is 0 or within fifteen, and the upper floor when higher. One at or under the lower floor and its underside SHALL raise an empty upper floor. A changed floor SHALL be written and the costs SHALL run again. A zero-extent box SHALL cover its cell and a neighbour.

#### Scenario: Close rectangles join, and a sample raises the floor
- **WHEN** a zero-extent box is taken around cell 12, cell 12 is queued after cell 10, cell 17 is queued after cell 10, seventeen separated rectangles are queued, and a refresh is given voxels 20, 30 and 12, plus 255, 0 and a voxel outside
- **THEN** the box covers cell 12 and a neighbour inside the margin of three, the close pair is one rectangle, the far pair is two, seventeen keep the newest sixteen, voxel 20 raises the lower floor to 20 and its roughness is no longer 0, voxel 30 raises the upper floor to 30, voxel 12 raises an empty upper floor to 12, and voxel 255 and the outside floor stay as they were

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

### Requirement: Path search
The search SHALL pop a LIFO open list by the smallest f (newest wins a tie), stop at the acceptance distance, an empty list or 4095 nodes, re-parent an open neighbour to a smaller f, and rebuild the path as the parent chain collapsed through collinear runs. It SHALL count the goal as reached only when the best node is the goal cell.

#### Scenario: Straight, detour and sealed
- **WHEN** a clear five-by-five grid is searched between (0, 2) and (4, 2)
- **THEN** the path runs straight along y = 2 from the start to the goal and the goal is reached
- **WHEN** a wall blocks the direct row
- **THEN** the path detours around it and every point is walkable
- **WHEN** the goal is sealed by all its in-bounds neighbours
- **THEN** the search reports the goal as not reached

### Requirement: Walkable snap
The snap SHALL return the start cell when it has a floor and its A and B bytes sum under 80, with height base + floor·10. Otherwise it SHALL sweep the 32×32 window in distance order, up to 220 nodes, and return the nearest walkable cell or nothing.

#### Scenario: Walkable, nearby and none
- **WHEN** the start cell is walkable
- **THEN** it is returned at once with the snapped height
- **WHEN** the start is blocked and one cell in the window is walkable
- **THEN** the sweep returns that cell
- **WHEN** every cell in the window is blocked
- **THEN** the snap returns nothing within the node cap

### Requirement: Path shortcut
The shortcut SHALL keep the first point, drop each point whose direct line from the last kept point to the following point is clear, keep it otherwise, and always keep the last point.

#### Scenario: Clear and blocked lines
- **WHEN** every line is clear on a four-point straight path
- **THEN** only the first and last points remain
- **WHEN** a line is blocked at a bend
- **THEN** the bend is kept and the smooth run after it is shortened
- **WHEN** the path has two points
- **THEN** it is returned unchanged

### Requirement: Path endpoint fix-up
The endpoint fix-up SHALL drop up to eight points from the path's end whose penalty byte is above 64/(quality+1), always keep the first point, and report how many remain.

#### Scenario: High penalties and the eight cap
- **WHEN** the last three of five points carry penalty 100 at quality 0
- **THEN** two points remain
- **WHEN** eleven points carry high penalties in a row
- **THEN** only eight are dropped and four remain
- **WHEN** the quality is 1
- **THEN** a penalty of 40 is dropped
