## ADDED Requirements

### Requirement: GI additive passes
After the groups' draw the system SHALL add each voxel light and then each registered shape as its own pass into the volume's 8-bit target: the stored codes decoded, the pass's colour clamped at 0 added, and the sum encoded through the driver's measured steps, as the original's blended draws into its sRGB target do.

#### Scenario: Two faint passes
- **WHEN** two passes of a third of a code each reach a voxel holding code 10
- **THEN** it keeps code 10, where their sum encoded once would give 11

### Requirement: GI stored alpha
The system SHALL store the volume's alpha 1 − 5·occupancy as its nearest 8-bit code.

#### Scenario: Open voxel
- **WHEN** a drawn voxel holds no occupancy
- **THEN** its stored alpha is 1

### Requirement: GI shape list clear
The system SHALL empty the GI manager's shape list when a level is torn down, as the original's scene clear sets its count to zero; a shape entity that outlives the level SHALL stay alive and unlisted.

#### Scenario: A shape outlives the level
- **WHEN** a level is torn down while a shape entity that is not a level entity is listed
- **THEN** the list is empty and the entity still exists

### Requirement: GI splat frame
The system SHALL splat a mesh's points through its world frame in centimetres: the frame's axis rows over the voxel size, its translation over the voxel size less 0.5 plus the block's whole-voxel offset, each point at `(x·r0 + y·r1) + (z·r2 + t)`, a centred mesh then half a voxel higher, truncated towards zero; the grid cells' floor points SHALL be splatted the same way from the cell's own space.

#### Scenario: A lattice point at 6.5 m
- **WHEN** a mesh point at 650 cm is splatted into a block whose first voxel is −128
- **THEN** it marks column 160, not 159

### Requirement: GI terrain vertex grid
The system SHALL take a terrain column's surface from its tile's vertex grid as the mean of the quad's anti-diagonal vertices (column + 1, row) and (column, row + 1) in centimetres, `((a + b)·0.5)/fc + b4` voxels, over the tile's present 6 m blocks only.

#### Scenario: A flat 40 cm tile
- **WHEN** a level is built over a tile whose vertices all stand at 40 cm, in a block whose vertical offset is 34
- **THEN** every interior column's lowest slice is 36

### Requirement: GI shape order
The system SHALL inject the registered shapes in the manager's list order, each as its own blended pass.

#### Scenario: List order
- **WHEN** three shapes are registered and the first is removed
- **THEN** the pass takes the third, then the second

### Requirement: GI moving mesh box
The system SHALL box a moving GI mesh in each frame by its node's bounds centre ± its half extents through the frame's absolute axes, in centimetres over the voxel size, less and plus 1.5, rounded half to even; a mesh without its node's bounds SHALL be boxed by its points padded 1.5 voxels.

#### Scenario: A node's bounds
- **WHEN** a mesh whose bounds are 30 by 10 by 10 cm about (2010, 110, 2010) cm moves 20 cm along X
- **THEN** its uploaded region runs from (98, 98, 4) to (104, 102, 8) in texture axes

## MODIFIED Requirements

### Requirement: GI shape injection
The system SHALL add every registered GI shape's fragment body into the light step, per voxel, over the shape's clipped column box and slice range, after the point-light groups and the voxel lights. The shell bodies SHALL read the voxel's world centimetres through the shape's frame; the line and multi-point bodies SHALL measure in the volume's units (the block's width is 1), positions scaled into them, attenuation unchanged. A shape SHALL carry up to 16 points.

#### Scenario: Cuboid in the block
- **WHEN** a cuboid shape lies inside the camera's block
- **THEN** the voxels of its clipped box take its colour falloff each step and the voxels outside it take none

#### Scenario: A blast's point in a 51.2 m block
- **WHEN** a multi-point shape of attenuation 5000 has a point at 512 cm in a block 51.2 m wide
- **THEN** the point is uploaded at 0.1 and lights voxels within √(1/5000) of the width, about 72 cm


### Requirement: GI undrawn voxels
The system SHALL clear the voxels of unmarked sectors, of sectors without geometry, of the margins, of the slices below a group's floor and of the top slice every step, as the original clears its target and draws only the groups, and SHALL still add the voxel lights and the shapes into them.

#### Scenario: Sector leaving the view
- **WHEN** the camera turns away from a lit sector
- **THEN** the sector's light is zero from the next step, apart from voxel lights and shapes

### Requirement: GI dynamic occupancy
The system SHALL update a moving GI mesh's occupancy every frame by splatting its old points with weight −20 and then its new points with +20 in each built block whose whole-voxel range meets the union of the two frames' boxes, and SHALL upload that box less the block's first voxel, ending one voxel short of the block's size. A thin mesh SHALL weigh a quarter and a centred one SHALL add half a voxel to the height and leave its column's lowest slice alone.

#### Scenario: Tie in the padding
- **WHEN** a point lies 4 voxels from the block's corner
- **THEN** its region runs from voxel 2 to voxel 6

