## ADDED Requirements

### Requirement: Story respawn portal
The story respawn SHALL load `exanima05` with the session-start arrival word, so the level's reconstruction device is the player's object. Once the level is in, the player SHALL stand at that object's frame 15 cm up, which the flow records as the respawn portal; without one, at the identity frame 15 cm up.

#### Scenario: The story respawn in exanima05
- **WHEN** the death screen's respawn loads `exanima05`
- **THEN** the player stands 15 cm over the level's one reconstruction device and that device is the recorded portal

### Requirement: Save close step
The flow's save close step SHALL leave no save file open; the save slots open a file per operation.

#### Scenario: Leaving a session
- **WHEN** a session ends with the save close step
- **THEN** no save package file stays open
