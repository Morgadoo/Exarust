## ADDED Requirements

### Requirement: Editable form
The system SHALL turn a decoded mesh into an editable form holding, for every channel (positions, texture coordinates, smoothing), one element index per triangle corner, and SHALL list each position element's corners with the last corner of the mesh first. Decoded meshes SHALL keep each vertex's texture-coordinate element and the per-triangle edge bits, for every mesh in `CharExt.rpk`.

#### Scenario: A square
- **WHEN** the square (0, 1, 2), (0, 2, 3) is made editable
- **THEN** the position channel holds those triples and element 0 lists face 1's corner before face 0's

### Requirement: Smoothing groups
The system SHALL group each position element's corners by smoothing bits (the low 16 bits of the triangle flags): from a seed corner, merge the bits of every corner sharing one until nothing grows, gather the corners sharing a bit with the result in their order, and seed the next group with the next remaining corner; a face without bits SHALL be a group of its own.

#### Scenario: Bits merge through a shared bit
- **WHEN** four faces round a point carry bits 1, 4, 3 and 2
- **THEN** the faces with bits 2, 3 and 1 form one group and the face with bit 4 another

### Requirement: Render form
The system SHALL convert an editable mesh to render form by splitting each position element's corners by smoothing group, by material group, by texture-coordinate element in groups whose material has a texture other than a reflection map, and by texture winding in groups whose material has a normal map, numbering one unified vertex per run group by group.

#### Scenario: A texture seam
- **WHEN** the square's second face names other texture elements on the diagonal
- **THEN** a textured group has six vertices and an untextured one four

#### Scenario: Mirrored texture
- **WHEN** the square's second face is mapped clockwise in texture space
- **THEN** a normal-mapped group has six vertices and a merely textured one four

### Requirement: Vertex-cache order
The system SHALL reorder each group's triangles greedily by the sum of their vertices' scores (2·n^−½ for n remaining triangles plus 0.75 for the three newest places of a 32-entry cache and (1 − (k − 3)/29)^1.5 for later places, from the original's fast logarithm and exponential), falling back to the head of the list sorted by starting score, SHALL sort each group's triangles by their flags when the render flags ask for it, and SHALL renumber vertices by first use.

#### Scenario: The lone triangle first
- **WHEN** a group holds triangles (5, 4, 6), (3, 4, 5) and (0, 1, 2)
- **THEN** the order is (0, 1, 2), (5, 4, 6), (3, 4, 5) and the vertices are renumbered 0, 1, 2, 6, 4, 3, 5

#### Scenario: Pinned curves
- **WHEN** a vertex has one remaining triangle
- **THEN** its score is the float with bits 0x3FFFFF7C

### Requirement: Subdivision points
The system SHALL subdivide an editable mesh once by the original's Loop rules: crease edges (one face, or smoothing groups that disagree) and edges between points on three or more creases take their midpoint, other edges 3/8 of each end and 1/8 of each opposite point; points on fewer than two creases keep 1 − β of themselves (β = 3/8, 3/16 with three neighbours) and share β among their neighbours, points on two creases go halfway from the mean of their crease edge points, and others stay.

#### Scenario: The square
- **WHEN** the unit square is subdivided
- **THEN** its diagonal point is (0.5, 0, −0.5) and its corner (0, 0, 0) moves to (0.125, 0, −0.125)

### Requirement: Subdivided texture coordinates
The system SHALL subdivide texture coordinates by the same rules inside each coordinate island, and an edge over which the texture folds SHALL become a seam taking the mean opposite coordinate projected onto it.

#### Scenario: A texture fold
- **WHEN** the second face's third corner is mapped to (0.6, 0), on the same side of the diagonal as the first face's
- **THEN** the diagonal's texture point is (0.475, 0.475)

### Requirement: Subdivided faces
The system SHALL replace every face by its three corner faces then its middle one, group by group, copy the triangle flags to the four, turn the middle face's edge bits by one edge, make colours white, and leave unchanged a mesh whose faces ×4 would exceed 65 535.

#### Scenario: Face order
- **WHEN** the unit square is subdivided
- **THEN** face 0 becomes (0, 5, 4), (5, 1, 6), (4, 6, 2), (4, 5, 6)

#### Scenario: Too many faces
- **WHEN** a mesh has 16 384 faces
- **THEN** it is left unchanged

### Requirement: Effect shells
The system SHALL build the unit icosahedron (poles at ±y, rings of five at latitude ±atan ½, the lower turned by 36°), its five-face top cap, and their subdivision (midpoints pushed to unit length, every face split into its corner faces then its middle one, 4n − 6 points counted).

#### Scenario: The bubble sphere
- **WHEN** the icosahedron is subdivided three times
- **THEN** it has 1 280 faces and 642 points on the unit sphere
