# mesh-editing Specification

## Purpose
Edits meshes at run time as the original does: the editable form of a
decoded mesh and its conversion back to render form (smoothing groups,
seam and mirror splits, the vertex-cache order), welding and the winding
flip, the Loop subdivision that smooths meshes, the subdivided polyhedra of
the effect shells and the mind effects' UV sphere.

## Requirements

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
The system SHALL subdivide texture coordinates by the same rules inside each coordinate island, and an edge over which the texture folds SHALL become a seam taking the mean opposite coordinate projected onto it. The fold test SHALL be an ordered "greater than zero", so an edge whose two coordinates coincide (a NaN test) SHALL take the plain rule.

#### Scenario: A texture fold
- **WHEN** the second face's third corner is mapped to (0.6, 0), on the same side of the diagonal as the first face's
- **THEN** the diagonal's texture point is (0.475, 0.475)

#### Scenario: A zero-length texture edge
- **WHEN** both ends of the square's diagonal are mapped to (0, 0)
- **THEN** the diagonal's texture point is (0.125, 0.125)

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

### Requirement: Welding
The system SHALL weld every stream that owns its channel and is not fixed: its elements sorted by their first component with a middle-pivot quicksort, each later element merged into the earliest live one whose squared distance is under 0.0001 cm² (positions) or 1e-9 (texture coordinates), the merged corners spliced before the survivor's head, and the survivors renumbered from zero in their old order, with streams sharing the channel following.

#### Scenario: Positions a hundredth of a centimetre apart
- **WHEN** two positions lie 0.009 cm apart
- **THEN** they weld, and two lying 0.011 cm apart do not

#### Scenario: Equal texture coordinates
- **WHEN** three corners carry texture coordinates (0, 0), (0, 0) and (1, 0)
- **THEN** the second element survives and the corners name elements 0, 0 and 1

### Requirement: Welded skin influences
The character weld SHALL fold a welded-away position's skin influences into its survivor (a bone already there keeps the larger weight, the rest fill the free slots up to four) and SHALL divide every element with two or more influences by their sum.

#### Scenario: Influences fold into the survivor
- **WHEN** a position with influence (bone 1, 0.25) absorbs one with (bone 1, 0.5) and (bone 2, 0.2)
- **THEN** it holds bones 1 and 2 weighted 0.5/0.7 and 0.2/0.7

### Requirement: Winding flip
The system SHALL reverse every face by swapping the first and third element of each corner triple on every channel, leaving the stream values in place.

#### Scenario: One triangle
- **WHEN** the triangle (0, 1, 2) is flipped
- **THEN** every channel holds (2, 1, 0)

### Requirement: UV sphere layout
The system SHALL build the mind effects' UV sphere of `segments` columns and `segments / 2` rows as (rows + 1)(segments − 1) + segments vertices in column pairs of triangle strips: a pair's first strip from column c's north pole between columns c and c + 1 to the south pole, its second strip patched to zigzag between columns c + 2 and c + 1, and the last pair's second strip writing the seam column `segments` itself.

#### Scenario: Four segments
- **WHEN** a sphere of 4 segments is built
- **THEN** it has 13 vertices and the strips (0, 1, 2, 3), (4, 7, 2, 5), (6, 7, 8, 9), (10, 11, 8, 12)

### Requirement: UV sphere vertices
The sphere's columns and rows SHALL both step by 2π/segments, and its directions SHALL come from the engine's fast sine (its quadrant rounded to nearest) and the double-precision cosine, with the z terms multiplied in double.

#### Scenario: The fast sine
- **WHEN** the fast sine of 0.5 is taken
- **THEN** its bits are 0x3EF57746

#### Scenario: A double z term
- **WHEN** a sphere of 8 segments and radius 2 is built
- **THEN** vertex 12's z is exactly −1

### Requirement: Editable part assembly
Appending an editable part SHALL join its groups to the destination's group with the same material or a new group, its channels by low key, and its streams by whole format word, skipping the normal stream (low key 2); the position stream SHALL be placed by the rigid `dst⁻¹ · src` in the original's float order, a stream gap filled with 0 (255 for colours), missing face flags filled with 1 and edge bits with 7.

#### Scenario: A turned destination
- **WHEN** a part is merged into a destination 1 m along +X and turned a quarter about Y
- **THEN** the part's (1, 0, 0) lands at (0, 0, 0), its (1, 0, −1) at (1, 0, 0), and no normal stream is merged

### Requirement: Editable material regroup
After an assembly the faces SHALL be sorted into material runs, stable within a group, with every channel, the face flags, the edge bits and the extra per-face word permuted alike.

#### Scenario: Interleaved groups
- **WHEN** faces 0, 1, 4, 5 belong to "a" and 2, 3 to "b"
- **THEN** face 4 becomes the third face and face 2 the fifth

### Requirement: Colour channel creation
Adding a colour channel SHALL append a stream with the first stream's id or the shared-channel bit, the colour format word, the first stream's channel and element count, and opaque white bytes unless bytes are supplied.

#### Scenario: A square
- **WHEN** a colour channel is added to a four-corner square without bytes
- **THEN** the new last stream has the colour id and format, channel 0 and four `[255, 255, 255, 255]` entries
