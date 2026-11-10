## ADDED Requirements

### Requirement: Device click
A single click on a placed teleporter or reconstruction device SHALL run the device's own use for the player's body, whoever clicked, and read its answer at once: yes when the destination screen opens or is already open, or the scan starts or already runs on the player; no when the use is refused. Without a player it SHALL answer no and ask nothing.

#### Scenario: Clicking the reconstruction device
- **WHEN** the player clicks a reconstruction device with a Power Crystal L on its slot, standing within reach
- **THEN** the use answers yes, runs for the player and the scan opens on the player 1600 ms later

#### Scenario: A refused use
- **WHEN** the player possesses another body, stands out of reach or the slot holds the wrong crystal
- **THEN** the use answers no

### Requirement: Destination screen pointer
While a destination screen shows, each frame SHALL hand it the pointer's point in physical pixels, a left (bit 0) or right (bit 1) press of that frame, Escape as the leave key, and the window's physical size with the interface's GUI scale as its unit; the screen SHALL hold the pointer, so the world's press and click paths leave it alone.

#### Scenario: Choosing a row
- **WHEN** a teleporter's screen has revealed its rows and the player presses the left button over a row's node
- **THEN** the journey starts, the bubble grows above the teleporter and the player does not walk to the pressed point
