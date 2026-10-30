## MODIFIED Requirements

### Requirement: Gamepad stick zoom
Only the stepped camera's stick zoom SHALL move; any other camera SHALL keep its zoom and asked pitch. The stepped camera's stick zoom SHALL subtract `(steps · 0.008333334) · ((max − min) · 0.2)` from the zoom, take the least below it and otherwise the most above it (a NaN kept), and hold the steeper of the new zoom's pitch floor and the asked pitch.

#### Scenario: One notch in
- **WHEN** a zoom of 1280 in an 800–2000 range takes 120 steps with an asked pitch of 0.5
- **THEN** the zoom is 1040 and the held pitch is the floor 0.805

#### Scenario: A free camera
- **WHEN** a camera without the stepped flag takes 120 steps
- **THEN** its zoom and pitch are unchanged

## ADDED Requirements

### Requirement: Camera tick clock
The tick clock and the frame stamp SHALL be unsigned 32-bit milliseconds compared unsigned, the clock snapped to the frame after the ninth tick, and the frame SHALL lie `1 − (clock − frame)/17` into the last tick, the difference widened to a signed 64-bit integer and the division and subtraction done in double before one narrowing.

#### Scenario: A wrapped clock
- **WHEN** the tick clock stands 9 ms below the 32-bit wrap and the frame stamp is 8
- **THEN** no tick runs

### Requirement: Camera controller lifetime
The camera controller's constructor SHALL store a 45° angle, bind the camera node and viewport back to it, add its point-translate hook to the node's rebuild list at priority 0, save the viewport's GUI callback slot and install its destructor there, add its frame tick to the scene's task list at priority 7, apply the preset and set the tick clock to the frame stamp. The destructor SHALL run the saved callback only when the slot was not empty.

#### Scenario: A viewport without a callback
- **WHEN** a controller is built on a viewport with no GUI callback and then destroyed
- **THEN** no saved callback runs

### Requirement: Preset keeps a placed distance
When the controller's keep byte is set, a preset SHALL keep the controller's distance instead of the record's and clear the byte; either distance SHALL then be raised to the least distance.

#### Scenario: A distance below the least
- **WHEN** the keep byte is set, the controller's distance is 5 and the least is 10
- **THEN** the distance is 10 and the byte is clear

### Requirement: Stepped pitch clamp
The stepped camera's placement SHALL hold the look-down amount `−pitch` to at least the held pitch and then at most the cap, a NaN pitch kept.

#### Scenario: Too shallow
- **WHEN** the pitch is −0.5 with a held pitch of 0.8 and a cap of 1.5
- **THEN** the pitch becomes −0.8

### Requirement: Orbit release snap
Releasing the chase camera's orbit button SHALL apply the free-camera drag's release snap to the target yaw against the shown yaw.

#### Scenario: A flick past an eighth of a turn
- **WHEN** the orbit button is released with the target yaw 0.9 rad ahead of a shown yaw of 0
- **THEN** the target yaw becomes π/2
