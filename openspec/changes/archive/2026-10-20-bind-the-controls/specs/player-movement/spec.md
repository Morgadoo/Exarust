# Spec Delta

## ADDED Requirements

### Requirement: Cursor Look
The system SHALL capture the mouse while the Cursor Look binding is held, hide the system and world cursors, turn the player's facing, heading and combat aim by 3.6 × Δx / window width radians for horizontal mouse motion, and turn the camera's target behind the new facing. Releasing the binding or losing window focus SHALL restore the pointer to where it was.

#### Scenario: Turn with the mouse
- **WHEN** Cursor Look is held and the mouse moves 800 logical pixels right across a 1600-pixel window
- **THEN** the player turns 1.8 radians right and the camera target follows behind it

#### Scenario: Hide the cursors
- **WHEN** Cursor Look is held
- **THEN** neither the system pointer nor the world cursor is drawn

#### Scenario: Release
- **WHEN** Cursor Look is released
- **THEN** the mouse is released and returned to its earlier position

### Requirement: Bound controls
The system SHALL read movement, primary attack, alternate attack, combat toggle, orbit camera and interact from the player's binding table rather than fixed keys.

#### Scenario: Rebound forward
- **WHEN** Forward is rebound from W to another key
- **THEN** W no longer walks the player and the new key does

#### Scenario: Either shift
- **WHEN** either shift key is held with default bindings
- **THEN** the player runs
