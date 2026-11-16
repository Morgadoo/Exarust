## ADDED Requirements

### Requirement: Controller input mode value
Apply SHALL write `InputMode` as `3` when the controller mode is on and `0` otherwise.

#### Scenario: Controller on
- **WHEN** the controller mode is on and Apply runs
- **THEN** the settings file's `InputMode` line is `3`

### Requirement: Controls view tabs with a controller
The Controls page SHALL show its Keyboard and Controller tabs at the page switch when a controller is connected, and hide them otherwise.

#### Scenario: A pad is connected
- **WHEN** a gamepad is connected and the Controls page is shown
- **THEN** the Keyboard and Controller tabs are visible
