# intersection-tests Specification

## Purpose
The original's shared intersection maths, as it computes it: sphere, box,
oriented-box and swept-sphere tests, and the convex volumes the renderer
clips portals and directional shadows with.

## Requirements

### Requirement: Original arithmetic
The system SHALL compute the shared intersection tests with the original's arithmetic: sums of x and y before z, the processor's minimum and maximum (a NaN selects the second operand), "apart" answers as the sign bit of reach² minus distance², and each branch on the side the original takes for a NaN.

#### Scenario: Touching spheres
- **WHEN** two spheres of radii 3 and 4 have centres 7 apart
- **THEN** they are not apart

#### Scenario: A sweep grazing a box face
- **WHEN** a sphere of radius 1 sweeps from (−10, 3, 0) to (10, 3, 0) past a box of half extents 2 at the origin
- **THEN** it hits the box

#### Scenario: A segment with a NaN projection
- **WHEN** a segment from the origin along (NaN, 0, 0) is tested against a sphere of radius 2 at (1, 0, 0)
- **THEN** the distance is measured from the start and the sphere meets it

### Requirement: Plane refresh
The system SHALL rebuild a face's plane from its polygon with the normal (v2 − v1) × (v0 − v1), normalised, a zero vector left zero, and the offset that puts v0 on the plane.

#### Scenario: A counter-clockwise triangle
- **WHEN** a face has the vertices (0, 0, 1), (1, 0, 1), (1, 1, 1)
- **THEN** its normal is (0, 0, 1) and its offset −1

### Requirement: Portal polygon clipping
The system SHALL clip a polygon by each face of a convex volume in order, walking it from its last vertex's side and keeping the first inner run with its crossings, and SHALL build one face per edge of what remains through the previous point, the apex and the point.

#### Scenario: A polygon dipping inside twice
- **WHEN** the zigzag (0, 1), (1, −1), (2, 1), (3, −1), (4, 1) is clipped by y ≤ 0
- **THEN** it keeps (0.5, 0), (1, −1), (1.5, 0) only

### Requirement: Volume cut
The system SHALL cut a convex volume by a plane, keeping each face's part on the inner side, and SHALL chain the cuts into a cap face on the plane, closing it when its ends' squared distance is below 1e-4 (compared in double) and extending it from its start otherwise, a NaN distance included.

#### Scenario: A cube cut through its centre
- **WHEN** a cube of half side 10 cm is cut by x ≤ 0
- **THEN** it has the −X face, four halved faces and a square cap on x = 0

### Requirement: Directional shadow volume
The system SHALL build the volume a directional light's casters can lie in from a view volume: a side plane through each first shared edge (squared length (x² + y²) + z² not below 1 cm², a NaN length kept) between a face open to the light and one turned away, its polygon extruded 10 000 cm toward the light, followed by the faces turned away; it SHALL return the side planes' last index as their count.

#### Scenario: A cube lit from above
- **WHEN** a cube of half side 10 cm is given the light direction (0, −1, 0)
- **THEN** it gets four side planes, one per side face, the bottom face closes it, and the returned count is 3
