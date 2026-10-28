## ADDED Requirements

### Requirement: Normal sum order
Each smoothing key's normal SHALL be accumulated over its position element's corners in reverse incidence order (last face first, each face's corners last first), after the element's list is split into key runs by the original's in-place swap partition, so a second key's corners can come out reordered.

#### Scenario: Alternating keys
- **WHEN** an element's corners, last face first, carry keys A B A B A from faces 4, 3, 2, 1, 0
- **THEN** the walk order is faces 4, 2, 0 for A and then 1, 3 for B

### Requirement: Cloth face normals
The cloth pass's normals SHALL be, per smoothing key, the unweighted sum of its faces' unit normals in the same walk order, normalised.

#### Scenario: Two planes
- **WHEN** two faces on different planes belong to different keys
- **THEN** each key keeps its own face's normal

### Requirement: Face record queries
A face record SHALL hold the unit normal, the plane offset, the corners, the first edge's squared length and the reciprocal squared area, and its queries SHALL reject out-of-range values with compares a NaN passes, only the final distance checks (`0 < t < best` or `t <= best`, the sweep's perpendicular gap, the reach's strict under) rejecting a NaN.

#### Scenario: A ray from above
- **WHEN** a ray runs down onto a face with an upward normal from 5 units above
- **THEN** it hits at distance 5, and the same ray from below misses

#### Scenario: A NaN centre
- **WHEN** a sphere's centre has a NaN coordinate
- **THEN** the push-out test passes its rejections and returns a NaN push
