## ADDED Requirements

### Requirement: Arena record append
Appending a value SHALL store the id and the value as the first word. Appending words SHALL store the id and all four words. Both SHALL raise the count by one and return the new slot.

#### Scenario: Value and words
- **WHEN** a value 7 is appended with id 3
- **THEN** the slot is 0 and the record is id 3 with words 7, 0, 0, 0
- **WHEN** words 1, 2, 3 and 4 are appended with id 5
- **THEN** the slot is 1 and the record is id 5 with words 1, 2, 3 and 4
