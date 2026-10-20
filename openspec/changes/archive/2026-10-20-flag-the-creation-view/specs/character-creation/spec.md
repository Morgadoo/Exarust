## ADDED Requirements

### Requirement: Creation view refresh
Refreshing SHALL run the link, camera, first and second ticks.

#### Scenario: The refresh
- **WHEN** the view refreshes
- **THEN** the link, camera, first and second ticks all run

### Requirement: Creation view flag
Setting the flag SHALL set bit `0x10000` of the word when the flag is set and clear it otherwise, then refresh and store the part result.

#### Scenario: Set and clear
- **WHEN** the flag is set and the word is 0
- **THEN** the word is `0x10000`, the refresh runs and the part result is stored
- **WHEN** the flag is clear and the word is `0x10000`
- **THEN** the word is 0
