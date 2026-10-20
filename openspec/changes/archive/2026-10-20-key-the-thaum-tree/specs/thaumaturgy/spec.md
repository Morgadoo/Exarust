## ADDED Requirements

### Requirement: Tree keys
A non-zero state SHALL go to the base and return its result. A set hide flag with its deadline less 0x3b6 above the clock SHALL return without work. With flag 0x2 clear the next key SHALL step +1 and the previous key −1. Any other key SHALL dim unless the dim flag is set.

#### Scenario: The base and hide window
- **WHEN** the state is 3
- **THEN** the base runs and its result returns
- **WHEN** the hide flag is set, the deadline is 1000 and the clock is 10
- **THEN** the handler returns without work

#### Scenario: The steps and dim
- **WHEN** the flags lack 0x2 and the key is the next code
- **THEN** the step is +1
- **WHEN** the key is the previous code
- **THEN** the step is −1
- **WHEN** the key is any other and the dim flag is clear
- **THEN** the tree dims
- **WHEN** the dim flag is set
- **THEN** nothing runs
