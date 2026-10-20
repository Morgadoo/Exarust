## ADDED Requirements

### Requirement: Arena pick
Picking SHALL count the positive entries among three, advance the seed by `seed × 0x08088405 + 1`, take the high half of `seed × count` as the index, and return the value at that index with the new seed.

#### Scenario: One, three and a leading zero
- **WHEN** the values are 5, 0 and 0 and the seed 0
- **THEN** the new seed is 1, the index 0 and the value 5
- **WHEN** the values are 3, 7 and 9 and the seed 0
- **THEN** the index is 0 and the value 3
- **WHEN** the values are 0, 3 and 0 and the seed 0
- **THEN** the index is 0 and the value 0
