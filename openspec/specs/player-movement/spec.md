# player-movement Specification

## Purpose
Lets the player move a character through original levels with collision, as
the foundation of physically driven bodies and combat.

## Requirements

### Requirement: Level collision
The system SHALL turn the meshes of a level shown in play mode into static colliders, so characters stand on floors and are stopped by walls.

#### Scenario: Standing on the arena floor
- **WHEN** `exarust play arena` starts
- **THEN** the player stands on the floor instead of falling through it

### Requirement: Camera-relative movement
The system SHALL move the player in the direction of the movement keys relative to the camera's heading, at walking speed or at running speed while Shift is held.

#### Scenario: Walking forward
- **WHEN** the forward key is held
- **THEN** the player moves away from the camera and the body turns to face that way

### Requirement: Clips follow movement
The system SHALL play the idle clip when the player is still, the walk clip when walking and the run clip when running.

#### Scenario: Start and stop
- **WHEN** the player starts walking and then stops
- **THEN** the clip changes from idle to walk and back to idle

### Requirement: Chase camera
The system SHALL keep the camera at an adjustable distance and angle above and behind the player.

#### Scenario: Zoom
- **WHEN** the mouse wheel is turned
- **THEN** the camera distance changes within limits

### Requirement: Original starts
The system SHALL start the player where the original does for a new game in the first story level, the arena hub and arena matches, facing as the original does; in an arena duel the opponent SHALL stand opposite, both facing the arena's centre.

#### Scenario: Arena duel
- **WHEN** `exarust --opponent play arenao3` runs
- **THEN** the player stands 2 m from the arena centre along +Z facing it, and the opponent 2 m along −Z facing it
