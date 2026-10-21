# mesh-geometry Specification

## Purpose
Derives the geometry a static mesh does not store, as the original's mesh
unit does: normals from the smoothing keys and corner angles, split
tangents, the refitted box and sphere, barycentric coordinates on a face,
the material regroup and the index type of each material group.

## Requirements

### Requirement: Generated mesh normals
A static mesh's normals SHALL be the sum, over each smoothing key, of the unit normals of the faces at its vertices, each weighted by its corner angle, normalised. Every shipped mesh of Objlib.rpk, CharExt.rpk and Resource.rpk SHALL get finite unit normals.

#### Scenario: A shared key
- **WHEN** two faces meet at a vertex through one smoothing key
- **THEN** its normal is the unit sum of their unit normals weighted by the corner angles

### Requirement: Corner angle weight
The corner weight SHALL be √(1 − c)·(1.571 − 0.1945c + 0.111c² − 0.111c³) for the cosine c between the corner's two edges, and 0 when the product of their squared lengths is not above 0x33FFFFD7 or c is not below 1.

#### Scenario: Corner weights
- **WHEN** the corner is a right angle
- **THEN** its weight is 1.571
- **WHEN** the edges are parallel
- **THEN** its weight is 0

### Requirement: Fallback faces
With face flags, faces carrying 0x10000 SHALL count toward a smoothing key's normal only when no face without the flag reaches that key.

#### Scenario: A fallback face
- **WHEN** a flagged face and an unflagged face share a key
- **THEN** only the unflagged face shapes its normal
- **WHEN** only a flagged face reaches a key
- **THEN** the flagged face shapes it

### Requirement: Split tangents
Each vertex's tangent SHALL be the sum of its faces' position gradients along u (the inverse of the UV determinant, or one when its magnitude is below 1e-10 in f64), made perpendicular to the normal and normalised, a zero tangent staying zero; its sign SHALL be 1 when (n × t)·b < 0 for the unprojected sum t and the bitangent sum b, and −1 otherwise.

#### Scenario: A flat quad
- **WHEN** a unit quad with +Z normals and standard UVs is split
- **THEN** every tangent is (1, 0, 0) with sign −1

#### Scenario: A collapsed UV
- **WHEN** all three UVs of a face coincide
- **THEN** the tangent is zero with sign −1

### Requirement: Refitted bounds
A mesh's box SHALL be the component-wise minimum and maximum of its positions, its half extents the maximum less the centre, held to at least 0.001 cm, and its radius the largest distance from the centre, square-rooted only above 0x33FFFFD7 cm².

#### Scenario: A single point
- **WHEN** a mesh has one vertex
- **THEN** its centre is that vertex, its half extents 0.001 cm and its radius 0

### Requirement: Face barycentric coordinates
The barycentric coordinates of a point on a face SHALL be computed after dropping the point onto the face's plane, with the face's stored reciprocal estimate of the Gram determinant, and the first coordinate as one less the other two.

#### Scenario: A lifted point
- **WHEN** a point above a face is measured
- **THEN** its coordinates equal those of its projection

### Requirement: Material regroup and index type
Regrouping SHALL order a mesh's faces by material group, stable within each group, with their flags and edge bits, and rebuild the group ranges, dropping empty groups. A material group's index buffer SHALL use 16-bit indices up to 65 535 vertices and 32-bit indices above.

#### Scenario: Two groups swapped
- **WHEN** a two-face mesh's faces belong to groups 1 and 0
- **THEN** the group-0 face comes first and its flags follow it
