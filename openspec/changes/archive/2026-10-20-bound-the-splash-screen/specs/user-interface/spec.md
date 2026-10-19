## ADDED Requirements

### Requirement: Splash bounds
The splash bounds SHALL write the screen to the first output, −1 to the second and return −1.

#### Scenario: The bounds
- **WHEN** the bounds run for a screen
- **THEN** the first output is the screen, the second is −1 and the result is −1

### Requirement: Splash destroy
Destroy SHALL make one release call on a set free flag, clear the splash global, free each stored pointer present, make one list call for a positive count, always run the base destructor, and make a final call only on a set free flag.

#### Scenario: The freeing path
- **WHEN** the free flag is set, both pointers are present and the count is 2
- **THEN** one release, two frees, one list call, one base call and one final call are counted
- **AND** the global is cleared

#### Scenario: The quiet path
- **WHEN** the free flag is clear and both pointers are absent
- **THEN** no release, no free, no list call and no final call are made
- **AND** the base call still runs
