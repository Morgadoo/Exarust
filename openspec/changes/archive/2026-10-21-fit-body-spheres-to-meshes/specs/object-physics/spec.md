# Spec Delta

## MODIFIED Requirements

### Requirement: Volume summary
The system SHALL summarise a body's voxel map as the original does: every filled cell from the low border to the last inner cell adds its index minus 0.5 to a running sum per axis, the centre is (sum/count/cells − 0.5) · half extent, and the mass is count · (2·h_z/n_z) · (2·h_y/n_y) · (2·h_x/n_x). A map with an axis of no cells SHALL have centre (0, 0, 0) and mass 0.1; a map with no filled cell SHALL have mass 0.

#### Scenario: A unit volume
- **WHEN** a 1 × 1 × 1 map over half extents (2, 4, 6) has its one cell filled
- **THEN** the centre is (0, 0, 0) and the mass is 384

#### Scenario: Two cells
- **WHEN** a 2 × 2 × 2 map over unit half extents has cells (1, 1, 1) and (1, 1, 2) filled
- **THEN** the mass is 2 and the centre is (−0.25, −0.25, 0)

#### Scenario: An empty volume
- **WHEN** a map has no cells along an axis
- **THEN** the centre is (0, 0, 0) and the mass is 0.1

## ADDED Requirements

### Requirement: Body grid counts
The system SHALL make a body's voxel grid over its mesh's stored box: the longest half extent (the first of equal ones) gets round(half · resolution) cells, ties to even, made even, at least the minimum depth and at most 96; each other axis gets n · half / longest half rounded up, made even when the builder asks. A longest half extent under 1 cm SHALL make no grid.

#### Scenario: Short axes round up
- **WHEN** a mesh's box has half extents 35 × 10 × 10 cm at 0.3 cells per cm
- **THEN** its grid is 10 × 4 × 4 cells (10.5 rounds to 10; 2.86 rounds up to 3 and is made even)

### Requirement: Builder resolution
The limb and root builders SHALL fit at 0.3 cells per cm when the mesh's smallest half extent is at least 10 cm and at 0.6 below it, with at least 6 cells along the longest axis, or at 1 cell per cm with at least 8 cells under their quality flag.

#### Scenario: A thin mesh
- **WHEN** a mesh's smallest half extent is 9.99 cm
- **THEN** it is fitted at 0.6 cells per cm

### Requirement: Body surface fill
The system SHALL mark every inner cell whose centre is within half a longest-axis cell of the mesh, then every empty cell with at least five marked neighbours whose offsets sum to at most 3 on each axis and whose centre is again within that radius. With the voxeliser's wide-query flag the two radii SHALL be √2/2 and 0.75·√2 of it, taken in double precision.

#### Scenario: The outer layer
- **WHEN** a 20 × 18.8 × 18.2 cm box is fitted over a 21 cm cube's 6 × 6 × 6 grid
- **THEN** exactly the cells of the first and last layer of each axis are marked

### Requirement: Body solid fill
The system SHALL make an empty cell solid when, for all three axes, a ray from its centre toward −axis reaching the mesh box widened by 0.01 cm and a ray toward +axis reaching 0.2 cm past it both hit the mesh.

#### Scenario: A box fills its grid
- **WHEN** the box above is fitted
- **THEN** its inner 4 × 4 × 4 cells are solid, and its 216 cells weigh 9261 cm³ with the centre at the box's

### Requirement: Body sphere query
A sphere SHALL touch a mesh face when the face's plane is no farther than the radius from its centre and the squared distance to the face's nearest point (the original's region test, with the processor's approximate reciprocals) is strictly under the squared radius less 1.192e-7. Speeding the query up SHALL NOT change any answer.

#### Scenario: Strictly inside the radius
- **WHEN** a point lies 1 cm beyond a face's corner in its plane
- **THEN** a sphere of radius 1 cm there does not touch the face and one of the next larger single-precision radius does

### Requirement: Body ray query
A ray SHALL hit a mesh face only when the face's normal points along it by at least 1e-8, the hit is between 1e-8 and the ray's length, and the hit lies in the face by the barycentric test on the two axes other than the normal's largest. Speeding the query up SHALL NOT change any answer.

#### Scenario: One-sided rays
- **WHEN** a ray crosses a face against its normal
- **THEN** it does not hit the face, while the same ray along the normal does

### Requirement: Body candidate spheres
The system SHALL take as candidates the cells 3 or more from the grid's ends whose eight corner cells are filled, with the radius in cells to the nearest cell short of eight filled corners, searched within the cell's distance to the grid's ends, kept when at least 1. Each candidate SHALL count the cells within r − ¼ of its corner point.

#### Scenario: A central sphere
- **WHEN** the 6 × 6 × 6 box above is fitted
- **THEN** its one candidate is the middle cell, 3 cells from the nearest cell short of eight corners

### Requirement: Body sphere pruning
The system SHALL sort the candidates by radius with the original's quicksort and, smallest first, drop any whose every counted cell another candidate also counts, taking its counts back, except the strictly largest. The survivors SHALL count their cells within r + ½ and become spheres at their corner point of radius r/(1 + 0.01·r) longest-axis cells, in double precision, in sorted order.

#### Scenario: Pruning order
- **WHEN** a solid 12 × 6 × 6 cell rod is fitted
- **THEN** its seven candidates of radius 3 are sorted 7, 8, 9, 6, 3, 4, 5 along it, cells 7 and 4 are dropped, and 8, 9, 6, 3 and 5 become spheres in that order

### Requirement: Body corner spheres
After the candidate spheres, every cell up to the last inner one with more than the builder's corner count of filled corners, at least two of them not counted by a sphere, SHALL drop the uncounted corners that a neighbour with more than one more filled corner shares, and give a sphere when more than the builder's free-corner count remain, or, when none remain, only for a cell of 4 to 6 corners whose mask is its own mirror or a face neighbour's.

#### Scenario: Shared corners
- **WHEN** a cell's −x neighbour has its four +x corners filled
- **THEN** those are this cell's four −x corners

### Requirement: Corner sphere placement
A corner sphere SHALL be one longest-axis cell across, at the cell's corner point moved toward its empty side by 0.02 of half a longest-axis cell per unit: −4 per empty corner direction and (8 − corners) times the offset of every neighbour short of eight. A mesh with no candidate SHALL get the corner spheres without the counting.

#### Scenario: A thin slab
- **WHEN** a solid 6 × 6 × 2 cell slab is fitted
- **THEN** each of its 25 cells with eight corners gives a sphere 2 cm across, the middle one at its corner point and the one beside the low x end moved 24 units toward +x

### Requirement: Body weighing
The system SHALL weigh a fitted body with mobility 10 000 over its volume and point masses 4 over the barycentric weights, by the original's triple products, of its centre moved by the builder's origin on its rest tetrahedron.

#### Scenario: Weighted point masses
- **WHEN** the moved centre has weights ½, ¼, ⅛, ⅛ on the rest tetrahedron
- **THEN** the point masses are 8, 16, 32, 32

#### Scenario: Shipped bodies
- **WHEN** the installed game's `Objlib` body nodes are fitted from their meshes (with `EXANIMA_DIR`)
- **THEN** most of them get their stored sphere lists bit for bit, with the fit's version and the voxeliser's wide-query flag they were made with
