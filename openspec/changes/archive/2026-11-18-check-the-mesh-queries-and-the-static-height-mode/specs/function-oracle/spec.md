## ADDED Requirements

### Requirement: Mesh query slots checked
The oracle SHALL check the mesh's two-sided, front, swept and swept nearest queries and its cloth sweep query, each in its brute-force form over the mesh's faces and its kd-tree leaf form over a face list, against the per-face tests of the port run over the same face records, comparing every callback's face, closest point and push and the query's return.

#### Scenario: The ten query slots
- **WHEN** the ten slots are replayed over 5000 cases each
- **THEN** every case is identical, a NaN's bits aside

### Requirement: The cloth leaf's callback index
The cloth sweep's kd-tree leaf SHALL be compared with the callback's index taken as the position in the face list, which is what the original hands its callback, while its return is the last face id pushed.

#### Scenario: A leaf push on the third listed face
- **WHEN** the cloth leaf pushes off the face listed third
- **THEN** its callback receives index 2 and the query returns that face's id
