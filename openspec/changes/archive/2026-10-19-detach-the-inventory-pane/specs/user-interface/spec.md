## ADDED Requirements

### Requirement: Inventory detach
An already inactive pane SHALL return true without work. Otherwise the detach SHALL mark its owner, place a visible child, decrement a stack above one or free a stack of one, copy the loot pane's origin, run the base method once and return its result.

#### Scenario: The inactive pane
- **WHEN** the pane is already inactive
- **THEN** the result is true and the base method does not run

#### Scenario: The active pane
- **WHEN** the pane has an owner and a visible child
- **THEN** the owner is marked and the child is placed
- **WHEN** the stack count is 2
- **THEN** it becomes 1 and its top is cleared
- **WHEN** the stack count is 1
- **THEN** it is freed and cleared
- **AND** the base result returns

### Requirement: Inventory page sentinel
The page override SHALL store −3 at field 0x6c when its argument is −1, and SHALL leave the field for any other argument. The base result SHALL return.

#### Scenario: The sentinel
- **WHEN** the argument is −1
- **THEN** field 0x6c becomes −3
- **WHEN** the argument is 4
- **THEN** field 0x6c keeps its value
