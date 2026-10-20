## ADDED Requirements

### Requirement: Morph vertex apply
For each vertex the system SHALL take the three points its joint names through the remap as the rows of a 3 × 3 basis, multiply the vertex's offset by them, subtract the controller's base, apply the owner's rows, and add the vertex's base. The result SHALL have one point per vertex.

#### Scenario: A basis and a base
- **WHEN** the points are (1, 0, 0), (0, 2, 0) and (0, 0, 3), the offset is (1, 1, 1), the controller's base is (1, 1, 1), the owner's rows are the identity and the vertex's base is (0, 0, 10)
- **THEN** the point is (0, 1, 12)

#### Scenario: Remap and orientation
- **WHEN** the remap is (2, 0, 1) and the offset is (1, 0, 0)
- **THEN** the point is (0, 0, 3)
- **WHEN** the owner's rows are (0, 1, 0), (−1, 0, 0), (0, 0, 1) and the offset is (1, 2, 0)
- **THEN** the point is (−4, 1, 0)
