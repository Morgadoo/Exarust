## ADDED Requirements

### Requirement: Footprint extents
With axes 10 and 10, inverse cell 1, extents 1, 1 and 1, and local values 2, 0, 4 elsewhere zero, the extents SHALL give low bounds 5 and 5 and high bounds 6 and 6. The slot SHALL advance by one modulo 16.

#### Scenario: The extents
- **WHEN** the axes are 10 and 10, the inverse is 1 and the local values are 2 and 4 at 0 and 2
- **THEN** the low bounds are 5 and 5 and the high bounds are 6 and 6
- **WHEN** the slot is 15
- **THEN** it becomes 0
