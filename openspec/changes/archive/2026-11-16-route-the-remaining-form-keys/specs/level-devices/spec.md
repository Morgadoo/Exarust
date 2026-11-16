## MODIFIED Requirements

### Requirement: Destination screen pointer
While a destination screen shows, each frame SHALL hand it the pointer's point in physical pixels, a left (bit 0) or right (bit 1) press of that frame, any key the window procedure sends as the leave key, and the window's physical size with the interface's GUI scale as its unit; the screen SHALL hold the pointer, so the world's press and click paths leave it alone.

#### Scenario: Choosing a row
- **WHEN** a teleporter's screen has revealed its rows and the player presses the left button over a row's node
- **THEN** the journey starts, the bubble grows above the teleporter and the player does not walk to the pressed point

#### Scenario: Any key leaves
- **WHEN** the screen's reveal is within its last 400 ms and W goes down
- **THEN** the screen closes
