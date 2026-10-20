## ADDED Requirements

### Requirement: Morph point query
The query SHALL start at the base point and add each block point times its weight when the weight's magnitude is at least the epsilon.

#### Scenario: Weighted sum and skip
- **WHEN** the base is 1, 2, 3, one point is 2, 4, 6 and its weight 0.5
- **THEN** the result is 2, 4, 6
- **WHEN** a weight is 0
- **THEN** that point is skipped
