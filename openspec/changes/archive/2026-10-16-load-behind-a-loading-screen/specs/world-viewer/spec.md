## ADDED Requirements

### Requirement: Responsive load
The viewer SHALL build a level over frames, and SHALL run on the async compute pool the work that cannot be cut into short pieces, so the window keeps answering the desktop while a level loads.

#### Scenario: Large level
- **WHEN** `exarust play exanima01` loads
- **THEN** no frame of the load takes a second, and the desktop never reports the window as not responding

### Requirement: Complete first frame
Level and play modes SHALL end their load only when the voxel GI block around the starting view is built and warmed up and, in play mode, when the level's collision is built and every character is dressed. With GI turned off, the load SHALL end without waiting for it.

#### Scenario: First frame after the load
- **WHEN** `exarust play exanima01 --screenshot first.png --frames 1` runs
- **THEN** the screenshot shows the level lit by its GI and the player in braies, not the loading screen

#### Scenario: GI off
- **WHEN** `exarust play arenao1 --no-gi` runs
- **THEN** the load ends once the characters are dressed

### Requirement: Scripted runs after the load
`--frames` (before a screenshot) and `--shove` SHALL count frames from the end of the load, and `--autowalk` SHALL start walking then.

#### Scenario: Screenshot
- **WHEN** `exarust play exanima01 --screenshot shot.png --frames 60` runs
- **THEN** the screenshot shows the level 60 frames after the loading screen went

### Requirement: Window identity
The window SHALL carry the app id (X11 class) `exarust`, so the desktop names it in its dialogs.

#### Scenario: Wayland
- **WHEN** the window opens on a Wayland desktop
- **THEN** its toplevel's app id is `exarust`
