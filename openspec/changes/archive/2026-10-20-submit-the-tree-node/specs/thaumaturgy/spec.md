## ADDED Requirements

### Requirement: Tree submit
The submit SHALL append one entry only in state 0x21 when the count is positive: the value and the second value over 5.0, then the count. It SHALL always clear the state and index and run the base once.

#### Scenario: The submitted entry
- **WHEN** the state is 0x21, the count is 3, the value is 10 and the second is 7.5
- **THEN** the entry is 2.0, 1.5 and 3
- **AND** the state and index clear and one base call runs
- **WHEN** the state is not 0x21 or the count is 0
- **THEN** no entry is appended and the base still runs
