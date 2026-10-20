## ADDED Requirements

### Requirement: Point-order adoption
Adoption SHALL seed from a target triangle whose three corner keys match a reference triangle's under a rotation (squared XY key distance at most 1e-8), then flood through shared points, matching each unvisited neighbour to an unvisited reference triangle. Unmatched points SHALL keep their identity.

#### Scenario: A rotated mesh
- **WHEN** the target keys are the reference keys cyclically rotated
- **THEN** the map takes each target point to the reference point with the same key
- **WHEN** two triangles share an edge
- **THEN** the flood maps the second triangle consistently

### Requirement: Morph part add
Adding a part SHALL refuse a target whose point count differs from the reference's and otherwise record zero weights, target-minus-reference deltas and no child positions.

#### Scenario: Matching and mismatched counts
- **WHEN** the counts match
- **THEN** the deltas are the target points minus the reference points
- **WHEN** the counts differ
- **THEN** nothing is added
