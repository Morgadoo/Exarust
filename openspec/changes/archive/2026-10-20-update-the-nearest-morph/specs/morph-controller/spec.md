## ADDED Requirements

### Requirement: Morph controller setup
Setup SHALL store the target's count, allocate both arrays and copy the target's points.

#### Scenario: The setup
- **WHEN** the target has 3 points
- **THEN** the count is 3 and both arrays are allocated
- **AND** the three points are copied

### Requirement: Morph nearest update
The nearest update SHALL replace the best point, extra, distance and marker only when the new distance is strictly below the stored one. A NaN distance SHALL not replace.

#### Scenario: Closer, equal and NaN
- **WHEN** the distance is 0.5 and the stored distance 1
- **THEN** the best point and values are replaced
- **WHEN** the distance is 1 and the stored distance 1
- **THEN** nothing changes
- **WHEN** the distance is NaN
- **THEN** nothing changes
