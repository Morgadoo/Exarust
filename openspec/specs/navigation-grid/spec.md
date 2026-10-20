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
