# arena-records Specification

## Purpose
The arena record log: appending match and career records to the global twenty-byte list.

## Requirements

### Requirement: Arena record append
Appending a value SHALL store the id and the value as the first word. Appending words SHALL store the id and all four words. Both SHALL raise the count by one and return the new slot.

#### Scenario: Value and words
- **WHEN** a value 7 is appended with id 3
- **THEN** the slot is 0 and the record is id 3 with words 7, 0, 0, 0
- **WHEN** words 1, 2, 3 and 4 are appended with id 5
- **THEN** the slot is 1 and the record is id 5 with words 1, 2, 3 and 4

### Requirement: Arena pick
Picking SHALL count the positive entries among three, advance the seed by `seed × 0x08088405 + 1`, take the high half of `seed × count` as the index, and return the value at that index with the new seed.

#### Scenario: One, three and a leading zero
- **WHEN** the values are 5, 0 and 0 and the seed 0
- **THEN** the new seed is 1, the index 0 and the value 5
- **WHEN** the values are 3, 7 and 9 and the seed 0
- **THEN** the index is 0 and the value 3
- **WHEN** the values are 0, 3 and 0 and the seed 0
- **THEN** the index is 0 and the value 0
