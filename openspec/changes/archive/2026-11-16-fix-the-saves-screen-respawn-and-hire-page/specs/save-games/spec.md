## ADDED Requirements

### Requirement: Saves screen paint after the screens' clear
The saves' character screen SHALL paint its continue form after the game screens' draw, which clears the GUI queue each frame, so that its records reach the queue every frame.

#### Scenario: The saves screen is open
- **WHEN** the continue form is open and a frame runs
- **THEN** the GUI queue holds the form's frame with its records

### Requirement: Respawn rebuild holds its record's hands
The player the respawn rebuilds from its stripped record SHALL NOT be given Exarust's carried torch; it holds what the record's hands hold.

#### Scenario: A story respawn
- **WHEN** the death screen's CONTINUE rebuilds the player at the reconstruction device
- **THEN** the new player holds no torch

### Requirement: Respawn rebuild starts at the portal
The rebuilt player's body SHALL start at the respawn placement rather than the level's own start, and the placement SHALL still lay it there once the load ends.

#### Scenario: The rebuilt body's first frames
- **WHEN** the respawn's level is in and the rebuilt player is spawned
- **THEN** its body stands at the respawn portal, not at the level's start
