## ADDED Requirements

### Requirement: Per-mode words
The game SHALL keep the mode switch's words from every change of the mode: the level cap 15 for practice and the story and 12 for the arena, unchanged for any other mode.

#### Scenario: The arena
- **WHEN** the mode becomes the arena
- **THEN** the skill screen, the HUD alert and the company report compare the levels with 12

### Requirement: Camera detach on no mode
A switch to no mode that the camera saw SHALL detach its controller from the view until a switch into a mode attaches it.

#### Scenario: Play ends with the player still there
- **WHEN** the scene switches to no mode
- **THEN** the camera holds its place while the player moves

### Requirement: Shot into the pin's saved view
A camera shot SHALL copy the camera into the cursor pin's saved view, so the pin does not carry the pointer across the shot.

#### Scenario: A shot during Cursor Look's alignment
- **WHEN** a shot is taken
- **THEN** the next pin step starts from the shot's view

### Requirement: The player's arena start shot
The first round of a match the player fights in SHALL frame the spawn spot with the fast two-point shot from the spot raised by the near offset.

#### Scenario: The player's duel
- **WHEN** the first round of the player's duel is stood
- **THEN** the camera takes the fast shot of the spawn spot
