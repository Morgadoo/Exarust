# morph-controller Specification

## Purpose
The morph controller: its target arrays and the nearest morph target it finds.

## Requirements

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

### Requirement: Nearest morph index
Finding SHALL scan the points and keep the smallest distance to the given point, returning its index and distance. A NaN distance SHALL not replace. An empty list SHALL return −1 and the maximum float.

#### Scenario: Scan, ties and empty
- **WHEN** the point is 0, 0, 0 and the points are 1, 0, 0; 0, 2, 0 and 0, 0, 0.5
- **THEN** the index is 2 and the distance 0.25
- **WHEN** a distance is NaN
- **THEN** it does not replace the best
- **WHEN** the list is empty
- **THEN** the index is −1 and the distance is the maximum float
