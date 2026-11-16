## ADDED Requirements

### Requirement: Open game screen holds the idle turn
While an in-game screen (any form of the 0xce100000 tag family: the inventory, a loot window, the journal, a container popup) is open, an idle player whose ten seconds have passed SHALL have the idle clock restamped instead of the camera turning, and the cursor read SHALL be left as it was.

#### Scenario: Idle with the journal open
- **WHEN** the player stands idle for thirty seconds with the journal open
- **THEN** the camera does not turn, and it turns ten seconds after the journal closes
