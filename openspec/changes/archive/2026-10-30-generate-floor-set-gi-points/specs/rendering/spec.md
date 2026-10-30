# Spec Delta

## ADDED Requirements

### Requirement: Floor-set generated GI points
The system SHALL mark a floor-set node for GI point generation when it is reached from the children of the centre-floor variants (`+FM…`), a direct child named `#…`, `+…` or `[…]` being skipped with its subtree and deeper nodes not name-tested, and is of class `0x89` with bit `0x10000` or bit `0x8` of its second flag word; a marked instance SHALL mark its master instead.

#### Scenario: Nested plus name
- **WHEN** a flagged class-`0x89` node named `+deep` sits two levels under a centre-floor variant
- **THEN** it is generated, while a flagged direct child named `+FBA00` and its subtree are not

#### Scenario: Shipped chute
- **WHEN** the shipped floor set `gefs_uwb02` loads
- **THEN** its flagged chute piece carries generated points and exanima01's walk from the start reaches 46 225 cells

### Requirement: Floor-set point generation run
The system SHALL run the surface generator on a marked node's triangles in centimetres at parameter 20 with the greedy cover, its points SHALL replace any stored chunk and be shared by the node's instances, and every other mesh SHALL keep its stored points.

#### Scenario: Shipped box
- **WHEN** the shipped floor set `gefs_uwa01` loads
- **THEN** its flagged box piece carries 164 generated points

### Requirement: Generated-point sphere query
The generator's sphere query over a node's triangles SHALL give the same answer as the plain closest-point test, inclusive at the radius, however the triangles are grouped for speed.

#### Scenario: Deep tree
- **WHEN** a 513-triangle mesh is queried over a lattice of points at a 5 cm radius
- **THEN** every answer equals the plain test's
