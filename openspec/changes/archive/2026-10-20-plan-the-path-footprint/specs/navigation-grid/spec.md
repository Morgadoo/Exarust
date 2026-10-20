## ADDED Requirements

### Requirement: Footprint rectangles
The planner SHALL merge a footprint into the first rectangle whose padded union is smaller than the two padded areas. Otherwise it SHALL append the footprint, adding eight to the capacity when the length has reached it. A busy planner SHALL leave the list alone. The padded area SHALL be `(x1 − x0 + 6) × (y1 − y0 + 6)`.

#### Scenario: Merge, append and grow
- **WHEN** `(0, 0, 9, 9)` is planned into an empty list, then `(5, 5, 14, 14)`, then `(100, 100, 109, 109)`
- **THEN** the first is appended at 0 and the capacity is 8
- **AND** the second merges into 0, leaving `(0, 0, 14, 14)`
- **AND** the third is appended at 1
- **WHEN** the planner is busy
- **THEN** nothing changes
- **WHEN** the length has reached a capacity of 1
- **THEN** appending raises the capacity to 9
