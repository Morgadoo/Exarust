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

### Requirement: Morph target switch
Switching SHALL do nothing unless the target's tag is `0xEDC0C589`. Otherwise it SHALL copy the point and points, store the point at the target and refresh it.

#### Scenario: Tagged and untagged
- **WHEN** the target's tag is `0xEDC0C589` and the point is 1, 2, 3
- **THEN** the switch and refresh run and the point is stored
- **WHEN** the tag differs
- **THEN** nothing is copied, stored or refreshed

### Requirement: Morph point blend
Blending SHALL add each source point times the weight to the matching destination point. The result SHALL have the shorter of the two counts.

#### Scenario: Weighted add and mismatch
- **WHEN** the destination is 1, 2, 3, the source 2, 4, 6 and the weight 0.5
- **THEN** the result is 2, 4, 6
- **WHEN** the source has fewer points
- **THEN** only the matching points are produced

### Requirement: Morph part walk
The walk SHALL count a node whose kind masked with `0xFFF` is `0x89` and whose `0xC00000` bits are `0x400000`, then visit every child recursively. The number of eligible nodes SHALL be returned.

#### Scenario: Eligible, ineligible and child
- **WHEN** a node has kind `0x89`, flags with `0xC00000` equal to `0x400000` and a child that is also eligible
- **THEN** the count is 2
- **WHEN** the node's kind is `0x01`
- **THEN** the node is not counted
- **WHEN** the flag band differs
- **THEN** the node is not counted

### Requirement: Morph weight merge
Merging SHALL do nothing when the scale is below the epsilon. Up to four source entries SHALL be read, stopping at the first id below one. An id already present SHALL add `weight × scale`; a new id SHALL append to the highest used slot plus one when that leaves room.

#### Scenario: Add, append and stop
- **WHEN** the current slots are 1 and 2 with weights 0.5 and 0.5, the source is 2 and 3 with weights 1 and 0.5, and the scale 2
- **THEN** slot 2 weighs 2.5 and slot 3 is appended at 1
- **WHEN** the scale is 0
- **THEN** nothing changes
- **WHEN** the first source id is 0
- **THEN** the merge stops

### Requirement: Morph point query
The query SHALL start at the base point and add each block point times its weight when the weight's magnitude is at least the epsilon.

#### Scenario: Weighted sum and skip
- **WHEN** the base is 1, 2, 3, one point is 2, 4, 6 and its weight 0.5
- **THEN** the result is 2, 4, 6
- **WHEN** a weight is 0
- **THEN** that point is skipped
