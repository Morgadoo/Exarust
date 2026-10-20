## ADDED Requirements

### Requirement: Barrier orientation
The orientation pass SHALL swap every point's y and z.

#### Scenario: Two points
- **WHEN** the points are (1, 2, 3) and (4, 5, 6)
- **THEN** they become (1, 3, 2) and (4, 6, 5)

### Requirement: Barrier pose advance
The pose advance SHALL make the current pairs previous, the next pairs current, and store the frame.

#### Scenario: One advance
- **WHEN** the current first is (1, 0, 0), the next first (2, 0, 0) and the frame argument 7
- **THEN** the new first is (2, 0, 0), its previous (1, 0, 0) and the frame 7
