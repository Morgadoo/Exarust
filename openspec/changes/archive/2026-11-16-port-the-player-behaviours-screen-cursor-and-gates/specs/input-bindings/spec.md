## MODIFIED Requirements

### Requirement: Gamepad movement and actions
In controller mode the pad SHALL move the player by the move stick in the camera's frame, drive the behaviour's screen cursor from the aim stick once its squared length is over 0.01, stab on `AltAction`, crouch on `CrouchC`, sprint on `Sprint` and swap weapons on `WepSwp`. Keyboard and mouse input SHALL keep working unchanged.

#### Scenario: No gamepad
- **WHEN** no gamepad is connected
- **THEN** the player is driven by the keyboard and mouse exactly as before

#### Scenario: Sprint
- **WHEN** `Sprint` is held in controller mode
- **THEN** the player sprints

## ADDED Requirements

### Requirement: Gamepad look buttons
The pad's `Look` and `CamMode` buttons SHALL start and hold the controls' mouse capture as Cursor Look's key and the orbit's button do, and while the capture holds the aim stick's pair SHALL be stored unturned.

#### Scenario: Look button
- **WHEN** `Look` is held in controller mode
- **THEN** Cursor Look's capture starts
