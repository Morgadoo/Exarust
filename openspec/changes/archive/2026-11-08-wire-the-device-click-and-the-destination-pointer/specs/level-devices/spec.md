## ADDED Requirements

### Requirement: Device click
A single click on a placed teleporter or reconstruction device SHALL run the device's own use for the player's body, whoever clicked; without a player it SHALL ask nothing. The click SHALL read the use's answer as no.

#### Scenario: Clicking the reconstruction device
- **WHEN** the player clicks a reconstruction device with a Power Crystal L on its slot, standing within reach
- **THEN** the device's use runs for the player and the scan opens on the player 1600 ms later

### Requirement: Destination screen pointer
While a destination screen shows, each frame SHALL hand it the pointer's point in physical pixels, a left (bit 0) or right (bit 1) press of that frame, Escape as the leave key, and the window's physical size with the interface's GUI scale as its unit.

#### Scenario: Choosing a row
- **WHEN** a teleporter's screen has revealed its rows and the player presses the left button over a row's node
- **THEN** the journey starts and the bubble grows above the teleporter
