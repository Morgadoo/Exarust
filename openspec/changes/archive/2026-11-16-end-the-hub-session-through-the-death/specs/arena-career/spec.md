## ADDED Requirements

### Requirement: The hub's session ends through the player's death
When the hub's session hook finds the manager dead it SHALL, after any wipe, end the session through the player's death: the screens swept, the fall's tip, the controls blocked and the end menu; the auto rematch SHALL start nothing once the session has ended.

#### Scenario: A dead manager back from a lost match
- **WHEN** the arena sends a dead manager back to the hub after a fall that was not fatal and the hub's session plays
- **THEN** the session ends with the alive tip, the blocked controls and the end menu, and no rematch starts

### Requirement: The hub's session hook waits for the hub's load
The hub's session hook SHALL not run while a level loads.

#### Scenario: The hub's load
- **WHEN** the hub is entered while its level loads and the player's body is dead, then the load ends with a living manager
- **THEN** the session is not ended and its phase stays playing
