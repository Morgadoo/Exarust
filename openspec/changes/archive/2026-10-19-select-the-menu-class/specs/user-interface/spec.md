## ADDED Requirements

### Requirement: Class select index
The class-select screen SHALL store the index at field 0x6c and return 3. It SHALL play one selection sound only when the index and the sound flag are both above zero.

#### Scenario: A positive index
- **WHEN** the index is 4 and the sound flag is positive
- **THEN** field 0x6c is 4, one sound plays and the result is 3

#### Scenario: The guard
- **WHEN** the index is 0 or negative
- **THEN** no sound plays and the result is still 3
- **WHEN** the index is positive but the sound flag is zero
- **THEN** no sound plays
