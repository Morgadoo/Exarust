## ADDED Requirements

### Requirement: Skill satellite
The satellite SHALL start from the stored slot's ring point at `(stored − 2.5) × 2π / count + 1 − turn`, with x radius both offsets and y radius only the first, then add the rounded `unit × 18.3 × cos` of `(sample − 1) × 0.2` of a turn to x and subtract the sine from y, each times the turn.

#### Scenario: Two samples
- **WHEN** the count is 8, the offsets are 10 and 5, the turn is 1, the size is 200 by 100, the page is 0, the unit is 4 and the stored code is 3
- **THEN** sample 1 is 358, 53
- **AND** sample 3 is 226, 10
- **WHEN** the page is 1 and the screen is scrolled
- **THEN** sample 1 is 354, 65
