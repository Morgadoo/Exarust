## ADDED Requirements

### Requirement: Pathfinder metrics
The constructor SHALL store cell size 30, depth 10, inverse 1/30, a zero counter and the seed byte, then run the base once.

#### Scenario: The opening
- **WHEN** the seed byte is 7
- **THEN** the cell is 30, the depth 10 and the inverse 1/30
- **AND** the counter is 0 and the seed is 7
- **AND** one base call runs
