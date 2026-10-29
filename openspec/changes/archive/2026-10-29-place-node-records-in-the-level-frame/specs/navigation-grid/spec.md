## MODIFIED Requirements

### Requirement: Grid columns
The system SHALL fill a column from voxel 0 to the terrain's height at the cell centre (bilinear between the four vertices as the height pass moved them; none next to a hole, the hole mask read a byte at a time), then mark the voxel each static mesh's GI sample point rounds to (nearest, ties to even; not on border cells or voxel 255), then the points generated for tile-grid floors, half a voxel higher.

#### Scenario: A point on a voxel boundary
- **WHEN** a sample point sits at height 0 in a grid whose voxel 221 tops out at 0
- **THEN** it marks voxel 222, the even one of the tie

#### Scenario: Ground next to a hole's byte edge
- **WHEN** vertex 8 of a terrain row is a hole
- **THEN** a cell centre between vertices 7 and 8 still has ground and one between 8 and 9 has none


### Requirement: Built at load
The system SHALL build the grid when a level finishes loading, from what is under the level root once it has world transforms, off the main thread, and keep it until the next level; until then the level has none (the original's fallback: no obstacle, no cost).

#### Scenario: Shipped levels
- **WHEN** the grids of arena, arenao1 and exanima01 are built from the installation
- **THEN** arena has 1956 lower and 10 upper floor cells, arenao1 14677 lower and 3678 upper, exanima01 220850 lower and 2279 upper; exanima01's start reaches 46113 cells, its closed doors keeping it from the stair room, and on the floors as built it reaches 195049 cells including the stair room's foot a metre lower

## ADDED Requirements

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
