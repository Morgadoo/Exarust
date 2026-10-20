## ADDED Requirements

### Requirement: Smallest enclosing sphere
The sun shadow fit's sphere SHALL be found in double precision by move-to-front, starting at the first point with radius 0. A point not in the support set and outside the sphere by more than 1e-5 (squared distance minus r²) SHALL give a new sphere from the support set and that point; a larger result SHALL replace the sphere and restart the scan. The centre and radius SHALL be narrowed to single precision.

#### Scenario: A right triangle
- **WHEN** the points are (0, 0, 0), (4, 0, 0) and (0, 3, 0)
- **THEN** the sphere is centred at (2, 1.5, 0) with radius 2.5, resting on the last two points

#### Scenario: A point within the slack
- **WHEN** the points are (0, 0, 0), (2, 0, 0) and a point 8e-6 outside their sphere in squared distance
- **THEN** the sphere stays centred at (1, 0, 0) with radius 1

### Requirement: Sphere candidates
The new sphere SHALL be chosen among the spheres through the point and one, two or three support points, in the original's fixed orders, as the smallest that holds the other support points, else the one that misses by least; the support set SHALL become the winner's points then the new point.

#### Scenario: A regular tetrahedron
- **WHEN** three corners of a regular tetrahedron are the support set and the fourth is added
- **THEN** every pair and triangle misses a corner, the circumsphere wins and the support set holds all four

### Requirement: Empty point set
Without points only the radius SHALL be written, as 4000.

#### Scenario: No points
- **WHEN** the point set is empty
- **THEN** the radius becomes 4000 and the centre is left as it was

### Requirement: Flat triangles and tetrahedra
Three points whose doubled Gram determinant is at most 1e-7 in magnitude, or four whose edge Gram determinant is, SHALL give the squared radius 3.4e38, so that they never win; a flat tetrahedron SHALL be centred at the origin. Three points SHALL fall back to an edge sphere when their circumcentre lies outside the triangle.

#### Scenario: Collinear and coplanar points
- **WHEN** three points lie on a line, or four in a plane
- **THEN** their candidate's squared radius is 3.4e38
