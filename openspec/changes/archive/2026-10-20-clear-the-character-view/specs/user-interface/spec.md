## ADDED Requirements

### Requirement: Character view destroy
The destructor SHALL make one release call on a set free flag, always run the base destructor once and make one final call only on a set free flag.

#### Scenario: The freeing path
- **WHEN** the free flag is set
- **THEN** one release call, one base call and one final call are counted
- **WHEN** the free flag is clear
- **THEN** only the base call runs

### Requirement: Character view clear
When a view exists the clear SHALL release it once, write zero to the owner's 0x130 word and clear the pointer. The base method SHALL always run and its result SHALL return.

#### Scenario: An existing view
- **WHEN** a view exists
- **THEN** one release call runs, the owner slot is cleared and the pointer is cleared
- **WHEN** no view exists
- **THEN** no release call runs
- **AND** the base result returns in both cases
