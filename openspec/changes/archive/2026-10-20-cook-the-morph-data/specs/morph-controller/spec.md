## ADDED Requirements

### Requirement: Morph data blob
Cooking the morph data SHALL write the magic 0xEDC0C589, a size of 12·(P+1)·(V+1)+12, the base point, the rest points, and each part's pivot and one delta per vertex.

#### Scenario: One part and two vertices
- **WHEN** one part and two rest points are cooked
- **THEN** the magic is 0xEDC0C589 and the size is 84

### Requirement: Morph delta
A vertex's delta SHALL be the barycentric triangle point minus the part's pivot, rotated by the owner's three rows.

#### Scenario: Identity and a quarter turn
- **WHEN** the barycentric weights are 0.5, 0.25 and 0.25 over the corners (0, 0, 0), (1, 0, 0) and (0, 1, 0), the pivot is the origin and the rotation is the identity
- **THEN** the delta is (0.25, 0.25, 0)
- **WHEN** the rotation turns a quarter about z
- **THEN** the delta is (−0.25, 0.25, 0)
